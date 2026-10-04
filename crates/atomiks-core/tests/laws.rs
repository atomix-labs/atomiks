//! The `Atom` laws over every built-in: a value's repr lies in its range, which may wrap, and
//! decodes back to it; a repr that decodes re-encodes to itself, unchecked too; each validity's
//! promise holds; and `None` takes a spare repr: zero where zero is the niche, else one outside its
//! value's range.
//!
//! Each law runs on every repr of a byte, on the edges of each width and range, and on random bits.

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
    use atomiks_core::ordering::Relaxed;
    use atomiks_core::validity::{Total, TotalZeroNiche, ZeroValid};
    use atomiks_core::{Atom, AtomOrd, ExactBits, Primitive};
    #[cfg(target_arch = "aarch64")]
    use atomiks_core::{Atomic, Load, MinMax};
    use proptest::prelude::prop_oneof;
    use proptest::test_runner::TestCaseError;
    use proptest::{prop_assert, prop_assert_eq, proptest};

    use crate::testing::law::{
        assert_holds, canonical, decodes_as_promised, edge_or_random_bits, edges, none_bits,
        none_takes_a_spare_repr, round_trips,
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

    /// The order law for `a` and `b`: the reprs' own order is the values'.
    fn reprs_order_as_ord<T: AtomOrd + Debug>(a: T, b: T) -> Result<(), TestCaseError>
    where
        T::Repr: Ord,
    {
        let (values, reprs) = (a.cmp(&b), a.to_repr().cmp(&b.to_repr()));
        prop_assert_eq!(reprs, values, "{:?} and {:?} order as their reprs do", a, b);
        Ok(())
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

    /// Every law on `bits` as the repr of the 128-bit values.
    #[cfg(wide)]
    fn wide_laws(bits: u128, other: u128) -> Result<(), TestCaseError> {
        integer_laws!(bits, other; u128, i128);
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
    fn every_edge_obeys_the_laws() {
        for bits in edges(&CHAR_EDGES) {
            assert_holds(narrow_integer_laws(bits, bits.wrapping_sub(1)));
            assert_holds(char_laws(bits, char::MAX));
            assert_holds(float_laws(bits));
            assert_holds(pointer_laws(bits));
            #[cfg(wide)]
            assert_holds(wide_laws(bits, bits.wrapping_sub(1)));
        }
    }

    /// Checks the `None` law for each integer's `NonZero`.
    macro_rules! nonzero_none_laws {
        ($($int:ty),+) => {$(
            assert_holds(none_takes_a_spare_repr::<NonZero<$int>>(none_bits::<NonZero<$int>>()));
        )+};
    }

    #[test]
    fn none_takes_a_spare_repr_of_each_built_in() {
        nonzero_none_laws!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);
        #[cfg(wide)]
        nonzero_none_laws!(u128, i128);
        assert_holds(none_takes_a_spare_repr::<char>(none_bits::<char>()));
        assert_holds(none_takes_a_spare_repr::<Option<char>>(none_bits::<Option<char>>()));
        assert_holds(none_takes_a_spare_repr::<()>(none_bits::<()>()));
        assert_holds(none_takes_a_spare_repr::<Option<()>>(none_bits::<Option<()>>()));
        assert_holds(none_takes_a_spare_repr::<PhantomData<str>>(none_bits::<PhantomData<str>>()));
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
