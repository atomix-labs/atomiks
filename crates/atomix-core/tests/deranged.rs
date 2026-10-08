//! deranged 0.5's ranged integers in an atomic: each stored as its integer, every repr of an 8-bit
//! one decoding exactly when it is a value, and the `Atom` laws at the edges and on random bits; an
//! `OptionRanged`'s `None` at the integer deranged keeps for it, beside atomix's own `Option` rule;
//! the conversions to atomix's ranged integers and back, for all twelve; statics; and `fetch_max`
//! and `fetch_min` on aarch64.
//!
//! A value compared with another names its bounds, by an alias or in full, as a user's code must:
//! deranged's `PartialEq` holds across bounds, so a bare `RangedU8::MIN` infers none.

#![cfg(feature = "deranged-05")]

// Under loom the impls stay on, since none reads an atomic's memory as bytes; `check-loom` builds
// this.
#[cfg(loom)]
const _: () = {
    use atomix_core::{Atom, AtomOrd};
    use deranged::{OptionRangedU8, RangedU8};

    /// Compiles only where `T`'s reprs order as its values.
    const fn orders_as_its_reprs<T: AtomOrd>() {}
    /// Compiles only where `T` is an `Atom`.
    const fn stores<T: Atom>() {}
    /// Compiles only where `T` converts to `U` and back.
    const fn converts<T: From<U> + Into<U>, U>() {}
    /// atomix's own ranged integer of the same bounds.
    type Ours = atomix_core::RangedU8<1, 10>;

    orders_as_its_reprs::<RangedU8<1, 10>>();
    stores::<OptionRangedU8<1, 10>>();
    converts::<RangedU8<1, 10>, Ours>();
};

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#[cfg(not(loom))]
#[cfg(test)]
mod testing;

#[cfg(not(loom))]
#[cfg(test)]
mod tests {
    use core::cmp::Ordering;
    use core::fmt::Debug;
    use core::mem::transmute_copy;

    #[cfg(target_arch = "aarch64")]
    use atomix_core::ordering::Relaxed;
    use atomix_core::ordering::{Acquire, Release};
    use atomix_core::validity::{Partial, Validity, ZeroValid};
    use atomix_core::{Atom, Atomic, ExactBits, Primitive, ReprRange};
    use deranged::{
        OptionRangedI8, OptionRangedI32, OptionRangedIsize, OptionRangedU8, OptionRangedU16,
        OptionRangedU64, OptionRangedUsize, RangedI8, RangedI16, RangedI32, RangedI64, RangedIsize,
        RangedU8, RangedU16, RangedU32, RangedU64, RangedUsize,
    };
    #[cfg(wide)]
    use deranged::{OptionRangedI128, OptionRangedU128, RangedI128, RangedU128};
    use proptest::proptest;
    use proptest::test_runner::TestCaseError;

    use crate::testing::atom::{decodes_exactly_its_values, field_width, repr_and_validity_are};
    use crate::testing::law::{
        Promise, assert_holds, decodes_as_promised, edge_or_random_bits, edges, none_laws,
        ranged_integer_laws, round_trips,
    };

    /// How many levels of the book a feed keeps, from 3 to 100.
    type Depth = RangedU32<3, 100>;
    /// An owner's id from 1, or `None`, at zero.
    type OwnerId = OptionRangedU64<1, { u64::MAX }>;
    /// How far a price moves in one update, in ticks, from -5 to 5.
    type PriceMove = RangedI8<-5, 5>;

    /// How many levels the feed keeps.
    static DEPTH: Atomic<Depth> = Atomic::new(Depth::MIN);
    /// The slot's owner.
    static OWNER: Atomic<OwnerId> = Atomic::new(OwnerId::None);

    /// The bounds of the ranges the laws run on, as unsigned bits, beside each width's edges.
    fn bounds() -> Vec<u128> {
        [3, 5, 10, 100, 300, -5, -300].into_iter().map(i128::cast_unsigned).collect()
    }

    /// The laws for the low bits of `bits` as `T`'s repr, whose validity is `V`: it decodes as `V`
    /// promises, and the value it decodes to, if any, round-trips.
    fn optional_laws<T, V>(bits: u128) -> Result<(), TestCaseError>
    where
        T: Atom<Validity = V> + PartialEq + Debug,
        T::Repr: ExactBits + PartialEq + Debug,
        V: Validity + Promise,
    {
        let repr = <T::Repr as Primitive>::from_bits(bits);
        decodes_as_promised::<V, T>(repr)?;
        T::from_repr(repr).map_or(Ok(()), round_trips)
    }

