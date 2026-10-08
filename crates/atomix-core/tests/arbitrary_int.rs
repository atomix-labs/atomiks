//! arbitrary-int's integers in an atomic: each stored as its base integer, every repr of an 8-bit
//! base decoding exactly when it is a value; the `Atom` laws at the edges of each width, and on
//! random bits; `None` beside the range; statics; `fetch_max` and `fetch_min`, through zero for
//! the signed, on aarch64; and, in a field of a packed struct, a `u4`'s bitwise operations, the
//! add of a `u61`, an `i12` and an `Int<i8, 8>` that fill their word, and a signed field's not.
//!
//! The dev-dependency turns on arbitrary-int's `hint`, so these run against the `value` of each
//! base it swaps in, whose promise that the value fits Miri checks; the library builds without it,
//! as in `check-rust-doc` and a user's build.

#![cfg(feature = "arbitrary-int")]
#![feature(const_trait_impl)]

// Under loom the impls stay on, since none reads an atomic's memory as bytes; `check-loom` builds
// this.
#[cfg(loom)]
const _: () = {
    use arbitrary_int::{i20, u20};
    use atomix_core::{AtomOrd, FieldAdd, FieldBitwise};

    /// Compiles only where `T`'s reprs order as its values.
    const fn orders_as_its_reprs<T: AtomOrd>() {}
    /// Compiles only where a field of `T` has the bitwise operations.
    const fn combines_in_a_field<T: FieldBitwise>() {}
    /// Compiles only where a field of `T` that ends at the top adds in place.
    const fn adds_in_a_field<T: FieldAdd>() {}

    orders_as_its_reprs::<u20>();
    orders_as_its_reprs::<i20>();
    combines_in_a_field::<u20>();
    combines_in_a_field::<i20>();
    adds_in_a_field::<u20>();
    adds_in_a_field::<i20>();
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
        Int, UInt, i1, i3, i5, i7, i12, i15, i20, i31, i33, i63, u1, u3, u4, u5, u7, u12, u15, u20,
        u31, u33, u61, u63,
    };
    #[cfg(wide)]
    use arbitrary_int::{i65, i127, u65, u127};
    #[cfg(aarch64_code)]
    use atomix_core::ordering::Relaxed;
    use atomix_core::ordering::{AcqRel, Acquire, Release};
    use atomix_core::validity::ZeroValid;
    use atomix_core::{Atom, Atomic, Field, ReprRange};
    use proptest::proptest;
    use proptest::test_runner::TestCaseError;

    use crate::testing::atom::{decodes_exactly_its_values, field_width, repr_and_validity_are};
    use crate::testing::field::{bits, canonical};
    use crate::testing::law::{
        assert_holds, edge_or_random_bits, edges, none_laws, ranged_integer_laws,
    };
    use crate::testing::packed::packed;

    /// The widths the laws run on beside each base's own: one bit, a few, one short of a base, and
    /// one past the base below.
    const WIDTHS: [u32; 11] = [1, 5, 7, 12, 15, 20, 31, 33, 63, 65, 127];

    /// A 20-bit sequence number, in a static: the impl is `const`.
    static SEQ: Atomic<u20> = Atomic::new(u20::new(5));
    /// A 20-bit signed offset, or `None` before the first.
    static OFFSET: Atomic<Option<i20>> = Atomic::new(None);

    packed! {
        /// A task: three bits of its state, then a count of its references, filling the word.
        struct Task in u64, projected as TaskFields {
            0 => queued: bool,
            1 => running: bool,
            2 => done: bool,
            3 => references: u61,
        }
    }

    packed! {
        /// A step: four bits of flags, then a signed delta, filling the word.
        struct Step in u16, projected as StepFields {
            0 => flags: u4,
            1 => delta: i12,
        }
    }

    packed! {
        /// A byte of flags, then a signed level as wide as its base, which a field stores as its
        /// bits are, unextended.
        struct Gauge in u16, projected as GaugeFields {
            0 => flags: u8,
            1 => level: Int<i8, 8>,
        }
    }

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

    #[cfg(aarch64_code)]
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

    #[test]
    fn a_u61_counter_at_the_top_wraps_at_its_largest_and_leaves_the_bits_below() {
        assert_eq!(bits::<Field<Task, 3, u61>>(), (3, 61), "the count's 61 bits, at the top");
        let first = Task { queued: true, running: false, done: true, references: u61::MAX };
        let task = Atomic::new(first);
        let TaskFields { queued, running, done, references } = task.fields();
        assert_eq!(references.fetch_add(1, AcqRel), first, "the task before");
        let wrapped = Task { references: u61::new(0), ..first };
        assert_eq!(canonical(&task), wrapped, "wrapped to zero, the bits below as they were");
        queued.clear(Release);
        running.set(Release);
        done.clear(Release);
        let started = Task { queued: false, running: true, done: false, ..wrapped };
        assert_eq!(references.fetch_sub(1, AcqRel), started, "zero before, the bits as changed");
        let back = Task { references: u61::MAX, ..started };
        assert_eq!(canonical(&task), back, "back to the largest, borrowing nothing from below");
    }

    #[test]
    fn a_delta_wider_than_the_field_adds_modulo_its_width() {
        let first = Task { queued: false, running: true, done: false, references: u61::MAX };
        let task = Atomic::new(first);
        task.fields().references.fetch_add(u64::MAX, Release);
        let less_one = Task { references: u61::MAX.wrapping_sub(u61::new(1)), ..first };
        assert_eq!(canonical(&task), less_one, "`u64::MAX` is 2^61 - 1 there, so one less");
    }

    #[test]
    fn a_u4_field_of_flags_combines_its_bits_alone() {
        assert_eq!(bits::<Field<Step, 0, u4>>(), (0, 4), "the flags' four bits");
        let first = Step { flags: u4::new(0b1010), delta: i12::new(-5) };
        let step = Atomic::new(first);
        let flags = step.fields().flags;
        flags.or(u4::new(0b0101), Release);
        assert_eq!(flags.load(Acquire), u4::new(0b1111), "or");
        flags.and(u4::new(0b0110), Release);
        assert_eq!(flags.load(Acquire), u4::new(0b0110), "and");
        flags.xor(u4::new(0b1100), Release);
        assert_eq!(flags.load(Acquire), u4::new(0b1010), "xor");
        flags.not(Release);
        let inverted = Step { flags: u4::new(0b0101), ..first };
        assert_eq!(canonical(&step), inverted, "not, the negative delta as it was");
    }

    #[test]
    fn a_signed_field_at_the_top_subtracts_across_zero_and_wraps_past_its_smallest() {
        let first = Step { flags: u4::new(0b1001), delta: i12::new(1) };
        let step = Atomic::new(first);
        let delta = step.fields().delta;
        assert_eq!(delta.fetch_sub(3, AcqRel), first, "the container before");
        let below = Step { delta: i12::new(-2), ..first };
        assert_eq!(canonical(&step), below, "below zero, the flags as they were");
        delta.fetch_add(2, AcqRel);
        assert_eq!(canonical(&step), Step { delta: i12::new(0), ..first }, "and back to zero");
        delta.update(AcqRel, Acquire, |_| i12::MIN);
        delta.fetch_sub(1, AcqRel);
        assert_eq!(canonical(&step), Step { delta: i12::MAX, ..first }, "wrapped to the largest");
    }

    #[test]
    fn a_signed_field_as_wide_as_its_base_adds_across_zero_and_wraps_past_its_largest() {
        assert_eq!(
            bits::<Field<Gauge, 1, Int<i8, 8>>>(),
            (8, 8),
            "the level's eight bits, at the top"
        );
        let first = Gauge { flags: 0xA5, level: Int::<i8, 8>::new(-1) };
        let gauge = Atomic::new(first);
        let GaugeFields { flags, level } = gauge.fields();
        level.fetch_add(2, AcqRel);
        let above = Gauge { level: Int::<i8, 8>::new(1), ..first };
        assert_eq!(canonical(&gauge), above, "above zero, the flags as they were");
        flags.and(0x0F, Release);
        level.update(AcqRel, Acquire, |_| Int::<i8, 8>::MAX);
        level.fetch_add(1, AcqRel);
        let wrapped = Gauge { flags: 0x05, level: Int::<i8, 8>::MIN };
        assert_eq!(canonical(&gauge), wrapped, "wrapped to the smallest, the flags as changed");
    }

    #[test]
    fn a_signed_field_inverts_as_its_own_not() {
        let first = Step { flags: u4::new(0b1001), delta: i12::MAX };
        let step = Atomic::new(first);
        step.fields().delta.not(Release);
        let inverted = Step { delta: i12::MIN, ..first };
        assert_eq!(canonical(&step), inverted, "sign-extended: `!i12::MAX` is `i12::MIN`");
        let first = Gauge { flags: 0xA5, level: Int::<i8, 8>::new(5) };
        let gauge = Atomic::new(first);
        gauge.fields().level.not(Release);
        let inverted = Gauge { level: Int::<i8, 8>::new(-6), ..first };
        assert_eq!(canonical(&gauge), inverted, "as wide as its base: `!5` is -6");
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
