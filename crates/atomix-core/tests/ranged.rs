//! The ranged integers as a user sees them, for widths signed and unsigned: each constructor and
//! operation at the edges of the range and of the integer, the conversions and the text, with each
//! refusal and its message, and comparing, hashing, stepping and formatting as the integer does;
//! the niche an `Option` takes; and in an atomic, the integer itself as the repr, `None` beside the
//! range, statics that need no feature gate, and the maximum and minimum through zero, on aarch64.

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    use core::error::Error;
    use core::hash::BuildHasher;
    use core::num::{IntErrorKind, ParseIntError};
    use core::str::FromStr;
    use std::hash::RandomState;

    #[cfg(target_arch = "aarch64")]
    use atomix_core::ordering::Relaxed;
    use atomix_core::ordering::{AcqRel, Acquire, Release};
    use atomix_core::{
        Atom, Atomic, ParseRangeError, RangeError, RangedI8, RangedI16, RangedI64, RangedI128,
        RangedIsize, RangedU8, RangedU64, RangedU128, RangedUsize, ReprRange,
    };

    /// How many levels of an order book a feed sends.
    type Depth = RangedU8<1, 10>;
    /// How far a price moves in one update, in ticks.
    type PriceMove = RangedI64<-5, 5>;

    /// How many levels of the book a feed keeps, from 3 to 100.
    static LEVELS: Atomic<RangedU64<3, 100>> = Atomic::new(RangedU64::MIN);
    /// The last move of a price, -5 to 5 ticks, or `None` before the first.
    static LAST_MOVE: Atomic<Option<RangedI8<-5, 5>>> = Atomic::new(None);

    /// The error core's parse gives for `text` as a `T`.
    fn parse_error<T: FromStr<Err = ParseIntError>>(text: &str) -> ParseIntError {
        text.parse::<T>().err().expect("the text is no integer of the type")
    }

    #[test]
    fn new_takes_each_integer_in_the_range() {
        assert_eq!(Depth::new(1), Some(Depth::MIN), "1 is `MIN`");
        assert_eq!(Depth::new(10), Some(Depth::MAX), "10 is `MAX`");
        assert_eq!(Depth::new(4).map(Depth::get), Some(4), "and 4 lies between");
        assert_eq!(PriceMove::new(-5), Some(PriceMove::MIN), "-5 is `MIN` of a range through zero");
        assert_eq!(PriceMove::new(0).map(PriceMove::get), Some(0), "and 0 lies inside it");
        assert_eq!(RangedU8::<7, 7>::new(7).map(RangedU8::get), Some(7), "a range of one");
    }

    #[test]
    fn new_refuses_each_integer_outside_the_range() {
        assert_eq!(Depth::new(0), None, "0, one below");
        assert_eq!(Depth::new(11), None, "11, one above");
        assert_eq!(Depth::new(u8::MAX), None, "and the integer's largest");
        assert_eq!(PriceMove::new(-6), None, "-6, one below a range through zero");
        assert_eq!(PriceMove::new(i64::MIN), None, "and the integer's smallest");
        assert_eq!(RangedU8::<7, 7>::new(8), None, "and 8 beside a range of one");
    }

    #[test]
    fn a_full_range_takes_every_integer() {
        assert_eq!(RangedU8::<0, 255>::new(0).map(RangedU8::get), Some(0), "the smallest `u8`");
        assert_eq!(RangedI8::<-128, 127>::new(-128).map(RangedI8::get), Some(-128), "an `i8`'s");
        let largest = RangedU128::<0>::new(u128::MAX).map(RangedU128::get);
        assert_eq!(largest, Some(u128::MAX), "and the largest `u128`");
        let smallest = RangedI128::<{ i128::MIN }>::new(i128::MIN).map(RangedI128::get);
        assert_eq!(smallest, Some(i128::MIN), "and the smallest `i128`");
    }

    #[test]
    fn max_defaults_to_the_integers_largest() {
        assert_eq!(RangedU64::<3>::MAX.get(), u64::MAX, "a `u64`'s");
        assert_eq!(RangedUsize::<1>::MAX.get(), usize::MAX, "a `usize`'s");
        assert_eq!(RangedIsize::<-1>::MAX.get(), isize::MAX, "and an `isize`'s, from -1");
    }

    #[test]
    fn new_unchecked_in_the_range_is_the_value_new_gives() {
        // SAFETY: 7 lies from 1 to 10.
        #[expect(unsafe_code, reason = "the unchecked constructor, given an integer in its range")]
        let depth = unsafe { Depth::new_unchecked(7) };
        assert_eq!(Some(depth), Depth::new(7), "the same value");
    }

    #[test]
    fn new_saturating_moves_an_integer_to_the_nearest_bound() {
        assert_eq!(Depth::new_saturating(0), Depth::MIN, "0, up to 1");
        assert_eq!(Depth::new_saturating(u8::MAX), Depth::MAX, "the integer's largest, down to 10");
        assert_eq!(Depth::new_saturating(6).get(), 6, "and 6, kept");
        assert_eq!(
            PriceMove::new_saturating(i64::MIN),
            PriceMove::MIN,
            "the integer's smallest, up to -5"
        );
    }

    #[test]
    fn checked_arithmetic_refuses_a_result_outside_the_range() {
        assert_eq!(Depth::MIN.checked_add(9), Some(Depth::MAX), "1 + 9 is 10");
        assert_eq!(Depth::MIN.checked_add(10), None, "1 + 10 is past 10");
        assert_eq!(Depth::MAX.checked_add(u8::MAX), None, "as is a sum past the integer");
        assert_eq!(Depth::MAX.checked_sub(9), Some(Depth::MIN), "10 - 9 is 1");
        assert_eq!(Depth::MIN.checked_sub(1), None, "1 - 1 is below 1");
        assert_eq!(Depth::MIN.checked_sub(2), None, "as is a difference below the integer");
        assert_eq!(Depth::MIN.checked_mul(10), Some(Depth::MAX), "1 * 10 is 10");
        assert_eq!(Depth::MAX.checked_mul(u8::MAX), None, "and a product past the integer");
        assert_eq!(PriceMove::MIN.checked_mul(-1), Some(PriceMove::MAX), "-5 * -1 is 5");
        assert_eq!(
            PriceMove::MAX.checked_sub(i64::MIN),
            None,
            "5 - `i64::MIN` is past the integer"
        );
    }

    #[test]
    fn saturating_arithmetic_stops_at_the_bounds() {
        assert_eq!(Depth::MIN.saturating_add(4).get(), 5, "1 + 4 is 5");
        assert_eq!(Depth::MIN.saturating_add(u8::MAX), Depth::MAX, "a sum past the integer is 10");
        assert_eq!(Depth::MAX.saturating_sub(u8::MAX), Depth::MIN, "a difference below 0 is 1");
        assert_eq!(Depth::MAX.saturating_mul(2), Depth::MAX, "10 * 2 is 10");
        assert_eq!(
            PriceMove::MIN.saturating_add(i64::MIN),
            PriceMove::MIN,
            "a sum below `i64` is -5"
        );
        assert_eq!(
            PriceMove::MIN.saturating_mul(i64::MAX),
            PriceMove::MIN,
            "-5 * `i64::MAX` is -5"
        );
        assert_eq!(
            PriceMove::MIN.saturating_sub(i64::MIN),
            PriceMove::MAX,
            "and -5 - `i64::MIN` is 5"
        );
    }

    #[test]
    fn min_max_and_clamp_keep_the_range() {
        let (down, up) =
            (PriceMove::new(-3).expect("-3 is a move"), PriceMove::new(4).expect("4 is one"));
        assert_eq!(down.min(up), down, "the smaller");
        assert_eq!(down.max(up), up, "the larger");
        let flat = PriceMove::new(0).expect("0 is a move");
        assert_eq!(PriceMove::MIN.clamp(flat, up), flat, "-5 clamped to 0 through 4");
        assert_eq!(PriceMove::MAX.clamp(down, up), up, "and 5 to -3 through 4");
    }

    #[test]
    fn values_compare_and_hash_as_their_integers() {
        let moves: Vec<PriceMove> = [3, -5, 0].into_iter().filter_map(PriceMove::new).collect();
        let mut sorted = moves.clone();
        sorted.sort_unstable();
        assert_eq!(
            sorted.iter().map(|price_move| price_move.get()).collect::<Vec<_>>(),
            [-5, 0, 3],
            "in order"
        );
        let hasher = RandomState::new();
        for price_move in moves {
            assert_eq!(
                hasher.hash_one(price_move),
                hasher.hash_one(price_move.get()),
                "the hash of {price_move}"
            );
        }
    }

    #[test]
    fn a_range_steps_through_each_value() {
        let integers = |depths: Vec<Depth>| depths.into_iter().map(Depth::get).collect::<Vec<_>>();
        assert_eq!(
            integers((Depth::MIN..=Depth::MAX).collect()),
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            "up"
        );
        assert_eq!(
            integers((Depth::MIN..Depth::MAX).rev().collect()),
            [9, 8, 7, 6, 5, 4, 3, 2, 1],
            "down"
        );
        assert_eq!(integers((Depth::MIN..=Depth::MAX).step_by(4).collect()), [1, 5, 9], "by 4");
        assert_eq!((PriceMove::MIN..=PriceMove::MAX).count(), 11, "and -5 to 5 is 11 moves");
    }

    #[test]
    fn a_step_past_the_end_ends_the_range_or_saturates() {
        assert_eq!((Depth::MIN..=Depth::MAX).nth(9), Some(Depth::MAX), "the tenth is 10");
        assert_eq!((Depth::MIN..=Depth::MAX).nth(10), None, "with none after it");
        assert_eq!((Depth::MIN..=Depth::MAX).rev().nth(10), None, "nor before 1");
        assert_eq!((Depth::MIN..).take(10).count(), 10, "an open range steps to 10");
        assert_eq!((Depth::MIN..).nth(10), Some(Depth::MAX), "and past it, saturating at 10");
        let full = RangedU8::<0, 255>::MIN..=RangedU8::MAX;
        assert_eq!(full.clone().count(), 256, "a full range steps to its end");
        assert_eq!(full.last().map(RangedU8::get), Some(255), "the integer's largest included");
    }

    #[test]
    fn try_from_takes_the_range_and_hands_back_a_refused_integer() {
        assert_eq!(Depth::try_from(10), Ok(Depth::MAX), "10 is `MAX`");
        let refused = Depth::try_from(11).expect_err("11 is past 10");
        assert_eq!(refused.integer(), 11, "the integer refused");
        assert_eq!(refused.bounds(), 1..=10, "and the range");
        assert_eq!(u8::from(Depth::MAX), 10, "and the integer back");
    }

    #[test]
    fn a_range_error_names_the_integer_and_the_range() {
        let message = |error: RangeError<i64>| error.to_string();
        assert_eq!(
            message(PriceMove::try_from(6).expect_err("6 is past 5")),
            "range error: 6 is not in -5..=5",
            "above"
        );
        assert_eq!(
            message(PriceMove::try_from(-6).expect_err("-6 is below -5")),
            "range error: -6 is not in -5..=5",
            "below"
        );
    }

    #[test]
    fn display_and_from_str_agree() {
        for price_move in PriceMove::MIN..=PriceMove::MAX {
            assert_eq!(price_move.to_string().parse(), Ok(price_move), "{price_move} round-trips");
        }
        assert_eq!(
            "+5".parse(),
            Ok(PriceMove::MAX),
            "and a leading `+` parses, as for the integer"
        );
    }

    #[test]
    fn from_str_refuses_a_text_that_is_no_integer_of_the_type() {
        for text in ["", "-", "x", "1.5", "256"] {
            let refused = text.parse::<Depth>();
            assert_eq!(refused, Err(ParseRangeError::Integer(parse_error::<u8>(text))), "{text:?}");
        }
        let below = "-129".parse::<RangedI8<-5, 5>>();
        let Err(ParseRangeError::Integer(cause)) = &below else {
            panic!("-129 is no `i8`, not {below:?}");
        };
        assert_eq!(cause.kind(), &IntErrorKind::NegOverflow, "-129 is below every `i8`");
        let refused = "256".parse::<Depth>().expect_err("256 is no `u8`");
        assert_eq!(refused.to_string(), "parse range error: the text is no `u8`", "the message");
        let cause = refused.source().map(ToString::to_string);
        assert_eq!(
            cause.as_deref(),
            Some("number too large to fit in target type"),
            "and core's cause"
        );
    }

    #[test]
    fn from_str_refuses_an_integer_outside_the_range() {
        let parsed = "-6".parse::<PriceMove>();
        let Err(ParseRangeError::Range(refused)) = parsed else {
            panic!("-6 is an `i64` below -5, not {parsed:?}");
        };
        assert_eq!(
            (refused.integer(), refused.bounds()),
            (-6, -5..=5),
            "the integer and the range"
        );
        let refused = "11".parse::<Depth>().expect_err("11 is past 10");
        assert_eq!(
            refused.to_string(),
            "parse range error: the integer is outside the range",
            "the message"
        );
        let cause = refused.source().map(ToString::to_string);
        assert_eq!(
            cause.as_deref(),
            Some("range error: 11 is not in 1..=10"),
            "and the range's cause"
        );
    }

    #[test]
    fn values_format_as_their_integers() {
        let depth = Depth::new(10).expect("10 is a depth");
        assert_eq!(format!("{depth:?} {depth} {depth:>3}"), "10 10  10", "in decimal, padded too");
        assert_eq!(
            format!("{depth:b} {depth:o} {depth:x} {depth:#X}"),
            "1010 12 a 0xA",
            "and in each base"
        );
        assert_eq!(format!("{:?}", Some(PriceMove::MIN)), "Some(-5)", "and in an `Option`");
    }

    #[test]
    fn an_option_keeps_its_none_outside_the_range() {
        assert_eq!(size_of::<Option<RangedU64<3>>>(), 8, "below 3");
        assert_eq!(size_of::<Option<RangedI8<-5, 5>>>(), 1, "past either end");
        assert_eq!(size_of::<Option<RangedU128<1>>>(), 16, "and at 0 in 128 bits");
    }

    #[test]
    fn an_option_of_a_full_range_takes_a_byte_more() {
        assert_eq!(size_of::<RangedU8<0, 255>>(), 1, "the value alone");
        assert_eq!(size_of::<Option<RangedU8<0, 255>>>(), 2, "no integer is left for `None`");
    }

    #[test]
    fn an_atomic_stores_the_integer_itself() {
        assert_eq!(PriceMove::MIN.to_repr(), -5, "-5, not its offset from `MIN`");
        assert_eq!(
            <RangedU64<3, 100>>::REPRS,
            ReprRange::new(3, 100),
            "so the reprs are the range"
        );
        assert_eq!(<RangedU64<3>>::REPRS, ReprRange::new(3, u128::from(u64::MAX)), "to `u64::MAX`");
        assert_eq!(<RangedI8<-5, 5>>::REPRS, ReprRange::from_signed(-5, 5), "through zero");
        assert_eq!(<RangedI16<-300, -10>>::REPRS, ReprRange::from_signed(-300, -10), "below it");
        assert_eq!(<RangedIsize<-1, 1>>::REPRS, ReprRange::from_signed(-1, 1), "in any width");
        assert_eq!(<RangedU8<0, 255>>::REPRS, ReprRange::FULL, "every repr, for a full range");
        assert_eq!(<RangedI8<{ i8::MIN }>>::REPRS, ReprRange::FULL, "signed too");
    }

    #[test]
    fn none_takes_the_repr_beside_the_range() {
        assert_eq!(None::<RangedU64<3, 100>>.to_repr(), 2, "2, below 3");
        assert_eq!(
            None::<RangedU64<3>>.to_repr(),
            0,
            "0, past `u64::MAX`, beside the range, as `ReprRange` picks it"
        );
        assert_eq!(
            None::<RangedI8<-5, 5>>.to_repr(),
            -6,
            "-6, 0xFA, beside the range, as `ReprRange` picks it"
        );
    }

    #[test]
    fn statics_hold_a_ranged_integer_and_an_option_of_one() {
        LEVELS.store(RangedU64::MAX, Release);
        assert_eq!(LEVELS.load(Acquire), RangedU64::MAX, "100 levels");
        assert_eq!(LAST_MOVE.load(Acquire), None, "no move yet");
        LAST_MOVE.store(Some(RangedI8::MIN), Release);
        assert_eq!(LAST_MOVE.swap(Some(RangedI8::MAX), AcqRel), Some(RangedI8::MIN), "-5");
        assert_eq!(LAST_MOVE.load(Acquire), Some(RangedI8::MAX), "then 5");
    }

    // x86_64 has no atomic maximum or minimum, so `fetch_max` and `fetch_min` are aarch64's alone.
    #[cfg(target_arch = "aarch64")]
    #[test]
    fn max_and_min_order_as_the_integers_through_zero() {
        let price_move = Atomic::new(PriceMove::new(-3).expect("-3 is a move"));
        price_move.fetch_max(PriceMove::new(2).expect("2 is one"), Relaxed);
        assert_eq!(price_move.load(Relaxed).get(), 2, "2, above -3");
        assert_eq!(price_move.fetch_min(PriceMove::MIN, Relaxed).get(), 2, "the value before");
        assert_eq!(price_move.load(Relaxed), PriceMove::MIN, "then -5, below 2");
    }
}
