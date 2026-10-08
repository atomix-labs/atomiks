//! The `Atom` laws over every built-in: a value's repr lies in its range, which may wrap, and
//! decodes back to it; a repr that decodes re-encodes to itself, unchecked too; each validity's
//! promise holds; a ranged integer's repr decodes, as itself, exactly where it lies in its range;
//! and `None` takes a spare repr: zero where zero is the niche, else one outside its value's range.
//!
//! Each law runs on every repr of a byte, on the edges of each width and range, and on random bits;
//! a 16-bit ranged integer's, on every repr of 16 bits too.

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#![cfg(not(loom))]
#![feature(f16, integer_casts)]
#![cfg_attr(wide, feature(f128))]

#[cfg(test)]
mod testing;

#[cfg(test)]
mod tests {
    use core::fmt::Debug;
    use core::marker::PhantomData;
    use core::num::{NonZero, Saturating, Wrapping};
    use core::ptr::{self, NonNull};

    #[cfg(target_arch = "aarch64")]
    use atomix_core::ordering::Relaxed;
    use atomix_core::validity::{Partial, Total, TotalZeroNiche, ZeroValid};
    use atomix_core::{
        Atom, ExactBits, Primitive, RangedI8, RangedI16, RangedI32, RangedI64, RangedIsize,
        RangedU8, RangedU16, RangedU32, RangedU64, RangedUsize,
    };
    #[cfg(target_arch = "aarch64")]
    use atomix_core::{AtomOrd, Atomic, Load, MinMax};
    #[cfg(wide)]
    use atomix_core::{RangedI128, RangedU128};
    use proptest::prelude::prop_oneof;
    use proptest::test_runner::TestCaseError;
    use proptest::{prop_assert, prop_assert_eq, proptest};

    use crate::testing::law::{
        assert_holds, canonical, decodes_as_promised, edge_or_random_bits, edges, none_laws,
        none_takes_a_spare_repr, ranged_integer_laws, reprs_order_as_ord, round_trips,
    };

    /// The last scalar value, and the first and last surrogates.
    const CHAR_EDGES: [u128; 3] = [0x10_FFFF, 0xD800, 0xDFFF];

    /// The range and round-trip laws for `value`, whose repr is a pointer.
    fn pointer_round_trips<T: Atom<Repr = *mut P> + PartialEq + Debug, P>(
        value: T,
    ) -> Result<(), TestCaseError> {
        let address = value.to_repr().addr();
        prop_assert!(
            u128::try_from(address).is_ok_and(|bits| T::REPRS.contains(bits)),
            "{value:?}'s address {address:#x} lies in {:?}",
            T::REPRS
        );
        prop_assert_eq!(T::from_repr(value.to_repr()), Some(value), "the value decodes back");
        canonical::<T>(value.to_repr())
    }

    /// The float laws for `bits`: they lie in range and are the repr of the float they make, which
    /// decodes back to them, checked and unchecked, NaN included.
    fn keeps_every_bit<T: Atom<Validity = Total> + Debug>(
        bits: u128, from_bits: fn(T::Repr) -> T,
    ) -> Result<(), TestCaseError>
    where
        T::Repr: ExactBits + PartialEq + Debug,
    {
        let repr = <T::Repr as Primitive>::from_bits(bits);
        let unsigned = repr.to_bits();
        prop_assert!(T::REPRS.contains(unsigned), "the bits {unsigned:#x} lie in {:?}", T::REPRS);
        prop_assert_eq!(from_bits(repr).to_repr(), repr, "the repr is the bits");
        decodes_as_promised::<Total, T>(repr)
    }

    /// The order law for `a` and `b` through the atomic maximum and minimum, which compare reprs:
    /// each keeps what `Ord`'s does, and returns the value before.
    #[cfg(target_arch = "aarch64")]
    fn atomics_order_as_ord<T: AtomOrd + Debug>(a: T, b: T) -> Result<(), TestCaseError>
    where
        T::Repr: Load + MinMax,
    {
        let larger = Atomic::from(a);
        prop_assert_eq!(larger.fetch_max(b, Relaxed), a, "fetch_max returns the value before");
        prop_assert_eq!(larger.load(Relaxed), a.max(b), "fetch_max keeps the larger");
        let smaller = Atomic::from(a);
        prop_assert_eq!(smaller.fetch_min(b, Relaxed), a, "fetch_min returns the value before");
        prop_assert_eq!(smaller.load(Relaxed), a.min(b), "fetch_min keeps the smaller");
        Ok(())
    }

