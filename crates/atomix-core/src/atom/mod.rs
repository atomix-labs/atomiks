//! A value storable in an atomic, and which read-modify-writes mean something on it.

mod option;
#[cfg(wide)]
mod pair;
mod ptr;
mod ranged;
mod scalar;

pub use self::ptr::PtrAtom;
#[cfg(wide)]
pub use self::ptr::VtablePointer;
// For deranged's ranged integers, which take the impls atomix's own do.
#[cfg(feature = "deranged-05")]
pub(crate) use self::ranged::ranged_atom;
use crate::primitive::{Bitwise, CompareExchange, ExactBits, FetchAdd, Primitive};
use crate::range::{PointeeAlignment, ReprRange, Tags};
use crate::validity::{Partial, Total, Validity};

/// A value one atomic holds: stored as its [`Repr`](Atom::Repr), and decoded on every load
/// without a check, because an atomic only ever holds reprs that decode.
///
/// A repr decodes when [`from_repr`](Atom::from_repr) returns `Some` for it. So that a
/// compare-exchange loop converges, a repr that decodes should re-encode to itself: every value has
/// one repr.
///
/// Prefer `#[derive(Atom)]`, with atomix's `derive` feature, for a struct or an enum of atoms: it
/// keeps each promise below, and a type without parameters needs no feature gate.
///
/// # Safety
/// For every value `v`:
///
/// - `to_repr(v)`'s unsigned bits lie in [`REPRS`](Atom::REPRS), the range wrapping at the repr's
///   width;
/// - `to_repr(v)` decodes, as `v`;
/// - `from_repr` is a function of its repr alone: whether, and as what, a repr decodes never
///   changes;
/// - `from_repr_unchecked(r)` returns the value `from_repr(r)` does, for every `r` that decodes;
/// - [`Validity`](Atom::Validity) is truthful: every repr it promises decodes, and for
///   [`ZeroNiche`] and [`TotalZeroNiche`], zero does not;
/// - `v` may move to another thread, whatever `Self`'s auto traits say: `Atomic<Self>` is `Send`
///   and `Sync`;
/// - an impl that writes the hidden `to_tagged_repr`, as the derive does for a tagged pointer,
///   keeps its doc's promise: an impl that writes none keeps it already.
///
/// # Examples
/// ```
/// #![feature(const_trait_impl)]
/// # extern crate atomix_core as atomix;
///
/// use atomix::ordering::{Acquire, Release};
/// use atomix::{Atom, Atomic, ReprRange};
///
/// /// The side of the book an order rests on.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// enum Side {
///     Bid,
///     Ask,
/// }
///
/// // SAFETY: `to_repr` gives 0 or 1, within `REPRS`, and `from_repr` decodes each as the side it
/// // came from, by the repr alone; the default `from_repr_unchecked` unwraps `from_repr`;
/// // `Partial`, the default validity, promises no other repr; and a `Side` holds no data, so it
/// // may cross threads.
/// const unsafe impl Atom for Side {
///     type Repr = u8;
///     const REPRS: ReprRange<u8> = ReprRange::new(0, 1);
///     fn to_repr(self) -> u8 {
///         match self {
///             Self::Bid => 0,
///             Self::Ask => 1,
///         }
///     }
///     fn from_repr(repr: u8) -> Option<Self> {
///         match repr {
///             0 => Some(Self::Bid),
///             1 => Some(Self::Ask),
///             _ => None,
///         }
///     }
/// }
///
/// // The side of the last fill, or `None` before the first: `None` takes repr 2, past `REPRS`.
/// static LAST_FILL: Atomic<Option<Side>> = Atomic::new(None);
///
/// LAST_FILL.store(Some(Side::Ask), Release);
/// assert_eq!(LAST_FILL.load(Acquire), Some(Side::Ask), "the side of the fill stored");
/// ```
///
/// [`ZeroNiche`]: crate::validity::ZeroNiche
/// [`TotalZeroNiche`]: crate::validity::TotalZeroNiche
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be stored in an atomic",
    label = "not `Atom`",
    note = "for a type of your own, derive it, `#[derive(Atom)]`, with atomix's `derive` feature, or keep each promise of `Atom`'s `# Safety` section in a `const unsafe impl`"
)]
#[cfg_attr(
    x86_64_without_cmpxchg16b,
    diagnostic::on_unimplemented(
        note = "a 128-bit value needs `cmpxchg16b` on x86_64: build with `-C target-cpu=x86-64-v2` or newer"
    )
)]
#[expect(unsafe_code, reason = "loads decode without a check, trusting the impl")]
pub const unsafe trait Atom: Copy {
    /// The primitive the value is stored as.
    type Repr: const Primitive + CompareExchange;
    /// Which reprs decode.
    type Validity: const Validity = Partial;
    /// A range of the repr's unsigned bits that holds every repr `to_repr` returns, and may wrap
    /// through zero: an `i8` from -1 to 1 runs from `0xFF` through zero to `0x01`.
    ///
    /// `Option`'s `None` takes a repr outside it, or zero where the validity says zero does not
    /// decode.
    const REPRS: ReprRange<Self::Repr>;
    /// How many low bits of a value stored as a pointer hold tags, its own or those of a value it
    /// holds: a word that holds it keeps its own above them. None for a value stored as bits, and
    /// for a hand impl, which no word then holds beside a tag.
    ///
    /// A type's layout may read it, so it reads no pointee's alignment, which
    /// [`POINTEE_ALIGNMENT`](Self::POINTEE_ALIGNMENT) holds apart.
    #[doc(hidden)]
    const TAG_WIDTH: u32 = 0;
    /// The alignment of what a value stored as a pointer points to, which leaves the low bits
    /// clear that its tags, and those of a word that holds it, go in: the least of its pointers'
    /// where one word holds several, as a pointer enum's does, and the first pointer's of two
    /// words, whose low bits alone hold tags. None for a value stored as bits, and for a hand impl.
    ///
    /// Only code and the checks read it, never a type's layout, which may hold the pointee.
    #[doc(hidden)]
    const POINTEE_ALIGNMENT: PointeeAlignment = PointeeAlignment::NONE;
    /// The repr this value is.
    fn to_repr(self) -> Self::Repr;
    /// The repr this value is, `tags` set in its low bits as [`Tags::set_in`] sets them, beside
    /// the bits under the tags it had set already: none, unless its pointer is misaligned for
    /// them, and the repr is then no value's, for the caller to refuse.
    ///
    /// A pointer word passes its own tags on to its pointer field, so a word nested in others
    /// tests its pointer once, against every tag each word sets; an exchange encodes two values
    /// and refuses either misaligned in one test. With no tags, and no bit found set, the repr is
    /// `to_repr`'s.
    #[doc(hidden)]
    #[inline]
    fn to_tagged_repr(self, tags: Tags) -> (Self::Repr, usize) {
        tags.set_in(self.to_repr())
    }
    /// The value `repr` encodes, or `None` for a repr no value encodes.
    fn from_repr(repr: Self::Repr) -> Option<Self>;
    /// The value `repr` encodes, without `from_repr`'s check.
    ///
    /// # Safety
    /// `repr` decodes: `from_repr(repr)` is `Some`.
    #[inline]
    unsafe fn from_repr_unchecked(repr: Self::Repr) -> Self {
        // SAFETY: the caller's repr decodes, so `from_repr` returns `Some`.
        unsafe { Self::from_repr(repr).unwrap_unchecked() }
    }
}

