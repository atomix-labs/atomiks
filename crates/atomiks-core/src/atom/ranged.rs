//! `Atom` for the ranged integers, each stored as its integer.

use super::{Atom, AtomOrd};
use crate::range::ReprRange;
use crate::ranged::{
    RangedI8, RangedI16, RangedI32, RangedI64, RangedIsize, RangedU8, RangedU16, RangedU32,
    RangedU64, RangedUsize,
};
#[cfg(wide)]
use crate::ranged::{RangedI128, RangedU128};
use crate::validity::Partial;

/// Implements `Atom` and `AtomOrd` for each ranged integer, its range built by `$constructor`, the
/// `ReprRange` constructor in the integer's own order: `new` unsigned, `from_signed` signed.
///
/// Neither `AtomAdd` nor `AtomBitwise`: a sum or an or of two values in the range can leave it.
macro_rules! ranged {
    ($($constructor:ident: $($name:ident($int:ident)),+);+ $(;)?) => {$($(
        // SAFETY: the repr is the integer, from `MIN` to `MAX`, so within `REPRS`, which holds the
        // bits of each integer from `MIN` to `MAX` in its own order, signed or unsigned; `new`
        // decodes exactly those, each as itself, by the repr alone, and `new_unchecked` builds the
        // same value from each; `Partial` promises nothing; an integer may cross threads.
        #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
        const unsafe impl<const MIN: $int, const MAX: $int> Atom for $name<MIN, MAX> {
            type Repr = $int;
            type Validity = Partial;
            // A widening cast never fails; `From` has no `usize` to `u128`, nor `isize` to `i128`.
            const REPRS: ReprRange<$int> =
                ReprRange::$constructor(MIN.strict_cast(), MAX.strict_cast());
            #[inline]
            fn to_repr(self) -> $int {
                self.get()
            }
            #[inline]
            fn from_repr(repr: $int) -> Option<Self> {
                Self::new(repr)
            }
            #[inline]
            unsafe fn from_repr_unchecked(repr: $int) -> Self {
                // SAFETY: the caller's repr decodes, so it lies from `MIN` to `MAX`.
                unsafe { Self::new_unchecked(repr) }
            }
        }
        impl<const MIN: $int, const MAX: $int> AtomOrd for $name<MIN, MAX> {}
    )+)+};
}

ranged! {
    new: RangedU8(u8), RangedU16(u16), RangedU32(u32), RangedU64(u64), RangedUsize(usize);
    from_signed: RangedI8(i8), RangedI16(i16), RangedI32(i32), RangedI64(i64), RangedIsize(isize);
}

// The 128-bit ones, where a 16-byte compare-exchange exists, as `u128` and `i128`.
#[cfg(wide)]
ranged! {
    new: RangedU128(u128);
    from_signed: RangedI128(i128);
}