    /// Checks every law on the low bits of `$bits` and `$other` as each integer type.
    ///
    /// The same laws run on its `NonZero`, the `Option` of that, `Wrapping` and `Saturating`.
    macro_rules! integer_laws {
        ($bits:expr, $other:expr; $($int:ty),+) => {$({
            let (a, b): ($int, $int) = ($bits.wrapping_cast(), $other.wrapping_cast());
            decodes_as_promised::<Total, $int>(a)?;
            decodes_as_promised::<TotalZeroNiche, NonZero<$int>>(a)?;
            decodes_as_promised::<Total, Option<NonZero<$int>>>(a)?;
            round_trips(a)?;
            round_trips(Wrapping(a))?;
            round_trips(Saturating(a))?;
            round_trips(NonZero::new(a))?;
            reprs_order_as_ord(a, b)?;
            if let (Some(a), Some(b)) = (NonZero::new(a), NonZero::new(b)) {
                round_trips(a)?;
                reprs_order_as_ord(a, b)?;
            }
        })+};
    }

    /// Checks the atomic order law on the low bits of `$bits` and `$other` as each integer type.
    ///
    /// The same law runs on its `NonZero`, `Wrapping` and `Saturating`.
    #[cfg(target_arch = "aarch64")]
    macro_rules! atomic_order_laws {
        ($bits:expr, $other:expr; $($int:ty),+) => {$({
            let (a, b): ($int, $int) = ($bits.wrapping_cast(), $other.wrapping_cast());
            atomics_order_as_ord(a, b)?;
            atomics_order_as_ord(Wrapping(a), Wrapping(b))?;
            atomics_order_as_ord(Saturating(a), Saturating(b))?;
            if let (Some(a), Some(b)) = (NonZero::new(a), NonZero::new(b)) {
                atomics_order_as_ord(a, b)?;
            }
        })+};
    }

    /// Every law on `bits` and `other` for each integer of 64 bits or fewer.
    fn narrow_integer_laws(bits: u128, other: u128) -> Result<(), TestCaseError> {
        integer_laws!(bits, other; u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);
        #[cfg(target_arch = "aarch64")]
        atomic_order_laws!(bits, other; u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);
        Ok(())
    }

    /// The atomic order law for the low bits of `bits` and `other` as `T`'s reprs, where both
    /// decode.
    #[cfg(target_arch = "aarch64")]
    fn ranged_atomics_order_as_ord<T: AtomOrd + Debug>(
        bits: u128, other: u128,
    ) -> Result<(), TestCaseError>
    where
        T::Repr: Load + MinMax,
    {
        let decode = |bits| T::from_repr(Primitive::from_bits(bits));
        if let (Some(a), Some(b)) = (decode(bits), decode(other)) {
            atomics_order_as_ord(a, b)
        } else {
            Ok(())
        }
    }

    /// Checks the ranged integer laws on the low bits of `$bits` and `$other` as each type's
    /// reprs, and on `aarch64` the atomic order law.
    macro_rules! ranged_laws {
        ($bits:expr, $other:expr; $($ranged:ty),+ $(,)?) => {$(
            ranged_integer_laws::<Partial, $ranged>($bits, $other)?;
            #[cfg(target_arch = "aarch64")]
            ranged_atomics_order_as_ord::<$ranged>($bits, $other)?;
        )+};
    }

    /// Checks the laws on the low bits of `$bits` as the repr of an `Option` of each type, whose
    /// validity is `Partial`.
    macro_rules! partial_option_laws {
        ($bits:expr; $($value:ty),+ $(,)?) => {$(
            decodes_as_promised::<Partial, Option<$value>>(Primitive::from_bits($bits))?;
        )+};
    }

    /// Every law on `bits` and `other` for the ranged integers of 16 bits, and on `bits` for an
    /// `Option` of each.
    fn sixteen_bit_ranged_laws(bits: u128, other: u128) -> Result<(), TestCaseError> {
        ranged_laws!(bits, other; RangedU16<3>, RangedI16<-300, 300>);
        partial_option_laws!(bits; RangedU16<3>, RangedI16<-300, 300>);
        Ok(())
    }

