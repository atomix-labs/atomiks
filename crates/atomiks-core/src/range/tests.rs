//! The range math, exhaustively over widths of 1 to 4 and 8 bits, and each constructor over the
//! primitives, at the 128-bit boundary too.

extern crate alloc;

use alloc::vec::Vec;
use alloc::{format, vec};

use super::{ReprRange, Span, mask};

/// The span `start..=end` of `width`-bit numbers, whose bounds the caller keeps within the width.
const fn span(start: u128, end: u128, width: u32) -> Span {
    Span { start, end, width }
}

/// Every number of `span`, from its start, found by stepping round the circle of its width.
fn numbers(span: Span) -> Vec<u128> {
    let mut numbers = vec![span.start];
    let mut number = span.start;
    while number != span.end {
        number = number.wrapping_add(1) & span.largest();
        numbers.push(number);
    }
    numbers
}

/// Whether each number of `span`'s width, by index, lies in it, found by stepping as `numbers`.
fn membership(span: Span) -> Vec<bool> {
    let index = |number| usize::try_from(number).expect("a test's width indexes a `Vec`");
    let mut membership = vec![false; index(span.largest()).wrapping_add(1)];
    for number in numbers(span) {
        membership[index(number)] = true;
    }
    membership
}

/// Every check on every span of `width` bits.
fn check_every_span(width: u32) {
    let largest = mask(width);
    for start in 0..=largest {
        for end in 0..=largest {
            let span = span(start, end, width);
            let membership = membership(span);
            for (bits, &inside) in (0..=largest).zip(&membership) {
                assert_eq!(span.contains(bits), inside, "{span:?} holds {bits}");
            }
            assert!(!span.contains(largest.wrapping_add(1)), "{span:?}: nothing wider");
            assert_eq!(span.wraps(), start > end, "{span:?} wraps through zero");
            assert_eq!(span.is_full(), !membership.contains(&false), "{span:?} is full");
        }
    }
}

#[test]
fn every_span_of_1_to_4_bits_holds_its_numbers() {
    for width in 1..=4 {
        check_every_span(width);
    }
}

#[test]
#[cfg_attr(miri, ignore = "every 8-bit span, too many for Miri; the math holds no unsafe code")]
fn every_span_of_8_bits_holds_its_numbers() {
    check_every_span(8);
}

#[test]
fn full_and_nonzero_span_each_primitive() {
    assert_eq!(ReprRange::<bool>::FULL.span(), span(0, 1, 1), "`bool`, one bit");
    assert_eq!(ReprRange::<bool>::NONZERO.span(), span(1, 1, 1), "and its nonzero repr, 1");
    assert_eq!(ReprRange::<u8>::FULL.span(), span(0, 0xFF, 8), "`u8`");
    assert_eq!(ReprRange::<i8>::FULL.span(), span(0, 0xFF, 8), "`i8`, as unsigned bits");
    assert_eq!(ReprRange::<u16>::NONZERO.span(), span(1, 0xFFFF, 16), "`u16`'s nonzero");
    assert_eq!(ReprRange::<i32>::FULL.span(), span(0, 0xFFFF_FFFF, 32), "`i32`");
    assert_eq!(ReprRange::<u64>::FULL.span(), span(0, u128::from(u64::MAX), 64), "`u64`");
    assert_eq!(
        ReprRange::<*mut u8>::NONZERO.span(),
        ReprRange::<usize>::NONZERO.span(),
        "a pointer's, an address's"
    );
}

#[test]
fn new_holds_its_bounds_in_unsigned_order() {
    assert_eq!(ReprRange::<u8>::new(0, 1).span(), span(0, 1, 8), "from zero");
    assert_eq!(ReprRange::<u8>::new(200, 201).span(), span(200, 201, 8), "anywhere");
    assert_eq!(ReprRange::<u8>::new(7, 7).span(), span(7, 7, 8), "one repr");
    assert_eq!(ReprRange::<u8>::new(0, 0xFF), ReprRange::FULL, "every repr");
    assert_eq!(ReprRange::<i8>::new(0x80, 0xFF).span(), span(0x80, 0xFF, 8), "an `i8`'s negatives");
    assert_eq!(ReprRange::<u32>::new(0, 0x10_FFFF).span(), span(0, 0x10_FFFF, 32), "`char`'s");
    assert_eq!(ReprRange::<bool>::new(1, 1), ReprRange::NONZERO, "`true` alone");
}

#[test]
#[should_panic(
    expected = "`ReprRange::<u8>::new(5, 3)`: the start is above the end: list the bounds in \
                unsigned order, or call `ReprRange::from_signed` for a range through zero"
)]
fn new_refuses_its_bounds_swapped() {
    let built = ReprRange::<u8>::new(5, 3);
    panic!("built {built:?}, not refused");
}

#[test]
#[should_panic(
    expected = "`ReprRange::<u8>::new(0, 300)`: the end is above the repr's largest bits, 255"
)]
fn new_refuses_a_bound_past_its_primitive() {
    let built = ReprRange::<u8>::new(0, 300);
    panic!("built {built:?}, not refused");
}

