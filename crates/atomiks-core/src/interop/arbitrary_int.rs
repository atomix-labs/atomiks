//! `Atom` for arbitrary-int's integers of any width, `UInt<_, BITS>` and `Int<_, BITS>`, each
//! stored as its base integer; and, in a field of a packed value, which stores one in its `BITS`
//! bits, the bitwise operations and the add at the top.
//!
//! One impl per base, by macro: with arbitrary-int's `hint` feature, which another crate in the
//! build may turn on, `value` exists only for each base, not generically.

use arbitrary_int::traits::Integer;
use arbitrary_int::{Int, UInt};

use crate::atom::{Atom, AtomOrd, FieldAdd, FieldBitwise};
use crate::range::ReprRange;
use crate::validity::ZeroValid;

/// Implements `Atom`, `AtomOrd`, `FieldBitwise` and `FieldAdd` for `UInt` over each base, stored
/// as the base integer.
///
/// `ZeroValid`, not `Total`, even where `BITS` is the base's width: a validity cannot be chosen
/// from `BITS`. Neither `AtomAdd` nor `AtomBitwise`, which need `Total`: the base's wrapping add,
/// and its `not`, leave `0..=MAX`. A field stores the `BITS` bits alone, so there an add or a `not`
/// stays in them: `FieldBitwise` and `FieldAdd`.
macro_rules! unsigned {
    ($($int:ident),+ $(,)?) => {$(
        #[doc = concat!("Stored as its `", stringify!($int), "`, from 0 to `MAX`.")]
        ///
        /// A field of a derived value takes its `BITS` bits alone, and zero decodes.
        // SAFETY: arbitrary-int keeps every `UInt` from 0 to `MAX`: its field is private, each safe
        // constructor and operation checks or wraps into the width, and `new_unchecked`'s contract
        // asks it of the caller. So the repr, the value, lies within `REPRS`; `try_new` decodes
        // exactly those, each as itself, by the repr alone, and `new_unchecked` builds the same
        // value from each; zero is a value, as `ZeroValid` promises; an integer may cross threads.
        #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
        const unsafe impl<const BITS: usize> Atom for UInt<$int, BITS> {
            type Repr = $int;
            type Validity = ZeroValid;
            const REPRS: ReprRange<$int> = ReprRange::new(0, Self::MASK.strict_cast());
            #[inline]
            fn to_repr(self) -> $int {
                self.value()
            }
            #[inline]
            fn from_repr(repr: $int) -> Option<Self> {
                match Self::try_new(repr) {
                    Ok(value) => Some(value),
                    Err(_past_the_width) => None,
                }
            }
            #[inline]
            unsafe fn from_repr_unchecked(repr: $int) -> Self {
                // SAFETY: the caller's repr decodes, so it is at most `MAX`.
                unsafe { Self::new_unchecked(repr) }
            }
        }

        /// Ordered as its integer, which the base's unsigned order is.
        impl<const BITS: usize> AtomOrd for UInt<$int, BITS> {}

        /// And, or, xor and not on the `BITS` bits a field stores it in are its own.
        // `do_not_recommend`, so a field of a value without `FieldBitwise` lists no other impls,
        // where it would list these alone, and only with this feature on.
        //
        // SAFETY: a packed value stores it in the `BITS` bits its `REPRS`, 0 to `MAX`, need, read
        // back zero-extended, so every pattern of them is a value, which decodes; and on them,
        // `and`, `or` and `xor` are its own `&`, `|` and `^`, and `not` its `!`, which flips the
        // `BITS` bits.
        #[diagnostic::do_not_recommend]
        #[expect(unsafe_code, reason = "bitwise operations write any pattern of a field's bits")]
        unsafe impl<const BITS: usize> FieldBitwise for UInt<$int, BITS> {}

        /// Wrapping add and subtract on the `BITS` bits a field stores it in are its own.
        // `do_not_recommend`, as for `FieldBitwise`.
        //
        // SAFETY: as for `FieldBitwise`, every pattern of the `BITS` bits decodes; and at the top
        // of the repr, where the carry or borrow leaves the word, an add or subtract on them wraps
        // modulo 2^BITS, as its `wrapping_add` and `wrapping_sub` do.
        #[diagnostic::do_not_recommend]
        #[expect(unsafe_code, reason = "a field's add writes any pattern of its bits")]
        unsafe impl<const BITS: usize> FieldAdd for UInt<$int, BITS> {}
    )+};
}

