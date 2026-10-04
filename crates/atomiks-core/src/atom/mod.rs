//! A value storable in an atomic, and which read-modify-writes mean something on it.

mod option;
mod ptr;
mod scalar;

use crate::primitive::{AddSub, Bitwise, CompareExchange, Primitive};
use crate::range::ReprRange;
use crate::validity::{Partial, Total, Validity};

/// A value that packs into one atomic word: stored as its [`Repr`](Atom::Repr), and decoded on
/// every load without a check, because an atomic only ever holds reprs that decode.
///
/// A repr decodes when [`from_repr`](Atom::from_repr) returns `Some` for it. So that a
/// compare-exchange loop converges, a repr that decodes should re-encode to itself: every value has
/// one repr.
///
/// Prefer `#[derive(Atom)]`, with atomiks' `derive` feature, for a struct or an enum of atoms: it
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
///   and `Sync`.
///
/// # Examples
/// ```
/// #![feature(const_trait_impl)]
/// # extern crate atomiks_core as atomiks;
///
/// use atomiks::ordering::{Acquire, Release};
/// use atomiks::{Atom, Atomic, ReprRange};
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
    note = "for a type of your own, derive it, `#[derive(Atom)]`, with atomiks' `derive` feature, or keep each promise of `Atom`'s `# Safety` section in a `const unsafe impl`"
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
    /// The repr this value is.
    fn to_repr(self) -> Self::Repr;
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
    label = "`add`, `sub`, `fetch_add` and `fetch_sub` need `AtomAdd`",
    note = "for your own newtype over a value that has it, derive it: `#[derive(AtomAdd)]`",
    note = "to change the value in a compare-exchange loop, call `update`"
)]
pub trait AtomAdd: Atom<Validity = Total, Repr: AddSub> {}

/// The repr's own (signed or unsigned) order is the value's [`Ord`].
///
/// A newtype over a value that has it derives it, `#[derive(AtomOrd)]`.
///
/// A wrong impl gives wrong values, never undefined behaviour: the larger or smaller of two reprs
/// is one of them, so it decodes.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic maximum or minimum",
    label = "`max`, `min`, `fetch_max` and `fetch_min` need `AtomOrd`",
    note = "for your own newtype over a value that has it, derive it: `#[derive(AtomOrd)]`",
    note = "to change the value in a compare-exchange loop, call `update`"
)]
pub trait AtomOrd: Atom + Ord {}

/// And, or, xor and not on the repr, an integer of at most 64 bits or `bool`, combine values.
///
/// A newtype over a value that has it derives it, `#[derive(AtomBitwise)]`.
///
/// A wrong impl gives wrong values, never undefined behaviour: every repr decodes ([`Total`]).
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic bitwise operations",
    label = "`and`, `or`, `xor`, `not` and their `fetch_` forms need `AtomBitwise`",
    note = "for your own newtype over a value that has it, derive it: `#[derive(AtomBitwise)]`",
    note = "to change the value in a compare-exchange loop, call `update`"
)]
pub trait AtomBitwise: Atom<Validity = Total, Repr: Bitwise> {}