/// Wrapping add and subtract on the repr, an integer of at most 64 bits, are add and subtract on
/// the value.
///
/// A newtype over a value that has it derives it, `#[derive(AtomAdd)]`.
///
/// A wrong impl gives wrong values, never undefined behaviour: every repr decodes ([`Total`]).
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic add",
    label = "`fetch_add` and `fetch_sub` need `AtomAdd`",
    note = "for your own newtype over a value that has it, derive it: `#[derive(AtomAdd)]`",
    note = "to change the value in a compare-exchange loop, call `update`"
)]
pub trait AtomAdd: Atom<Validity = Total, Repr: FetchAdd> {}

/// The repr's own (signed or unsigned) order is the value's [`Ord`].
///
/// A newtype over a value that has it derives it, `#[derive(AtomOrd)]`.
///
/// A wrong impl gives wrong values, never undefined behaviour: the larger or smaller of two reprs
/// is one of them, so it decodes.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic maximum or minimum",
    label = "`fetch_max` and `fetch_min` need `AtomOrd`",
    note = "for your own newtype over a value that has it, derive it: `#[derive(AtomOrd)]`",
    note = "to change the value in a compare-exchange loop, call `update`"
)]
pub trait AtomOrd: Atom + Ord {}

/// And, or, xor and not on the repr, an integer of at most 64 bits or `bool`, combine values, and
/// any one bit of the repr may be set, cleared or inverted.
///
/// A newtype over a value that has it derives it, `#[derive(AtomBitwise)]`.
///
/// A wrong impl gives wrong values, never undefined behaviour: every repr decodes ([`Total`]).
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic bitwise operations",
    label = "`and`, `or`, `xor`, `not`, their `fetch_` forms, and `bit_set`, `bit_clear` and `bit_toggle` need `AtomBitwise`",
    note = "for your own newtype over a value that has it, derive it: `#[derive(AtomBitwise)]`",
    note = "to change the value in a compare-exchange loop, call `update`"
)]
pub trait AtomBitwise: Atom<Validity = Total, Repr: Bitwise> {}

