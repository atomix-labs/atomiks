//! Each of the twelve ranged integers at the edges of its range and of its integer, each refusal
//! compared whole; and in a debug build, `new_unchecked`'s check of its contract.

use core::str::FromStr;

use super::{
    RangedI8, RangedI16, RangedI32, RangedI64, RangedI128, RangedIsize, RangedU8, RangedU16,
    RangedU32, RangedU64, RangedU128, RangedUsize,
};
use crate::errors::{ParseRangeError, RangeError};

#[test]
fn each_ranged_integer_takes_its_range_and_refuses_the_integers_past_it() {
    macro_rules! check {
        ($name:ident($int:ident)) => {{
            type Depth = $name<1, 10>;
            let name = stringify!($name);
            assert_eq!(Depth::new(1).map(Depth::get), Some(1), "{name}: 1 is in 1..=10");
            assert_eq!(Depth::new(10).map(Depth::get), Some(10), "{name}: and 10");
            assert_eq!(Depth::try_from(0), Err(RangeError::new(0, 1, 10)), "{name}: 0 is not");
            assert_eq!(Depth::try_from(11), Err(RangeError::new(11, 1, 10)), "{name}: nor 11");
            let largest = Depth::try_from($int::MAX);
            assert_eq!(largest, Err(RangeError::new($int::MAX, 1, 10)), "{name}: nor its largest");
        }};
    }
    each_ranged_integer!(check);
}

#[test]
fn each_ranged_integer_bounds_its_full_range_by_the_integers_own() {
    macro_rules! check {
        ($name:ident($int:ident)) => {{
            type Full = $name<{ $int::MIN }>;
            let name = stringify!($name);
            assert_eq!(Full::MIN.get(), $int::MIN, "{name}: `MIN` is the integer's smallest");
            assert_eq!(Full::MAX.get(), $int::MAX, "{name}: `MAX` defaults to its largest");
            assert_eq!(Full::new($int::MIN), Some(Full::MIN), "{name}: `new` takes the smallest");
            assert_eq!(Full::new($int::MAX), Some(Full::MAX), "{name}: the largest too");
        }};
    }
    each_ranged_integer!(check);
}

#[test]
fn each_ranged_integer_saturates_at_its_bounds() {
    macro_rules! check {
        ($name:ident($int:ident)) => {{
            type Depth = $name<1, 10>;
            let name = stringify!($name);
            assert_eq!(Depth::new_saturating($int::MIN), Depth::MIN, "{name}: up to 1");
            assert_eq!(Depth::new_saturating($int::MAX), Depth::MAX, "{name}: down to 10");
            assert_eq!(Depth::MAX.saturating_add($int::MAX), Depth::MAX, "{name}: a sum past it");
            assert_eq!(Depth::MIN.saturating_sub($int::MAX), Depth::MIN, "{name}: a difference");
            assert_eq!(Depth::MAX.saturating_mul($int::MAX), Depth::MAX, "{name}: a product");
            assert_eq!(Depth::MAX.checked_add($int::MAX), None, "{name}: none past the integer");
        }};
    }
    each_ranged_integer!(check);
}

#[test]
fn each_ranged_integer_parses_its_range_and_refuses_the_rest() {
    macro_rules! check {
        ($name:ident($int:ident)) => {{
            type Depth = $name<1, 10>;
            let name = stringify!($name);
            assert_eq!("10".parse(), Ok(Depth::MAX), "{name}: 10 is in 1..=10");
            let past = Err(ParseRangeError::Range(RangeError::new(11, 1, 10)));
            assert_eq!("11".parse::<Depth>(), past, "{name}: 11 is not");
            let malformed = $int::from_str("1x").map_err(ParseRangeError::Integer);
            assert_eq!("1x".parse::<Depth>().map(Depth::get), malformed, "{name}: nor `1x`");
        }};
    }
    each_ranged_integer!(check);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "an integer outside `MIN..=MAX`")]
fn new_unchecked_outside_the_range_panics_in_a_debug_build() {
    // SAFETY: the `cfg` is this crate's own, so `new_unchecked`'s `debug_assert!` is built, and it
    // panics on 0, below 1, before anything is built from it.
    #[expect(unsafe_code, reason = "the unchecked constructor, given 0, below its range")]
    let _depth = unsafe { RangedU8::<1, 10>::new_unchecked(0) };
}
