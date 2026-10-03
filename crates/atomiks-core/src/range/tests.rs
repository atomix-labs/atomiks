//! The range math, exhaustively over every span of 1 to 4 and 8 bits, checked against searches
//! and a ranking written apart from it, and each constructor over the primitives, at the 128-bit
//! boundary too.

extern crate alloc;

use alloc::vec::Vec;
use alloc::{format, vec};

use super::{ReprRange, Span, mask, sign_extend};

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

/// The widths whose every span the exhaustive tests check: Miri runs those of 1 to 4 bits only,
/// since every 8-bit span is too many for it.
const WIDTHS: &[u32] = if cfg!(miri) { &[1, 2, 3, 4] } else { &[1, 2, 3, 4, 8] };

/// Runs `check` on every span of each of `WIDTHS`.
fn each_span(check: fn(Span)) {
    for &width in WIDTHS {
        let largest = mask(width);
        for start in 0..=largest {
            for end in 0..=largest {
                check(span(start, end, width));
            }
        }
    }
}

/// `span` grown by the number before its start.
fn grown_down(span: Span) -> Span {
    Span { start: span.start.wrapping_sub(1) & span.largest(), ..span }
}

/// `span` grown by the number after its end.
fn grown_up(span: Span) -> Span {
    Span { end: span.end.wrapping_add(1) & span.largest(), ..span }
}

/// The narrowest width, found by trying each, in which every one of `numbers`, of `width` bits,
/// fits as an unsigned or as a two's complement number.
fn narrowest_by_search(numbers: &[u128], width: u32) -> u32 {
    (0..=width)
        .find(|&narrow| {
            let unsigned = numbers.iter().all(|&number| number <= mask(narrow));
            let half = 1_i128.unbounded_shl(narrow.wrapping_sub(1));
            let signed = narrow > 0
                && numbers.iter().all(|&number| {
                    (half.wrapping_neg()..half).contains(&sign_extend(number, width))
                });
            unsigned || signed
        })
        .expect("the width itself holds every number")
}

/// The number `Option`'s `None` takes beside `span`, found by ranking the one before its start
/// and the one after its end: the narrower field, then zero, then not wrapping, then before.
fn none_by_ranking(span: Span) -> Option<u128> {
    if span.is_full() {
        return None;
    }
    let (below, above) = (grown_down(span), grown_up(span));
    let rank = |grown: Span, none: u128, after: bool| {
        (grown.field_width(), none != 0, grown.wraps(), after)
    };
    let (below_rank, above_rank) = (rank(below, below.start, false), rank(above, above.end, true));
    Some(if above_rank < below_rank { above.end } else { below.start })
}

/// `span` holds exactly its numbers, and is full where it holds every one.
fn holds_its_numbers(span: Span) {
    let membership = membership(span);
    for (bits, &inside) in (0..).zip(&membership) {
        assert_eq!(span.contains(bits), inside, "{span:?} holds {bits}");
    }
    assert!(!span.contains(span.largest().wrapping_add(1)), "{span:?}: nothing wider");
    assert_eq!(span.wraps(), span.start > span.end, "{span:?} wraps through zero");
    assert_eq!(span.is_full(), !membership.contains(&false), "{span:?} is full");
}

/// `span`'s field is the narrowest that holds its numbers, found by trying each width.
fn packs_into_the_narrowest_field(span: Span) {
    let narrowest = narrowest_by_search(&numbers(span), span.width);
    assert_eq!(span.field_width(), narrowest, "{span:?} packs into {narrowest} bits");
}

/// `None` takes the number beside `span` the ranking picks, and the span grown to hold it gains
/// that number alone, as narrow as either neighbour allows.
fn gives_none_a_spare_number(span: Span) {
    assert_eq!(span.spare_for_none(), none_by_ranking(span), "{span:?}: the ranking's pick");
    let Some(none) = span.spare_for_none() else {
        assert!(span.is_full(), "{span:?}: only a full span leaves `None` no number");
        return;
    };
    assert!(!span.contains(none), "{span:?}: `None`, {none}, lies outside");
    let with_none = span.including(none);
    let (was_inside, is_inside) = (membership(span), membership(with_none));
    for (bits, (&was, &is)) in (0..).zip(was_inside.iter().zip(&is_inside)) {
        assert_eq!(is, was || bits == none, "{span:?} grown by {none}: holds {bits}");
    }
    let narrowest = grown_down(span).field_width().min(grown_up(span).field_width());
    assert_eq!(with_none.field_width(), narrowest, "{span:?}: as narrow as a neighbour allows");
}

/// Nested `Option`s take `span`'s spare numbers one by one, each beside the span grown by those
/// before, until it is full.
fn nested_options_take_every_spare_number(span: Span) {
    let spare = membership(span).iter().filter(|&&inside| !inside).count();
    let (mut with_nones, mut taken) = (span, 0_usize);
    while let Some(none) = with_nones.spare_for_none() {
        let next = with_nones.including(none);
        assert!(
            next == grown_down(with_nones) || next == grown_up(with_nones),
            "{span:?}: `None` {none} lies beside {with_nones:?}"
        );
        (with_nones, taken) = (next, taken.wrapping_add(1));
    }
    assert!(with_nones.is_full(), "{span:?}: refused only once full");
    assert_eq!(taken, spare, "{span:?}: every spare number taken");
}

#[test]
fn every_span_holds_its_numbers() {
    each_span(holds_its_numbers);
}

#[test]
fn every_span_packs_into_the_narrowest_field() {
    each_span(packs_into_the_narrowest_field);
}

#[test]
fn every_span_gives_none_a_spare_number() {
    each_span(gives_none_a_spare_number);
}