/// And, or, xor and not on the bits a packed value stores it in combine values, and every pattern
/// of those bits decodes.
///
/// So a field of it has the bitwise operations. A packed value stores a field in as few bits as
/// its [`REPRS`](Atom::REPRS) need, read back zero-extended, or sign-extended where two's
/// complement takes fewer. Every value with [`AtomBitwise`] has it, since each of its reprs
/// decodes. So does each of arbitrary-int's integers, with the `arbitrary-int` feature, though none
/// has `AtomBitwise`: a field stores a `u4` in four bits, and each pattern of them is a `u4`.
///
/// # Safety
/// Every pattern of those bits, read back so, is a repr that decodes.
///
/// # Examples
/// [atomix's example][facade] ors a flag into a quote's field of flags, a `u8`, whose
/// [`AtomBitwise`] gives it this.
///
/// [facade]: https://docs.rs/atomix-rs/latest/atomix/#changing-one-field
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no bitwise operations in a field",
    label = "`and`, `or`, `xor`, `not` and their `fetch_` forms on a field need `FieldBitwise`",
    note = "and, or and xor on the field's bits could leave a pattern that is no `{Self}`",
    note = "to change the field in a compare-exchange loop, call `update`"
)]
#[expect(unsafe_code, reason = "a field's bitwise operations write any pattern of its bits")]
pub unsafe trait FieldBitwise: Atom<Repr: ExactBits> {}

// `do_not_recommend`, so a value without `AtomBitwise` reports this trait's message, which speaks
// of fields, not `AtomBitwise`'s.
//
// SAFETY: every repr of an `AtomBitwise` value decodes (`Total`), so every pattern of its bits,
// read back however, does.
#[diagnostic::do_not_recommend]
#[expect(unsafe_code, reason = "`Total` keeps the promise")]
unsafe impl<T: AtomBitwise<Repr: ExactBits>> FieldBitwise for T {}

/// Wrapping add and subtract on the bits a packed value stores it in are add and subtract on the
/// value, and every pattern of those bits decodes.
///
/// So a field of it that ends at its container's top bit adds in place. A packed value stores a
/// field as [`FieldBitwise`] says. Every value with [`AtomAdd`] has it, since each of its reprs
/// decodes, and so does each of arbitrary-int's integers, as for `FieldBitwise`: a `u61` above
/// three `bool`s fills a `u64`, and counts in place.
///
/// # Safety
/// Every pattern of those bits, read back so, is a repr that decodes.
///
/// # Examples
/// [`#[derive(Atom)]`'s example][derive] counts a slot's readers in a field at the top of its
/// word, a `u32`, whose [`AtomAdd`] gives it this.
///
/// [derive]: https://docs.rs/atomix-rs/latest/atomix/derive.Atom.html#changing-one-field
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic add in a field",
    label = "`fetch_add` and `fetch_sub` on a field need `FieldAdd`",
    note = "an add on the field's bits could leave a pattern that is no `{Self}`",
    note = "to change the field in a compare-exchange loop, call `update`"
)]
#[expect(unsafe_code, reason = "a field's add writes any pattern of its bits")]
pub unsafe trait FieldAdd: Atom<Repr: ExactBits> {}

// `do_not_recommend`, as `FieldBitwise`'s.
//
// SAFETY: every repr of an `AtomAdd` value decodes (`Total`), so every pattern of its bits, read
// back however, does.
#[diagnostic::do_not_recommend]
#[expect(unsafe_code, reason = "`Total` keeps the promise")]
unsafe impl<T: AtomAdd<Repr: ExactBits>> FieldAdd for T {}