    /// Every law on `bits` and `other` for ranged integers of 64 bits or fewer, unsigned and
    /// signed: ranges below, through and above zero, full ones, one of a single integer, and the
    /// default `MAX`; and on `bits` for an `Option` of each of 16 bits or fewer that leaves `None`
    /// a repr.
    fn narrow_ranged_laws(bits: u128, other: u128) -> Result<(), TestCaseError> {
        ranged_laws!(bits, other; RangedU8<1, 10>, RangedU8<0, 255>, RangedI8<-5, 5>);
        ranged_laws!(bits, other; RangedI8<-100, -10>, RangedI8<{ i8::MIN }>);
        partial_option_laws!(bits; RangedU8<1, 10>, RangedI8<-5, 5>, RangedI8<-100, -10>);
        sixteen_bit_ranged_laws(bits, other)?;
        ranged_laws!(bits, other; RangedU32<7, 7>, RangedI32<{ i32::MIN }, -1>);
        ranged_laws!(bits, other; RangedU64<3>, RangedI64<{ i64::MIN }, 5>);
        ranged_laws!(bits, other; RangedUsize<1>, RangedIsize<-1, 1>);
        Ok(())
    }

    /// Every law on `bits` as the repr of the 128-bit values.
    #[cfg(wide)]
    fn wide_laws(bits: u128, other: u128) -> Result<(), TestCaseError> {
        integer_laws!(bits, other; u128, i128);
        ranged_integer_laws::<Partial, RangedU128<1>>(bits, other)?;
        ranged_integer_laws::<Partial, RangedI128<{ i128::MIN }, -1>>(bits, other)?;
        ranged_integer_laws::<Partial, RangedI128<-5, 5>>(bits, other)?;
        keeps_every_bit(bits, f128::from_bits)
    }

    /// Every law on the low bits of `bits` as the repr of a `char` and of its `Option`s.
    ///
    /// The order laws compare the `char` they make, if any, with `other`.
    fn char_laws(bits: u128, other: char) -> Result<(), TestCaseError> {
        let repr = u32::from_bits(bits);
        decodes_as_promised::<ZeroValid, char>(repr)?;
        decodes_as_promised::<ZeroValid, Option<char>>(repr)?;
        decodes_as_promised::<ZeroValid, Option<Option<char>>>(repr)?;
        round_trips(None::<char>)?;
        round_trips(None::<Option<char>>)?;
        if let Some(letter) = char::from_u32(repr) {
            round_trips(letter)?;
            round_trips(Some(letter))?;
            reprs_order_as_ord(letter, other)?;
            #[cfg(target_arch = "aarch64")]
            atomics_order_as_ord(letter, other)?;
        }
        Ok(())
    }

    /// Every law on the low bits of `bits` as each float's.
    fn float_laws(bits: u128) -> Result<(), TestCaseError> {
        keeps_every_bit(bits, f16::from_bits)?;
        keeps_every_bit(bits, f32::from_bits)?;
        keeps_every_bit(bits, f64::from_bits)
    }

    /// Every law on the low bits of `bits` as an address, for each pointer and `Option<NonNull>`.
    fn pointer_laws(bits: u128) -> Result<(), TestCaseError> {
        let pointer = ptr::without_provenance_mut::<u8>(bits.wrapping_cast());
        pointer_round_trips(pointer)?;
        pointer_round_trips(pointer.cast_const())?;
        pointer_round_trips(NonNull::new(pointer))?;
        if let Some(value) = NonNull::new(pointer) {
            pointer_round_trips(value)?;
        }
        decodes_as_promised::<Total, *mut u8>(pointer)?;
        decodes_as_promised::<Total, *const u8>(pointer)?;
        decodes_as_promised::<TotalZeroNiche, NonNull<u8>>(pointer)?;
        decodes_as_promised::<Total, Option<NonNull<u8>>>(pointer)
    }

