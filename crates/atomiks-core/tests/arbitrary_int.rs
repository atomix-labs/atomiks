//! arbitrary-int's integers in an atomic: each stored as its base integer, every repr of an 8-bit
//! base decoding exactly when it is a value; the `Atom` laws at the edges of each width, and on
//! random bits; `None` beside the range; statics; and `fetch_max` and `fetch_min`, through zero for
//! the signed, on aarch64.
//!
//! The dev-dependency turns on arbitrary-int's `hint`, so these run against the `value` of each
//! base it swaps in, whose promise that the value fits Miri checks; the library builds without it,
//! as in `check-rust-doc` and a user's build.

#![cfg(feature = "arbitrary-int")]

// Under loom the impls stay on, since none reads an atomic's memory as bytes; `check-loom` builds
// this.
#[cfg(loom)]
const _: () = {
    use arbitrary_int::{i20, u20};
    use atomiks_core::AtomOrd;

    /// Compiles only where `T`'s reprs order as its values.
    const fn orders_as_its_reprs<T: AtomOrd>() {}

    orders_as_its_reprs::<u20>();
    orders_as_its_reprs::<i20>();
};

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#[cfg(not(loom))]
#[cfg(test)]
mod testing;

#[cfg(not(loom))]
#[cfg(test)]
mod tests {
    use arbitrary_int::traits::Integer;
    use arbitrary_int::{
        Int, UInt, i1, i3, i5, i7, i12, i15, i20, i31, i33, i63, u1, u3, u5, u7, u12, u15, u20,
        u31, u33, u63,
    };
    #[cfg(wide)]
    use arbitrary_int::{i65, i127, u65, u127};
    #[cfg(wide)]
    use atomiks_core::ordering::AcqRel;
    #[cfg(target_arch = "aarch64")]
    use atomiks_core::ordering::Relaxed;
    use atomiks_core::ordering::{Acquire, Release};
    use atomiks_core::validity::ZeroValid;
    use atomiks_core::{Atom, Atomic, ReprRange};
    use proptest::proptest;
    use proptest::test_runner::TestCaseError;

    use crate::testing::atom::{decodes_exactly_its_values, field_width, repr_and_validity_are};
    use crate::testing::law::{
        assert_holds, edge_or_random_bits, edges, none_laws, ranged_integer_laws,
    };

    /// The widths the laws run on beside each base's own: one bit, a few, one short of a base, and
    /// one past the base below.
    const WIDTHS: [u32; 11] = [1, 5, 7, 12, 15, 20, 31, 33, 63, 65, 127];

    /// A 20-bit sequence number, in a static: the impl is `const`.
    static SEQ: Atomic<u20> = Atomic::new(u20::new(5));
    /// A 20-bit signed offset, or `None` before the first.
    static OFFSET: Atomic<Option<i20>> = Atomic::new(None);

    #[test]
    fn each_is_stored_as_its_base_and_zero_decodes() {
        const {
            repr_and_validity_are::<u3, u8, ZeroValid>();
            repr_and_validity_are::<u20, u32, ZeroValid>();
            repr_and_validity_are::<u63, u64, ZeroValid>();
            repr_and_validity_are::<i3, i8, ZeroValid>();
            repr_and_validity_are::<i20, i32, ZeroValid>();
            repr_and_validity_are::<Option<u3>, u8, ZeroValid>();
        }
    }