    /// Every law on `bits` and `other` for ranged integers of each width, and on `bits` for
    /// optional ones: ranges below, through and above zero, full ones, and one of a single
    /// integer; `None` low, and high where `MIN` is the lowest integer.
    fn every_law(bits: u128, other: u128) -> Result<(), TestCaseError> {
        ranged_integer_laws::<Partial, RangedU8<1, 10>>(bits, other)?;
        ranged_integer_laws::<Partial, RangedU8<0, 255>>(bits, other)?;
        ranged_integer_laws::<Partial, PriceMove>(bits, other)?;
        ranged_integer_laws::<Partial, RangedI8<{ i8::MIN }, 127>>(bits, other)?;
        ranged_integer_laws::<Partial, RangedU16<3, { u16::MAX }>>(bits, other)?;
        ranged_integer_laws::<Partial, RangedI16<-300, 300>>(bits, other)?;
        ranged_integer_laws::<Partial, RangedU32<7, 7>>(bits, other)?;
        ranged_integer_laws::<Partial, RangedI32<{ i32::MIN }, -1>>(bits, other)?;
        ranged_integer_laws::<Partial, RangedU64<3, { u64::MAX }>>(bits, other)?;
        ranged_integer_laws::<Partial, RangedI64<{ i64::MIN }, 5>>(bits, other)?;
        ranged_integer_laws::<Partial, RangedUsize<1, { usize::MAX }>>(bits, other)?;
        ranged_integer_laws::<Partial, RangedIsize<-1, 1>>(bits, other)?;
        optional_laws::<OptionRangedU8<3, 10>, _>(bits)?;
        optional_laws::<OptionRangedU8<0, 10>, _>(bits)?;
        optional_laws::<OptionRangedI8<-5, 5>, _>(bits)?;
        optional_laws::<OptionRangedI8<{ i8::MIN }, 5>, _>(bits)?;
        optional_laws::<OptionRangedU16<0, 300>, _>(bits)?;
        optional_laws::<OptionRangedI32<{ i32::MIN }, 0>, _>(bits)?;
        optional_laws::<OwnerId, _>(bits)?;
        optional_laws::<OptionRangedUsize<0, 5>, _>(bits)?;
        optional_laws::<OptionRangedIsize<{ isize::MIN }, -1>, _>(bits)?;
        #[cfg(wide)]
        {
            ranged_integer_laws::<Partial, RangedU128<1, { u128::MAX }>>(bits, other)?;
            ranged_integer_laws::<Partial, RangedI128<-5, 5>>(bits, other)?;
            optional_laws::<OptionRangedU128<0, 5>, _>(bits)?;
            optional_laws::<OptionRangedI128<-5, 5>, _>(bits)?;
        }
        Ok(())
    }

    /// The integer deranged 0.5 stores `value` as.
    #[expect(unsafe_code, reason = "reads deranged's layout, which no accessor gives")]
    fn stored_u8<const MIN: u8, const MAX: u8>(value: OptionRangedU8<MIN, MAX>) -> u8 {
        // SAFETY: deranged 0.5 declares `OptionRangedU8` `repr(transparent)` over a `u8`, so its
        // one byte is initialized, and any byte is a `u8`.
        unsafe { transmute_copy::<OptionRangedU8<MIN, MAX>, u8>(&value) }
    }

    /// The integer deranged 0.5 stores `value` as.
    #[expect(unsafe_code, reason = "reads deranged's layout, which no accessor gives")]
    fn stored_i8<const MIN: i8, const MAX: i8>(value: OptionRangedI8<MIN, MAX>) -> i8 {
        // SAFETY: deranged 0.5 declares `OptionRangedI8` `repr(transparent)` over an `i8`, so its
        // one byte is initialized, and any byte is an `i8`.
        unsafe { transmute_copy::<OptionRangedI8<MIN, MAX>, i8>(&value) }
    }