    #[test]
    fn every_byte_obeys_the_laws() {
        for bits in 0..=u128::from(u8::MAX) {
            assert_holds(narrow_integer_laws(bits, bits.wrapping_add(1)));
            assert_holds(narrow_ranged_laws(bits, bits.wrapping_add(1)));
            let byte = u8::from_bits(bits);
            assert_holds(decodes_as_promised::<ZeroValid, ()>(byte));
            assert_holds(decodes_as_promised::<ZeroValid, PhantomData<str>>(byte));
            assert_holds(decodes_as_promised::<ZeroValid, Option<()>>(byte));
            assert_holds(decodes_as_promised::<ZeroValid, Option<Option<()>>>(byte));
            assert_holds(decodes_as_promised::<ZeroValid, Option<PhantomData<str>>>(byte));
        }
        for value in [false, true] {
            assert_holds(decodes_as_promised::<Total, bool>(value));
            assert_holds(round_trips(value));
        }
        assert_holds(round_trips(()));
        assert_holds(round_trips(PhantomData::<str>));
        assert_holds(round_trips(Some(())));
        assert_holds(round_trips(None::<()>));
        assert_holds(round_trips(None::<Option<()>>));
    }

    #[test]
    fn every_repr_of_16_bits_obeys_the_ranged_laws() {
        for bits in 0..=u128::from(u16::MAX) {
            assert_holds(sixteen_bit_ranged_laws(bits, bits.wrapping_add(1)));
        }
    }

    #[test]
    fn every_edge_obeys_the_laws() {
        for bits in edges(&CHAR_EDGES) {
            assert_holds(narrow_integer_laws(bits, bits.wrapping_sub(1)));
            assert_holds(narrow_ranged_laws(bits, bits.wrapping_sub(1)));
            assert_holds(char_laws(bits, char::MAX));
            assert_holds(float_laws(bits));
            assert_holds(pointer_laws(bits));
            #[cfg(wide)]
            assert_holds(wide_laws(bits, bits.wrapping_sub(1)));
        }
    }

    #[test]
    fn none_takes_a_spare_repr_of_each_built_in() {
        none_laws!(NonZero<u8>, NonZero<u16>, NonZero<u32>, NonZero<u64>, NonZero<usize>);
        none_laws!(NonZero<i8>, NonZero<i16>, NonZero<i32>, NonZero<i64>, NonZero<isize>);
        none_laws!(char, Option<char>, (), Option<()>, PhantomData<str>);
        none_laws!(RangedU8<1, 10>, RangedI8<-5, 5>, RangedI8<-100, -10>, RangedU16<3>);
        none_laws!(RangedI16<-300, 300>, RangedU32<7, 7>, RangedI32<{ i32::MIN }, -1>);
        none_laws!(RangedU64<3>, RangedI64<{ i64::MIN }, 5>, RangedUsize<1>, RangedIsize<-1, 1>);
        #[cfg(wide)]
        none_laws!(NonZero<u128>, NonZero<i128>);
        #[cfg(wide)]
        none_laws!(RangedU128<1>, RangedI128<{ i128::MIN }, -1>, RangedI128<-5, 5>);
        let null = None::<NonNull<u8>>.to_repr().addr();
        assert_holds(none_takes_a_spare_repr::<NonNull<u8>>(
            u128::try_from(null).expect("an address fits a `u128`"),
        ));
    }

    #[test]
    fn an_option_of_a_nonzero_or_non_null_is_total() {
        /// Compiles only where `T` declares every repr decodes.
        const fn is_total<T: Atom<Validity = Total>>() {}
        is_total::<Option<NonZero<u8>>>();
        is_total::<Option<NonZero<i64>>>();
        is_total::<Option<NonNull<u8>>>();
    }

    proptest! {
        #[test]
        fn integers_obey_the_laws(bits in edge_or_random_bits(&[]), other: u128) {
            narrow_integer_laws(bits, other)?;
        }

        #[test]
        fn ranged_integers_obey_the_laws(bits in edge_or_random_bits(&[]), other: u128) {
            narrow_ranged_laws(bits, other)?;
        }

        #[test]
        fn chars_obey_the_laws(
            bits in prop_oneof![edge_or_random_bits(&CHAR_EDGES), 0..=0x11_0001_u128], other: char,
        ) {
            char_laws(bits, other)?;
        }

        #[test]
        fn floats_keep_every_bit(bits in edge_or_random_bits(&[])) {
            float_laws(bits)?;
        }

        #[test]
        fn pointers_obey_the_laws(bits in edge_or_random_bits(&[])) {
            pointer_laws(bits)?;
        }
    }

    #[cfg(wide)]
    proptest! {
        #[test]
        fn wide_values_obey_the_laws(bits in edge_or_random_bits(&[]), other: u128) {
            wide_laws(bits, other)?;
        }
    }
}