    #[test]
    fn the_range_is_the_width_unsigned_or_signed() {
        assert_eq!(u1::REPRS, ReprRange::new(0, 1), "0 and 1");
        assert_eq!(u3::REPRS, ReprRange::new(0, 7), "0 to 7");
        assert_eq!(u63::REPRS, ReprRange::new(0, u128::from(u64::MAX >> 1)), "0 to 2^63 - 1");
        assert_eq!(i1::REPRS, ReprRange::from_signed(-1, 0), "-1 and 0");
        assert_eq!(i3::REPRS, ReprRange::from_signed(-4, 3), "-4 to 3, through zero");
        assert_eq!(
            i63::REPRS,
            ReprRange::from_signed(-(1 << 62), (1 << 62) - 1),
            "-2^62 to 2^62 - 1"
        );
        assert_eq!(UInt::<u8, 8>::REPRS, ReprRange::FULL, "the base's full width");
        assert_eq!(Int::<i8, 8>::REPRS, ReprRange::FULL, "signed too");
    }

    #[test]
    fn a_field_takes_the_width_alone() {
        assert_eq!(field_width(u3::REPRS), 3, "u3 in three bits");
        assert_eq!(field_width(i3::REPRS), 3, "i3 in three, sign-extended");
        assert_eq!(field_width(u20::REPRS), 20, "u20 in twenty");
        assert_eq!(field_width(i20::REPRS), 20, "i20 in twenty");
    }

    /// Checks each 8-bit width's every repr: exactly the values decode, each as itself.
    macro_rules! each_8_bit_width {
        ($($bits:literal)+) => {$(
            decodes_exactly_its_values::<UInt<u8, $bits>, _>(0..=u8::MAX, 1 << $bits);
            decodes_exactly_its_values::<Int<i8, $bits>, _>(i8::MIN..=i8::MAX, 1 << $bits);
        )+};
    }

    #[test]
    fn every_byte_decodes_exactly_when_it_is_a_value() {
        each_8_bit_width!(1 2 3 4 5 6 7 8);
    }

    #[test]
    fn values_round_trip_at_the_edges() {
        for value in [u20::new(0), u20::new(1), u20::MAX] {
            assert_eq!(u20::from_repr(value.to_repr()), Some(value), "{value:?} round-trips");
        }
        for value in [i20::MIN, i20::new(-1), i20::new(0), i20::MAX] {
            assert_eq!(i20::from_repr(value.to_repr()), Some(value), "{value:?} round-trips");
        }
        assert_eq!(i3::new(-1).to_repr(), -1, "sign-extended, as the base reads it");
    }

    #[test]
    fn a_repr_past_the_width_is_refused() {
        assert_eq!(u20::from_repr(1 << 20), None, "2^20 is past u20");
        assert_eq!(i20::from_repr(1 << 19), None, "2^19 is past i20");
        assert_eq!(i20::from_repr(-(1 << 19) - 1), None, "and below it");
        assert_eq!(i3::from_repr(0b0000_0111), None, "i3's bits, unextended, are not -1");
    }

    #[test]
    fn none_takes_the_repr_beside_the_range() {
        assert_eq!(None::<u3>.to_repr(), 8, "above 7");
        assert_eq!(Option::<u3>::REPRS, ReprRange::new(0, 8), "0 to 8");
        assert_eq!(Option::<u3>::from_repr(8), Some(None), "8 decodes as `None`");
        assert_eq!(Option::<u3>::from_repr(9), None, "9 decodes as nothing");
        assert_eq!(Option::<u3>::from_repr(7), Some(Some(u3::new(7))), "7 as itself");
        assert_eq!(field_width(Option::<u3>::REPRS), 4, "one bit more");
        assert_eq!(None::<u7>.to_repr(), 128, "and above 127");
        assert_eq!(None::<i3>.to_repr(), -5, "below -4");
    }

    #[test]
    fn statics_hold_the_values_stored() {
        assert_eq!(SEQ.load(Acquire), u20::new(5), "the value built in const");
        SEQ.store(u20::MAX, Release);
        assert_eq!(SEQ.load(Acquire), u20::MAX, "the largest");
        assert_eq!(OFFSET.load(Acquire), None, "`None` built in const");
        OFFSET.store(Some(i20::MIN), Release);
        assert_eq!(OFFSET.load(Acquire), Some(i20::MIN), "the smallest");
    }

