//! A value storable in an atomic, and which read-modify-writes mean something on it.

mod scalar;

use crate::primitive::{Bitwise, CellOps, FetchAdd, Primitive};
use crate::validity::{Partial, Total, Validity};

/// A value that packs into one atomic word: stored as its [`Repr`](Atom::Repr), and decoded on
/// every load without a check, because an atomic only ever holds reprs that decode.
///
/// A repr decodes when [`from_repr`](Atom::from_repr) returns `Some` for it. So that a
/// compare-exchange loop converges, a repr that decodes should re-encode to itself: every value has
/// one repr.
///
/// # Safety
/// For every value `v`:
///
/// - `to_repr(v)`'s unsigned bits lie within `MIN_REPR..=MAX_REPR`;
/// - `to_repr(v)` decodes, as `v`;
/// - `from_repr` is a function of its repr alone: whether, and as what, a repr decodes never
///   changes;
/// - `from_repr_unchecked(r)` returns the value `from_repr(r)` does, for every `r` that decodes;
/// - [`Validity`](Atom::Validity) is truthful: every repr it promises decodes, and for
///   [`ZeroNiche`](crate::validity::ZeroNiche) and [`TotalButZero`](crate::validity::TotalButZero),
///   zero does not;
/// - `v` may move to another thread, whatever `Self`'s auto traits say: `Atomic<Self>` is `Send`
///   and `Sync`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be stored in an atomic",
    label = "not `Atom`",
    note = "for a type of your own, derive it: `#[derive(Atom)]`"
)]
#[cfg_attr(
    all(target_arch = "x86_64", not(target_feature = "cmpxchg16b")),
    diagnostic::on_unimplemented(
        note = "a 128-bit value needs `cmpxchg16b` on x86_64: build with `-C target-cpu=x86-64-v2` or newer"
    )
)]
#[expect(unsafe_code, reason = "loads decode without a check, trusting the impl")]
pub const unsafe trait Atom: Copy {
    /// The primitive the value is stored as.
    type Repr: const Primitive + CellOps;
    /// Which reprs decode.
    type Validity: const Validity = Partial;
    /// No repr `to_repr` returns lies below it, as unsigned bits.
    const MIN_REPR: u128;
    /// No repr `to_repr` returns lies above it, as unsigned bits; its bit length is the value's
    /// width as a field of a packed type.
    const MAX_REPR: u128;
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
/// A wrong impl gives wrong values, never undefined behaviour: every repr decodes ([`Total`]).
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic add",
    label = "`add`, `sub`, `fetch_add` and `fetch_sub` need `AtomAdd`",
    note = "for a newtype, derive it: `#[derive(AtomAdd)]`",
    note = "to change the value in a compare-exchange loop, call `update`"
)]
pub trait AtomAdd: Atom<Validity = Total, Repr: FetchAdd> {}

/// The repr's own (signed or unsigned) order is the value's [`Ord`].
///
/// A wrong impl gives wrong values, never undefined behaviour: the larger or smaller of two reprs
/// is one of them, so it decodes.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic maximum or minimum",
    label = "`max`, `min`, `fetch_max` and `fetch_min` need `AtomOrd`",
    note = "for a newtype, derive it: `#[derive(AtomOrd)]`",
    note = "to change the value in a compare-exchange loop, call `update`"
)]
pub trait AtomOrd: Atom + Ord {}

/// And, or, xor and not on the repr, an integer of at most 64 bits or `bool`, combine values.
///
/// A wrong impl gives wrong values, never undefined behaviour: every repr decodes ([`Total`]).
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic bitwise operations",
    label = "`and`, `or`, `xor`, `not` and their `fetch_` forms need `AtomBitwise`",
    note = "for a newtype, derive it: `#[derive(AtomBitwise)]`",
    note = "to change the value in a compare-exchange loop, call `update`"
)]
pub trait AtomBitwise: Atom<Validity = Total, Repr: Bitwise> {}