/// Implements `Atom`, `AtomOrd`, `FieldBitwise` and `FieldAdd` for `Int` over each base, stored as
/// the base integer, its value sign-extended, so the base's signed order is the value's.
///
/// As for `UInt`, `ZeroValid`, neither `AtomAdd` nor `AtomBitwise`, and `FieldBitwise` and
/// `FieldAdd`.
macro_rules! signed {
    ($($int:ident),+ $(,)?) => {$(
        #[doc = concat!("Stored as its `", stringify!($int), "`, from `MIN` to `MAX`, sign-extended.")]
        ///
        /// A field of a derived value takes its `BITS` bits alone, and zero decodes.
        // SAFETY: arbitrary-int keeps every `Int` from `MIN` to `MAX`: its field is private, each
        // safe constructor and operation checks or wraps into the width, and `new_unchecked`'s
        // contract asks it of the caller. So the repr, the value as the base reads it, lies within
        // `REPRS`, which holds their bits in signed order; `try_new` decodes exactly those, each as
        // itself, by the repr alone, and `new_unchecked` builds the same value from each; zero is a
        // value, as `ZeroValid` promises; an integer may cross threads.
        #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
        const unsafe impl<const BITS: usize> Atom for Int<$int, BITS> {
            type Repr = $int;
            type Validity = ZeroValid;
            const REPRS: ReprRange<$int> = ReprRange::from_signed(
                <Self as Integer>::MIN.value().strict_cast(),
                <Self as Integer>::MAX.value().strict_cast(),
            );
            #[inline]
            fn to_repr(self) -> $int {
                self.value()
            }
            #[inline]
            fn from_repr(repr: $int) -> Option<Self> {
                match Self::try_new(repr) {
                    Ok(value) => Some(value),
                    Err(_past_the_width) => None,
                }
            }
            #[inline]
            unsafe fn from_repr_unchecked(repr: $int) -> Self {
                // SAFETY: the caller's repr decodes, so it lies from `MIN` to `MAX`.
                unsafe { Self::new_unchecked(repr) }
            }
        }

        /// Ordered as its integer, which the base's signed order is.
        impl<const BITS: usize> AtomOrd for Int<$int, BITS> {}

        /// And, or, xor and not on the `BITS` bits a field stores it in are its own.
        // `do_not_recommend`, as for `UInt`'s.
        //
        // SAFETY: a packed value stores it in the `BITS` bits its `REPRS`, `MIN` to `MAX`, need,
        // read back sign-extended, or as they are where `BITS` is the base's width, so every
        // pattern of them is a value, which decodes; and sign extension keeps `and`, `or`, `xor`
        // and `not`, so on those bits they are its own `&`, `|`, `^` and `!`.
        #[diagnostic::do_not_recommend]
        #[expect(unsafe_code, reason = "bitwise operations write any pattern of a field's bits")]
        unsafe impl<const BITS: usize> FieldBitwise for Int<$int, BITS> {}

        /// Wrapping add and subtract on the `BITS` bits a field stores it in are its own.
        // `do_not_recommend`, as for `UInt`'s.
        //
        // SAFETY: as for `FieldBitwise`, every pattern of the `BITS` bits decodes; and at the top
        // of the repr, where the carry or borrow leaves the word, an add or subtract on them wraps
        // modulo 2^BITS, which in two's complement is its `wrapping_add` and `wrapping_sub`.
        #[diagnostic::do_not_recommend]
        #[expect(unsafe_code, reason = "a field's add writes any pattern of its bits")]
        unsafe impl<const BITS: usize> FieldAdd for Int<$int, BITS> {}
    )+};
}

unsigned!(u8, u16, u32, u64);
signed!(i8, i16, i32, i64);

// The 128-bit bases, where a 16-byte compare-exchange exists, as for `u128` and `i128`.
#[cfg(wide)]
unsigned!(u128);
#[cfg(wide)]
signed!(i128);