#[test]
fn nested_options_take_every_spare_number_of_every_span() {
    each_span(nested_options_take_every_spare_number);
}

#[test]
fn a_span_packs_into_the_narrower_of_unsigned_and_signed() {
    let byte = |start, end| span(start, end, 8).field_width();
    assert_eq!(byte(0xFF, 0x01), 2, "-1 to 1, signed");
    assert_eq!(byte(0xFF, 0x00), 1, "-1 and 0");
    assert_eq!(byte(0, 1), 1, "0 and 1, unsigned");
    assert_eq!(byte(0, 0xFF), 8, "every byte");
    assert_eq!(byte(0xFB, 0x05), 4, "-5 to 5");
    assert_eq!(byte(0, 0), 0, "zero alone, in no bits");
    assert_eq!(byte(200, 201), 7, "200 and 201, -56 and -55 as two's complement");
    assert_eq!(byte(100, 0), 8, "through zero and through 127 to -128, every bit");
}

#[test]
fn none_takes_the_repr_each_rule_names() {
    assert_eq!(ReprRange::<u8>::new(0, 1).spare_for_none(), Some(2), "from zero: after the end");
    assert_eq!(ReprRange::<u8>::new(200, 201).spare_for_none(), Some(199), "a tie: before");
    assert_eq!(ReprRange::<u8>::NONZERO.spare_for_none(), Some(0), "from 1: zero");
    assert_eq!(ReprRange::<u8>::new(128, 255).spare_for_none(), Some(0), "to the top: zero");
    assert_eq!(ReprRange::<i8>::from_signed(-1, 1).spare_for_none(), Some(0xFE), "-1 to 1: -2");
    assert_eq!(
        ReprRange::<i8>::from_signed(-2, 1).spare_for_none(),
        Some(0xFD),
        "-2 to 1: -3, a tie"
    );
    assert_eq!(
        ReprRange::<i8>::from_signed(-4, 1).spare_for_none(),
        Some(2),
        "-4 to 1: 2, narrower"
    );
    assert_eq!(ReprRange::<u8>::FULL.spare_for_none(), None, "every repr: none");
    assert_eq!(span(5, 4, 8).spare_for_none(), None, "nor a full span through zero");
}

#[test]
fn a_range_grows_to_its_nearer_side() {
    let signs = ReprRange::<i8>::from_signed(-1, 1);
    assert_eq!(signs.including(0xFE), ReprRange::from_signed(-2, 1), "down, through zero");
    assert_eq!(ReprRange::<u8>::new(0, 1).including(2), ReprRange::new(0, 2), "up");
    assert_eq!(ReprRange::<u8>::new(16, 32).including(0), ReprRange::new(0, 32), "the nearer way");
    assert_eq!(ReprRange::<u8>::new(1, 5).including(3), ReprRange::new(1, 5), "not where it holds");
    assert_eq!(ReprRange::<u8>::NONZERO.including(0), ReprRange::FULL, "to every repr");
    let all_but_the_lowest = ReprRange::<i8>::from_signed(-127, 127);
    assert_eq!(all_but_the_lowest.including(0x80), ReprRange::FULL, "from zero once full");
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

/// -2 as 128 bits of two's complement.
const MINUS_TWO: u128 = (-2_i128).cast_unsigned();

// The 128-bit boundary, in const eval, where an overflow is an error.
const _: () = {
    let every = span(0, u128::MAX, 128);
    assert!(every.contains(u128::MAX) && every.contains(0), "every number of 128 bits");
    assert!(every.spare_for_none().is_none() && every.field_width() == 128, "none spare, 128 bits");
    let signs = span(u128::MAX, 1, 128);
    assert!(signs.contains(u128::MAX) && signs.contains(0), "-1 through zero");
    assert!(!signs.contains(2) && !signs.contains(MINUS_TWO), "to 1");
    assert!(matches!(signs.spare_for_none(), Some(MINUS_TWO)), "`None` takes -2");
    assert!(signs.field_width() == 2, "in 2 bits, signed");
    assert!(matches!(span(1, u128::MAX, 128).spare_for_none(), Some(0)), "from 1: zero");
    let top = span(u128::MAX, u128::MAX, 128);
    assert!(matches!(top.spare_for_none(), Some(0)), "the largest alone: zero, after it");
    let with_zero = top.including(0);
    assert!(with_zero.start == u128::MAX && with_zero.end == 0, "grown up, through zero");
    let below_top = span(0, MINUS_TWO, 128);
    assert!(matches!(below_top.spare_for_none(), Some(u128::MAX)), "from zero: the largest");
    let low_half = span(0, i128::MAX.cast_unsigned(), 128);
    assert!(low_half.field_width() == 127, "the low half, 127 bits unsigned");
    let high_half = span(i128::MIN.cast_unsigned(), u128::MAX, 128);
    assert!(high_half.field_width() == 128, "the high half, 128 bits either way");
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
    assert!(
        matches!(ReprRange::<i128>::from_signed(-1, 1).spare_for_none(), Some(MINUS_TWO)),
        "-2"
    );
    assert!(matches!(ReprRange::<u128>::NONZERO.spare_for_none(), Some(0)), "and zero beside 1");
    let every = ReprRange::<i128>::from_signed(i128::MIN, i128::MAX).span();
    assert!(every.start == 0 && every.end == u128::MAX, "every `i128`, from zero as it is full");
    let rotated = span(i128::MIN.cast_unsigned(), i128::MAX.cast_unsigned(), 128);
    assert!(rotated.is_full() && !span(1, u128::MAX, 128).is_full(), "a span through zero is full");
};
