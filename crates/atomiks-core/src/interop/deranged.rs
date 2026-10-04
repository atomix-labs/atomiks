//! `Atom` for deranged 0.5's ranged integers, `RangedU8` to `RangedIsize`, and their
//! `OptionRanged` forms, each stored as its integer; and `From` both ways between each ranged
//! integer and atomiks' own of the same bounds.

use core::any::type_name;

use deranged::{
    OptionRangedI8, OptionRangedI16, OptionRangedI32, OptionRangedI64, OptionRangedIsize,
    OptionRangedU8, OptionRangedU16, OptionRangedU32, OptionRangedU64, OptionRangedUsize, RangedI8,
    RangedI16, RangedI32, RangedI64, RangedI128, RangedIsize, RangedU8, RangedU16, RangedU32,
    RangedU64, RangedU128, RangedUsize,
};
#[cfg(wide)]
use deranged::{OptionRangedI128, OptionRangedU128};

use crate::atom::{Atom, AtomOrd, ranged_atom};
use crate::message::{Message, refuse};
use crate::primitive::ExactBits;
use crate::range::ReprRange;
use crate::ranged::{self, each_ranged_integer};
use crate::validity::{Partial, ZeroValid};

// Each of deranged's ranged integers takes the impls atomiks' own does, from the same macro: its
// names, imported here, are deranged's.
ranged_atom! {
    new: RangedU8(u8), RangedU16(u16), RangedU32(u32), RangedU64(u64), RangedUsize(usize);
    from_signed: RangedI8(i8), RangedI16(i16), RangedI32(i32), RangedI64(i64), RangedIsize(isize);
}

// The 128-bit ones, where a 16-byte compare-exchange exists, as `u128` and `i128`.
#[cfg(wide)]
ranged_atom! {
    new: RangedU128(u128);
    from_signed: RangedI128(i128);
}

/// Implements `From` both ways between deranged's ranged integer `$name`, over `$int`, and atomiks'
/// own of the same name and bounds.
///
/// Each saturates, which never moves the integer: the bounds are the same. The optimizer sees as
/// much, since each `get` tells it the range, so each compiles to a move at most, as
/// `tests/codegen.rs` pins.
macro_rules! convert {
    ($name:ident($int:ident)) => {
        /// Keeps the integer, which lies in deranged's range, since the bounds are the same.
        impl<const MIN: $int, const MAX: $int> From<ranged::$name<MIN, MAX>> for $name<MIN, MAX> {
            #[inline]
            fn from(value: ranged::$name<MIN, MAX>) -> Self {
                Self::new_saturating(value.get())
            }
        }

        /// Keeps the integer, which lies in atomiks' range, since the bounds are the same.
        impl<const MIN: $int, const MAX: $int> From<$name<MIN, MAX>> for ranged::$name<MIN, MAX> {
            #[inline]
            fn from(value: $name<MIN, MAX>) -> Self {
                Self::new_saturating(value.get())
            }
        }
    };
}

// Each of the twelve, the 128-bit ones too: a conversion needs no 16-byte atomic.
each_ranged_integer!(convert);

/// Refuses the build of `T`, an optional ranged integer whose range holds every integer, which
/// leaves `None` none.
const fn refuse_full_range<T>() -> ! {
    let rest = "`: its range holds every integer, so `None` has none left";
    refuse(&Message::new().text("`").name(type_name::<T>(), rest.len()).text(rest))
}

/// The integer `None` takes in an optional ranged integer of `$int` from `$min` to `$max`, as
/// deranged 0.5 keeps it: the lowest integer, or the highest where `$min` is the lowest. Refuses
/// the build of `$type` where the range holds every integer.
macro_rules! none_repr {
    ($type:ty, $int:ident, $min:ident, $max:ident) => {
        if $min != $int::MIN {
            $int::MIN
        } else if $max != $int::MAX {
            $int::MAX
        } else {
            refuse_full_range::<$type>()
        }
    };
}

/// Implements `Atom` for each of deranged's optional ranged integers, `None` at the integer
/// deranged keeps for it, as `none_repr` gives, and its range `$constructor`'s grown to hold it.
///
/// `ZeroValid` for the unsigned ones, whose zero is `Some(0)` or `None`; `Partial` for the signed
/// ones.
macro_rules! optional {
    ($($constructor:ident, $validity:ident: $($name:ident($ranged:ident, $int:ident)),+);+ $(;)?) => {$($(
        /// Stored as its integer, and `None` as the integer deranged keeps for it: the lowest, or
        /// the highest where `MIN` is the lowest. A range that holds every integer leaves `None`
        /// none, and its atomic is refused.
        ///
        /// It has no [`AtomOrd`]: deranged orders `None` below every value, which the repr's order
        /// keeps only where `None` is the lowest integer, not where `MIN` is.
        // SAFETY: `to_repr` gives a value's integer, from `MIN` to `MAX`, or `None`'s, which
        // `REPRS` holds beside them; `None`'s integer lies outside `MIN..=MAX`, the build refused
        // where none does, and `from_repr` tests for it first, by the repr alone, so each repr
        // decodes as the value it came from, and `new_unchecked` builds the same value from each
        // other one; for `ZeroValid`, zero is `Some(0)` where `MIN` is zero, else the unsigned
        // `None`; an integer may cross threads. Nothing here rests on where deranged keeps `None`.
        #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
        const unsafe impl<const MIN: $int, const MAX: $int> Atom
            for $name<MIN, MAX>
        {
            type Repr = $int;
            type Validity = $validity;
            const REPRS: ReprRange<$int> =
                ReprRange::$constructor(MIN.strict_cast(), MAX.strict_cast()).including(
                    ExactBits::to_bits(none_repr!(Self, $int, MIN, MAX)),
                );
            #[inline]
            fn to_repr(self) -> $int {
                match self.get() {
                    Some(value) => value.get(),
                    None => const { none_repr!(Self, $int, MIN, MAX) },
                }
            }
            #[inline]
            fn from_repr(repr: $int) -> Option<Self> {
                if repr == const { none_repr!(Self, $int, MIN, MAX) } {
                    return Some(Self::None);
                }
                match $ranged::<MIN, MAX>::new(repr) {
                    Some(value) => Some(Self::Some(value)),
                    None => None,
                }
            }
            #[inline]
            unsafe fn from_repr_unchecked(repr: $int) -> Self {
                if repr == const { none_repr!(Self, $int, MIN, MAX) } {
                    return Self::None;
                }
                // SAFETY: the caller's repr decodes, and it is not `None`'s, so it lies from `MIN`
                // to `MAX`.
                Self::Some(unsafe { $ranged::<MIN, MAX>::new_unchecked(repr) })
            }
        }
    )+)+};
}

optional! {
    new, ZeroValid:
        OptionRangedU8(RangedU8, u8), OptionRangedU16(RangedU16, u16),
        OptionRangedU32(RangedU32, u32), OptionRangedU64(RangedU64, u64),
        OptionRangedUsize(RangedUsize, usize);
    from_signed, Partial:
        OptionRangedI8(RangedI8, i8), OptionRangedI16(RangedI16, i16),
        OptionRangedI32(RangedI32, i32), OptionRangedI64(RangedI64, i64),
        OptionRangedIsize(RangedIsize, isize);
}

#[cfg(wide)]
optional! {
    new, ZeroValid: OptionRangedU128(RangedU128, u128);
    from_signed, Partial: OptionRangedI128(RangedI128, i128);
}