    /// Checks that each of atomix's ranged integers converts to deranged's of the same bounds and
    /// back, at both bounds, keeping the integer.
    macro_rules! both_ways {
        ($($name:ident($min:expr, $max:expr)),+ $(,)?) => {$({
            type Ours = atomix_core::$name<{ $min }, { $max }>;
            type Theirs = deranged::$name<{ $min }, { $max }>;
            for ours in [Ours::MIN, Ours::MAX] {
                let theirs = Theirs::from(ours);
                assert_eq!(theirs.get(), ours.get(), "{ours:?} is deranged's {theirs:?}");
                assert_eq!(Ours::from(theirs), ours, "and back");
            }
        })+};
    }

    #[test]
    fn each_is_stored_as_its_integer() {
        const {
            repr_and_validity_are::<RangedU8<3, 10>, u8, Partial>();
            repr_and_validity_are::<PriceMove, i8, Partial>();
            repr_and_validity_are::<OptionRangedU8<3, 10>, u8, ZeroValid>();
            repr_and_validity_are::<OptionRangedI8<-5, 5>, i8, Partial>();
        }
        assert_eq!(RangedU8::<3, 10>::REPRS, ReprRange::new(3, 10), "3 to 10");
        assert_eq!(PriceMove::REPRS, ReprRange::from_signed(-5, 5), "-5 to 5");
    }

    #[test]
    fn every_byte_decodes_exactly_when_it_is_a_value() {
        decodes_exactly_its_values::<RangedU8<3, 10>, _>(0..=u8::MAX, 8);
        decodes_exactly_its_values::<RangedU8<0, 255>, _>(0..=u8::MAX, 256);
        decodes_exactly_its_values::<PriceMove, _>(i8::MIN..=i8::MAX, 11);
        decodes_exactly_its_values::<OptionRangedU8<3, 10>, _>(0..=u8::MAX, 9);
        decodes_exactly_its_values::<OptionRangedU8<0, 10>, _>(0..=u8::MAX, 12);
        decodes_exactly_its_values::<OptionRangedI8<-5, 5>, _>(i8::MIN..=i8::MAX, 12);
        decodes_exactly_its_values::<OptionRangedI8<-128, 5>, _>(i8::MIN..=i8::MAX, 135);
    }

    #[test]
    fn values_round_trip_at_the_edges() {
        type Id = RangedU64<1, { u64::MAX }>;
        type Negative = RangedI64<{ i64::MIN }, -1>;
        for value in [Id::MIN, Id::MAX] {
            assert_eq!(Id::from_repr(value.to_repr()), Some(value), "{value:?} round-trips");
        }
        for value in [Negative::MIN, Negative::MAX] {
            assert_eq!(Negative::from_repr(value.to_repr()), Some(value), "{value:?} round-trips");
        }
        for value in [OwnerId::None, OwnerId::Some(RangedU64::MAX)] {
            assert_eq!(OwnerId::from_repr(value.to_repr()), Some(value), "{value:?} round-trips");
        }
        assert_eq!(Id::from_repr(0), None, "0 is below 1");
        assert_eq!(Negative::from_repr(0), None, "and above -1");
    }

    #[test]
    fn every_edge_obeys_the_laws() {
        for bits in edges(&bounds()) {
            assert_holds(every_law(bits, bits.wrapping_sub(1)));
        }
    }

    #[test]
    fn none_takes_the_integer_deranged_keeps_for_it() {
        // The lowest integer, or the highest where `MIN` is the lowest.
        assert_eq!(OptionRangedU8::<3, 10>::None.to_repr(), 0, "zero, below 3");
        assert_eq!(OptionRangedU8::<0, 10>::None.to_repr(), u8::MAX, "255, as 0 is a value");
        assert_eq!(OptionRangedI8::<-5, 5>::None.to_repr(), i8::MIN, "-128");
        assert_eq!(OptionRangedI8::<-128, 5>::None.to_repr(), i8::MAX, "127, as -128 is a value");
        assert_eq!(stored_u8(OptionRangedU8::<3, 10>::None), 0, "deranged stores it at 0");
        assert_eq!(stored_u8(OptionRangedU8::<0, 10>::None), u8::MAX, "and at 255");
        assert_eq!(stored_i8(OptionRangedI8::<-5, 5>::None), i8::MIN, "and at -128");
        assert_eq!(stored_i8(OptionRangedI8::<-128, 5>::None), i8::MAX, "and at 127");
    }