    /// Each of `WIDTHS`' largest unsigned and signed values, and its smallest signed one, as the
    /// base reads it.
    fn width_edges() -> Vec<u128> {
        let each = |width: u32| {
            let top = u128::MAX.unbounded_shr(128_u32.strict_sub(width));
            [top, top.unbounded_shr(1), u128::MAX.unbounded_shl(width.strict_sub(1))]
        };
        WIDTHS.into_iter().flat_map(each).collect()
    }

    /// Every law on `bits` and `other` for each of `WIDTHS`, and each base's own width.
    fn every_width_laws(bits: u128, other: u128) -> Result<(), TestCaseError> {
        /// Checks the laws for each type.
        macro_rules! laws {
            ($($int:ty),+ $(,)?) => {$(ranged_integer_laws::<ZeroValid, $int>(bits, other)?;)+};
        }
        laws!(u1, u5, u7, UInt<u8, 8>, u12, u15, UInt<u16, 16>, u20, u31, UInt<u32, 32>, u33, u63);
        laws!(UInt<u64, 64>, i1, i5, i7, Int<i8, 8>, i12, i15, Int<i16, 16>, i20, i31);
        laws!(Int<i32, 32>, i33, i63, Int<i64, 64>);
        #[cfg(wide)]
        laws!(u65, u127, UInt<u128, 128>, i65, i127, Int<i128, 128>);
        Ok(())
    }

    #[test]
    fn every_edge_of_every_width_obeys_the_laws() {
        for bits in edges(&width_edges()) {
            assert_holds(every_width_laws(bits, bits.wrapping_sub(1)));
        }
    }

    #[test]
    fn none_takes_a_spare_repr_of_each_width() {
        none_laws!(u1, u3, u12, u33, u63, i1, i3, i12, i33, i63);
        #[cfg(wide)]
        none_laws!(u65, u127, i65, i127);
    }

    proptest! {
        #[test]
        fn arbitrary_width_integers_obey_the_laws(
            bits in edge_or_random_bits(&width_edges()),
            other in edge_or_random_bits(&width_edges()),
        ) {
            every_width_laws(bits, other)?;
        }
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn max_and_min_follow_the_order_through_zero() {
        let unsigned = Atomic::new(u20::new(7));
        assert_eq!(unsigned.fetch_max(u20::MAX, Relaxed), u20::new(7), "7 before");
        assert_eq!(unsigned.fetch_min(u20::new(3), Relaxed), u20::MAX, "the largest before");
        assert_eq!(unsigned.load(Relaxed), u20::new(3), "then 3");
        let signed = Atomic::new(i20::new(-1));
        signed.fetch_max(i20::new(1), Relaxed);
        assert_eq!(signed.load(Relaxed), i20::new(1), "1 is above -1");
        signed.fetch_min(i20::MIN, Relaxed);
        assert_eq!(signed.load(Relaxed), i20::MIN, "the smallest is below 1");
        signed.fetch_max(i20::new(-3), Relaxed);
        assert_eq!(signed.load(Relaxed), i20::new(-3), "-3 is above the smallest");
    }

    #[cfg(wide)]
    #[test]
    fn the_128_bit_bases_store_where_a_16_byte_exchange_exists() {
        assert_eq!(u127::REPRS, ReprRange::new(0, u128::MAX >> 1), "0 to 2^127 - 1");
        assert_eq!(
            i127::REPRS,
            ReprRange::from_signed(i128::MIN >> 1, i128::MAX >> 1),
            "-2^126 to 2^126 - 1"
        );
        let wide = Atomic::new(u127::MAX);
        assert_eq!(wide.update(AcqRel, Acquire, |_| u127::new(1)), u127::MAX, "the largest");
        assert_eq!(wide.into_inner(), u127::new(1), "then 1");
        assert_eq!(u127::from_repr(1 << 127), None, "2^127 is past u127");
    }
}