#[test]
#[should_panic(expected = "`ReprRange::<bool>::new(0, 2)`: the end is above the repr's largest")]
fn new_refuses_a_bool_past_one() {
    let built = ReprRange::<bool>::new(0, 2);
    panic!("built {built:?}, not refused");
}

/// Every check on every `from_signed` range of `i8`: it holds exactly the bytes whose two's
/// complement lies from its start up to its end.
#[test]
#[cfg_attr(miri, ignore = "every 8-bit range, too many for Miri; the math holds no unsafe code")]
fn signed_holds_its_bounds_in_signed_order() {
    for start in i8::MIN..=i8::MAX {
        for end in start..=i8::MAX {
            let range = ReprRange::<i8>::from_signed(i128::from(start), i128::from(end));
            for value in i8::MIN..=i8::MAX {
                let bits = u128::from(value.cast_unsigned());
                assert_eq!(
                    range.contains(bits),
                    (start..=end).contains(&value),
                    "{start}..={end} holds {value}"
                );
            }
        }
    }
}

#[test]
fn signed_runs_through_zero_from_below_it() {
    assert_eq!(ReprRange::<i8>::from_signed(-1, 1).span(), span(0xFF, 0x01, 8), "-1 to 1 wraps");
    assert_eq!(
        ReprRange::<i8>::from_signed(-5, -1).span(),
        span(0xFB, 0xFF, 8),
        "negatives do not"
    );
    assert_eq!(ReprRange::<i8>::from_signed(-128, 127), ReprRange::FULL, "every `i8`");
    assert_eq!(
        ReprRange::<u8>::from_signed(1, 3),
        ReprRange::new(1, 3),
        "a `u8`'s as two's complement"
    );
    assert_eq!(ReprRange::<bool>::from_signed(-1, 0), ReprRange::FULL, "and a `bool`'s, -1 and 0");
    assert_eq!(
        ReprRange::<i64>::from_signed(-1, 1).span(),
        span(u128::from(u64::MAX), 1, 64),
        "-1 is every bit of the repr's width"
    );
}

#[test]
#[should_panic(
    expected = "`ReprRange::<i8>::from_signed(1, -1)`: the start is above the end: list the bounds in \
                signed order"
)]
fn signed_refuses_its_bounds_swapped() {
    let built = ReprRange::<i8>::from_signed(1, -1);
    panic!("built {built:?}, not refused");
}

#[test]
#[should_panic(
    expected = "`ReprRange::<i8>::from_signed(-129, 0)`: the start is below the repr's lowest signed \
                value, -128"
)]
fn signed_refuses_a_start_below_its_primitive() {
    let built = ReprRange::<i8>::from_signed(-129, 0);
    panic!("built {built:?}, not refused");
}

#[test]
#[should_panic(
    expected = "`ReprRange::<u8>::from_signed(0, 128)`: the end is above the repr's highest signed \
                value, 127"
)]
fn signed_refuses_an_end_above_its_primitive() {
    let built = ReprRange::<u8>::from_signed(0, 128);
    panic!("built {built:?}, not refused");
}

#[test]
fn a_range_compares_and_prints_its_bounds() {
    let range = ReprRange::<i8>::from_signed(-1, 1);
    assert_ne!(range, ReprRange::from_signed(0, 1), "a range with another start differs");
    assert_ne!(range, ReprRange::from_signed(-1, 0), "and one with another end");
    assert_eq!(
        format!("{range:?}"),
        "ReprRange { start: 255, end: 1 }",
        "`Debug` prints the bounds as unsigned bits"
    );
}

// The 128-bit boundary, in const eval, where an overflow is an error.
const _: () = {
    let every = span(0, u128::MAX, 128);
    assert!(every.contains(u128::MAX) && every.contains(0), "every number of 128 bits");
    let wrapping = span(u128::MAX, 1, 128);
    assert!(wrapping.contains(u128::MAX) && wrapping.contains(0), "-1 through zero");
    assert!(!wrapping.contains(2) && !wrapping.contains(u128::MAX - 1), "to 1");
};

// The 128-bit primitives' ranges, where `mask` shifts by zero.
#[cfg(wide)]
const _: () = {
    let full = ReprRange::<u128>::FULL.span();
    assert!(full.start == 0 && full.end == u128::MAX, "every `u128`");
    let nonzero = ReprRange::<i128>::NONZERO.span();
    assert!(nonzero.start == 1 && nonzero.end == u128::MAX, "every `i128` but zero");
    let signs = ReprRange::<i128>::from_signed(-1, 1).span();
    assert!(signs.start == u128::MAX && signs.end == 1, "-1 to 1, through zero");
    let every = ReprRange::<i128>::from_signed(i128::MIN, i128::MAX).span();
    assert!(every.start == 0 && every.end == u128::MAX, "every `i128`, from zero as it is full");
    let rotated = span(i128::MIN.cast_unsigned(), i128::MAX.cast_unsigned(), 128);
    assert!(rotated.is_full() && !span(1, u128::MAX, 128).is_full(), "a span through zero is full");
};