    #[test]
    fn an_optional_range_grows_to_none_by_the_fewer_reprs() {
        assert_eq!(OptionRangedU8::<3, 10>::REPRS, ReprRange::new(0, 10), "0 to 10");
        assert_eq!(OptionRangedU8::<0, 10>::REPRS, ReprRange::from_signed(-1, 10), "255 to 10");
        assert_eq!(OptionRangedI8::<-5, 5>::REPRS, ReprRange::from_signed(-128, 5), "-128 to 5");
        assert_eq!(field_width(OptionRangedU8::<3, 10>::REPRS), 4, "four bits");
        assert_eq!(field_width(OptionRangedU8::<0, 10>::REPRS), 5, "five, sign-extended");
        assert_eq!(field_width(OptionRangedI8::<-5, 5>::REPRS), 8, "the whole byte");
    }

    #[test]
    fn atomix_option_puts_none_beside_the_range() {
        // atomix's rule: the side beside the range that keeps the field narrower.
        assert_eq!(None::<RangedU8<3, 10>>.to_repr(), 2, "2, below 3");
        assert_eq!(None::<RangedU8<0, 10>>.to_repr(), 11, "11, above a range from 0");
        assert_eq!(None::<PriceMove>.to_repr(), -6, "-6, below -5");
        assert_eq!(field_width(Option::<PriceMove>::REPRS), 4, "four bits, not eight");
        // And of an optional ranged integer, beside its `None`.
        assert_eq!(None::<OptionRangedU8<3, 10>>.to_repr(), 11, "11, as 0 is `None`");
        none_laws!(RangedU8<3, 10>, RangedU8<0, 10>, PriceMove, OptionRangedU8<3, 10>);
    }

    #[test]
    fn the_repr_order_is_not_the_value_order_where_none_is_high() {
        type FromZero = OptionRangedU8<0, 10>;
        type FromOne = OptionRangedU8<1, 10>;
        let (none, three) = (FromZero::None, FromZero::Some(RangedU8::new_static::<3>()));
        assert_eq!(none.cmp(&three), Ordering::Less, "deranged orders `None` lowest");
        assert_eq!(
            none.to_repr().cmp(&three.to_repr()),
            Ordering::Greater,
            "but its repr, 255, is the highest"
        );
        let (none, three) = (FromOne::None, FromOne::Some(RangedU8::new_static::<3>()));
        assert_eq!(
            (none.cmp(&three), none.to_repr().cmp(&three.to_repr())),
            (Ordering::Less, Ordering::Less),
            "where `None` is 0, they agree"
        );
    }

    #[test]
    fn each_of_the_twelve_converts_both_ways_keeping_its_integer() {
        both_ways!(RangedU8(3, 10), RangedU16(0, 300), RangedU32(1, u32::MAX));
        both_ways!(RangedU64(0, 0), RangedU128(1, u128::MAX), RangedUsize(0, usize::MAX));
        both_ways!(RangedI8(-5, 5), RangedI16(i16::MIN, -1), RangedI32(-300, 300));
        both_ways!(RangedI64(0, i64::MAX), RangedI128(i128::MIN, i128::MAX));
        both_ways!(RangedIsize(-1, 1));
    }

    #[test]
    fn statics_hold_the_values_stored() {
        assert_eq!(DEPTH.load(Acquire), Depth::MIN, "built in const");
        DEPTH.store(Depth::MAX, Release);
        assert_eq!(DEPTH.load(Acquire), Depth::MAX, "the largest");
        assert_eq!(OWNER.load(Acquire), OwnerId::None, "`None` built in const");
        assert_eq!(OWNER.load(Acquire).to_repr(), 0, "at zero");
        OWNER.store(OwnerId::Some(RangedU64::MAX), Release);
        assert_eq!(OWNER.load(Acquire).get_primitive(), Some(u64::MAX), "the largest id");
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn max_and_min_follow_the_order_through_zero() {
        let atomic = Atomic::new(PriceMove::new_static::<-1>());
        atomic.fetch_max(PriceMove::new_static::<2>(), Relaxed);
        assert_eq!(atomic.load(Relaxed).get(), 2, "2 is above -1");
        atomic.fetch_min(PriceMove::MIN, Relaxed);
        assert_eq!(atomic.load(Relaxed).get(), -5, "-5 is below 2");
    }

    proptest! {
        #[test]
        fn ranged_integers_obey_the_laws(
            bits in edge_or_random_bits(&bounds()),
            other in edge_or_random_bits(&bounds()),
        ) {
            every_law(bits, other)?;
        }
    }
}
