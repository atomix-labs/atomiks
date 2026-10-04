//! The range math and the layouts, exhaustively over every span and field of 1 to 4 and 8 bits and
//! every set of 4-bit numbers, checked against searches and rankings written apart from them, and
//! each constructor over the primitives, at the 128-bit boundary too.

extern crate alloc;

use alloc::vec::Vec;
use alloc::{format, vec};

use super::{FieldLayout, NicheLayout, ReprRange, Span, mask, sign_extend};

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
fn each_span<F: Fn(Span)>(check: F) {
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

/// Whether `value` lies among the two's complement numbers of `width` bits, none for 0.
fn fits_signed(value: i128, width: u32) -> bool {
    let half = 1_i128.unbounded_shl(width.wrapping_sub(1));
    width > 0 && (half.wrapping_neg()..half).contains(&value)
}

/// The largest of a set of numbers of one width, and the lowest and highest as two's complement:
/// what the narrowest field that holds them depends on.
type Extremes = (u128, i128, i128);

/// The extremes of `numbers`, of `width` bits, grown from `from`.
fn extremes(numbers: &[u128], width: u32, from: Extremes) -> Extremes {
    numbers.iter().fold(from, |(largest, lowest, highest), &number| {
        let value = sign_extend(number, width);
        (largest.max(number), lowest.min(value), highest.max(value))
    })
}

/// The extremes of no number, which any number's replace.
const NO_EXTREMES: Extremes = (0, i128::MAX, i128::MIN);

/// The narrowest field, found by trying each width, that holds numbers of `width` bits with
/// `extremes`: unsigned, or two's complement where that is narrower.
fn layout_holding((largest, lowest, highest): Extremes, width: u32) -> FieldLayout {
    let unsigned = (0..=width).find(|&narrow| largest <= mask(narrow));
    let signed =
        (0..=width).find(|&narrow| fits_signed(lowest, narrow) && fits_signed(highest, narrow));
    let unsigned = unsigned.expect("the width itself holds every number");
    let signed = signed.expect("the width itself holds every number");
    FieldLayout { width: unsigned.min(signed), signed: signed < unsigned }
}

/// The narrowest field, found by trying each width, that holds every one of `numbers`, of `width`
/// bits.
fn layout_by_search(numbers: &[u128], width: u32) -> FieldLayout {
    layout_holding(extremes(numbers, width, NO_EXTREMES), width)
}

/// The number `Option`'s `None` takes beside `span`, found by ranking the one before its start
/// and the one after its end: the narrower field, then zero, then not wrapping, then before.
fn none_by_ranking(span: Span) -> Option<u128> {
    if span.is_full() {
        return None;
    }
    let (below, above) = (grown_down(span), grown_up(span));
    let rank = |grown: Span, none: u128, after: bool| {
        (grown.field_layout().width, none != 0, grown.wraps(), after)
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

/// `span`'s field is the narrowest that holds its numbers, found by trying each width, and holds
/// each.
fn packs_into_the_narrowest_field(span: Span) {
    let numbers = numbers(span);
    let narrowest = layout_by_search(&numbers, span.width);
    assert_eq!(span.field_layout(), narrowest, "{span:?} packs into {narrowest:?}");
    for number in numbers {
        assert!(narrowest.holds(number, span.width), "{span:?}: its field holds {number}");
    }
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
    let narrowest = grown_down(span).field_layout().width.min(grown_up(span).field_layout().width);
    assert_eq!(
        with_none.field_layout().width,
        narrowest,
        "{span:?}: as narrow as a neighbour allows"
    );
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
    let byte = |start, end| span(start, end, 8).field_layout();
    let (unsigned, signed) =
        (|width| FieldLayout { width, signed: false }, |width| FieldLayout { width, signed: true });
    assert_eq!(byte(0xFF, 0x01), signed(2), "-1 to 1, signed");
    assert_eq!(byte(0xFF, 0x00), signed(1), "-1 and 0");
    assert_eq!(byte(0, 1), unsigned(1), "0 and 1, unsigned");
    assert_eq!(byte(0, 0xFF), unsigned(8), "every byte");
    assert_eq!(byte(0xFB, 0x05), signed(4), "-5 to 5");
    assert_eq!(byte(0, 0), unsigned(0), "zero alone, in no bits");
    assert_eq!(byte(200, 201), signed(7), "200 and 201, -56 and -55 as two's complement");
    assert_eq!(byte(0x80, 0xFF), unsigned(8), "-128 to -1, unsigned on a tie");
    assert_eq!(byte(100, 0), unsigned(8), "through zero and through 127 to -128, every bit");
    let signs = ReprRange::<i8>::from_signed(-1, 1).field_layout();
    assert_eq!(signs, signed(2), "and a range's, as its span's");
}

#[test]
fn a_signed_span_of_any_width_packs_into_the_narrowest_field_that_holds_its_ends() {
    let ends = [i128::from(i64::MIN), -256, -129, -128, -2, -1, 0, 1, 127, 128, 255, 256];
    for (index, &start) in ends.iter().enumerate() {
        for &end in &ends[index..] {
            let narrowest = layout_by_search(&[start.cast_unsigned(), end.cast_unsigned()], 128);
            assert_eq!(FieldLayout::from_signed(start, end), narrowest, "{start} to {end}");
        }
    }
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

/// Runs `check` on every field layout of each of `WIDTHS`, with that width: unsigned of each width
/// up to it, and signed of each from 1.
fn each_layout(check: fn(FieldLayout, u32)) {
    for &width in WIDTHS {
        let unsigned = (0..=width).map(|narrow| FieldLayout { width: narrow, signed: false });
        let signed = (1..=width).map(|narrow| FieldLayout { width: narrow, signed: true });
        for layout in unsigned.chain(signed) {
            check(layout, width);
        }
    }
}

/// `layout` holds exactly the numbers of `width` bits that fit it: below its width's bits where
/// unsigned, among its width's two's complement numbers where signed.
fn holds_only_what_fits(layout: FieldLayout, width: u32) {
    for number in 0..=mask(width) {
        let fits = if layout.signed {
            fits_signed(sign_extend(number, width), layout.width)
        } else {
            number <= mask(layout.width)
        };
        assert_eq!(layout.holds(number, width), fits, "{layout:?} of {width} bits holds {number}");
    }
}

/// `layout` reads back each number of `width` bits it holds, packed at offsets from the lowest to
/// the highest, whatever bits lie beside it.
fn reads_back_what_it_holds(layout: FieldLayout, width: u32) {
    for number in (0..=mask(width)).filter(|&number| layout.holds(number, width)) {
        for offset in [0, 1, 5, u128::BITS.wrapping_sub(layout.width)] {
            let field = mask(layout.width).unbounded_shl(offset);
            let packed = layout.pack(number, offset);
            assert_eq!(packed & !field, 0, "{layout:?} at {offset}: {number} in its own bits");
            assert_eq!(layout.unpack(packed, offset, width), number, "{layout:?}: reads {number}");
            assert_eq!(
                layout.unpack(packed | !field, offset, width),
                number,
                "{layout:?}: reads {number} beside set bits"
            );
        }
    }
}

#[test]
fn every_field_holds_only_the_numbers_that_fit_it() {
    each_layout(holds_only_what_fits);
}

#[test]
fn every_field_reads_back_what_it_holds_at_any_offset() {
    each_layout(reads_back_what_it_holds);
}

#[test]
fn a_field_reads_back_to_its_primitive() {
    let signs = ReprRange::<i8>::from_signed(-1, 1).field_layout();
    let packed = signs.pack(0xFF, 6);
    assert_eq!(packed, 0b1100_0000, "-1 packs into its two bits");
    assert_eq!(signs.unpack(packed, 6, 8), 0xFF, "and reads back as an `i8`'s -1");
    assert_eq!(signs.unpack(packed, 6, 16), 0xFFFF, "or an `i16`'s");
    assert!(signs.holds(0xFF, 8) && !signs.holds(0x02, 8), "it holds -1, not 2");
    let side = ReprRange::<u8>::new(0, 1).field_layout();
    assert_eq!(side.unpack(0b10, 1, 8), 1, "an unsigned field reads back zero-extended");
    assert!(!side.holds(0xFF, 8), "and holds no bit above its own");
}

/// The width of the numbers whose every set the enclosing tests check: Miri's 3 bits, since every
/// set of 4-bit numbers is too many for it.
const SET_WIDTH: u32 = if cfg!(miri) { 3 } else { 4 };

/// Runs `check` on each set of numbers of `SET_WIDTH` bits that is not empty, in ascending order.
fn each_set(check: fn(&[u128])) {
    let count = 1_u32.unbounded_shl(SET_WIDTH);
    for set in 1..1_u32.unbounded_shl(count) {
        let members = (0..count).filter(|&number| set.unbounded_shr(number) & 1 == 1);
        check(&members.map(u128::from).collect::<Vec<_>>());
    }
}

/// The span `Span::enclosing` gives `members`, which are not empty, of `SET_WIDTH` bits.
fn enclosing(members: &mut [u128]) -> Span {
    Span::enclosing(members, SET_WIDTH).expect("each set has a member")
}

/// How many numbers lie in the longest run, round the circle of `SET_WIDTH` bits, that holds none
/// of `members`, found by walking from each number.
fn longest_gap(members: &[u128]) -> usize {
    let largest = mask(SET_WIDTH);
    (0..=largest)
        .map(|start| {
            let steps = 0..=largest;
            steps
                .take_while(|&step| !members.contains(&(start.wrapping_add(step) & largest)))
                .count()
        })
        .max()
        .expect("the circle has a number")
}

/// The span the tie rule gives `members`, found by walking the circle: it leaves out the longest
/// run after a member that holds none, the run through zero where runs tie, else the lowest.
fn enclosing_by_search(members: &[u128]) -> Span {
    let largest = mask(SET_WIDTH);
    let run_after = |member: u128| {
        let steps = 1..=largest;
        let missing =
            steps.take_while(|&step| !members.contains(&(member.wrapping_add(step) & largest)));
        u128::try_from(missing.count()).expect("a run is shorter than the circle")
    };
    let greatest = *members.last().expect("each set has a member");
    let longest = members.iter().map(|&member| run_after(member)).max().expect("as above");
    let before = if run_after(greatest) == longest {
        greatest
    } else {
        *members.iter().find(|&&member| run_after(member) == longest).expect("one is longest")
    };
    span(before.wrapping_add(longest).wrapping_add(1) & largest, before, SET_WIDTH)
}

/// `members`' span holds each, leaves out a longest gap between them, and is the one the tie rule
/// names.
fn encloses_all_but_the_longest_gap(members: &[u128]) {
    let span = enclosing(&mut members.to_vec());
    assert!(members.iter().all(|&member| span.contains(member)), "{members:?} in {span:?}");
    let outside = membership(span).iter().filter(|&&inside| !inside).count();
    assert_eq!(outside, longest_gap(members), "{members:?}: {span:?} leaves out a longest gap");
    assert_eq!(span, enclosing_by_search(members), "{members:?}: the gap the tie rule names");
}

/// `members` give the same span in any order, repeated, and with bits above their width, which it
/// leaves sorted and dropped.
fn encloses_alike_in_any_order(members: &[u128]) {
    let ascending = enclosing(&mut members.to_vec());
    let mut descending: Vec<u128> = members.iter().rev().copied().collect();
    assert_eq!(enclosing(&mut descending), ascending, "{members:?} descending");
    assert_eq!(descending, members, "{members:?}: left in ascending order");
    let mut twice: Vec<u128> = members.iter().chain(members.iter().rev()).copied().collect();
    assert_eq!(enclosing(&mut twice), ascending, "{members:?} twice over");
    let mut wide: Vec<u128> = (0_u32..)
        .zip(members.iter().rev())
        .map(|(index, &member)| member | u128::from(index).unbounded_shl(SET_WIDTH))
        .collect();
    assert_eq!(enclosing(&mut wide), ascending, "{members:?} with bits above their width");
    assert_eq!(wide, members, "{members:?}: left with those bits dropped");
}

/// Nested `Option`s over `members`' span take its spare numbers one by one, none a member's or
/// taken before, until it is full.
fn nested_options_fill_the_gap(members: &[u128]) {
    let span = enclosing(&mut members.to_vec());
    let spare = membership(span).iter().filter(|&&inside| !inside).count();
    let (mut with_nones, mut taken) = (span, members.to_vec());
    while let Some(none) = with_nones.spare_for_none() {
        assert!(!taken.contains(&none), "{members:?}: `None` {none} is spare");
        taken.push(none);
        with_nones = with_nones.including(none);
    }
    assert!(with_nones.is_full(), "{members:?}: refused only once full");
    assert_eq!(taken.len(), members.len().wrapping_add(spare), "{members:?}: each spare taken");
}

#[test]
fn every_set_is_enclosed_by_all_but_its_longest_gap() {
    each_set(encloses_all_but_the_longest_gap);
}

#[test]
fn every_set_is_enclosed_alike_in_any_order() {
    each_set(encloses_alike_in_any_order);
}

#[test]
fn nested_options_fill_the_gap_beside_every_set() {
    each_set(nested_options_fill_the_gap);
}

#[test]
fn a_range_encloses_an_enums_discriminants() {
    assert_eq!(ReprRange::<u8>::enclosing(&mut [1, 0]), ReprRange::new(0, 1), "0 and 1");
    let signs = ReprRange::<i8>::enclosing(&mut [0xFF, 0, 1]);
    assert_eq!(signs, ReprRange::from_signed(-1, 1), "-1, 0 and 1, through zero");
    let ends = ReprRange::<u8>::enclosing(&mut [0, 0xFF]);
    assert_eq!(ends.span(), span(0xFF, 0, 8), "0 and 255, through zero");
    assert_eq!(ReprRange::<u8>::enclosing(&mut [7, 7]), ReprRange::new(7, 7), "one, twice");
    assert_eq!(ReprRange::<bool>::enclosing(&mut [1, 0]), ReprRange::FULL, "both of a `bool`");
    let masked = ReprRange::<u8>::enclosing(&mut [0x1FF, 0x100]);
    assert_eq!(masked, ends, "each to its low byte");
    let mut every: Vec<u128> = (0..=0xFF).rev().collect();
    assert_eq!(ReprRange::<u8>::enclosing(&mut every), ReprRange::FULL, "every byte");
    for hole in 0..=0xFF_u128 {
        let mut others: Vec<u128> = (0..=0xFF).filter(|&number| number != hole).collect();
        let range = ReprRange::<u8>::enclosing(&mut others);
        assert_eq!(range.spare_for_none(), Some(hole), "`None` takes the one hole, {hole}");
    }
}

#[test]
#[should_panic(
    expected = "`ReprRange::<u8>::enclosing(&mut [])`: no bits to enclose, and a range holds at \
                least one repr"
)]
fn enclosing_refuses_no_bits() {
    let built = ReprRange::<u8>::enclosing(&mut []);
    panic!("built {built:?}, not refused");
}

/// The widths of the fields whose every span the top field test packs: Miri's 1 and 2 bits,
/// since it packs each at four offsets into two widths.
const FIELD_WIDTHS: &[u32] = if cfg!(miri) { &[1, 2] } else { &[1, 2, 3, 4] };

/// A packed value `width` bits wide whose top field holds `field`'s numbers at `offset` is stored
/// as exactly the patterns the field packs into, beside any bits below it, and its range holds
/// exactly those, or their hull where the field wraps both ways below the value's top.
fn packs_as_top_field(field: Span, offset: u32, width: u32) {
    let layout = field.field_layout();
    let value_layout = FieldLayout::from_top_field(layout, offset);
    let below = 0..=mask(offset);
    let mut packed: Vec<u128> = numbers(field)
        .into_iter()
        .flat_map(|number| below.clone().map(move |low| layout.pack(number, offset) | low))
        .map(|bits| value_layout.unpack(bits, 0, width))
        .collect();
    packed.sort_unstable();
    let canonical: Vec<u128> = (0..=mask(width))
        .filter(|&bits| value_layout.holds(bits, width))
        .filter(|&bits| field.contains(layout.unpack(bits, offset, field.width)))
        .collect();
    let at = format!("{field:?} at {offset} in {width} bits");
    assert_eq!(canonical, packed, "{at}: the value holds exactly what the field packs");
    let range = Span::from_top_field(field, offset, width);
    let field_end = offset.wrapping_add(layout.width);
    if field.wraps() && !layout.signed && field_end < width {
        assert_eq!(range, span(0, mask(field_end), width), "{at}: the hull");
    } else {
        let inside: Vec<u128> = (0..=mask(width)).filter(|&bits| range.contains(bits)).collect();
        assert_eq!(inside, packed, "{at}: the range holds exactly what the field packs");
    }
}

#[test]
fn a_top_field_gives_its_value_exactly_the_patterns_it_packs() {
    for &field_width in FIELD_WIDTHS {
        let largest = mask(field_width);
        for field in (0..=largest).flat_map(|start| (0..=largest).map(move |end| (start, end))) {
            let field = span(field.0, field.1, field_width);
            for offset in 0..=3 {
                let end = FieldLayout::from_top_field(field.field_layout(), offset).width;
                for width in [end.max(1), 8] {
                    packs_as_top_field(field, offset, width);
                }
            }
        }
    }
}

#[test]
fn a_packed_value_takes_its_range_from_its_top_field() {
    let side = ReprRange::<u8>::new(0, 1);
    let quote = ReprRange::<u64>::from_top_field(side, 32);
    assert_eq!(quote, ReprRange::new(0, (1 << 33) - 1), "unsigned: zeros above the field");
    let signs = ReprRange::<i8>::from_signed(-1, 1);
    let signed = ReprRange::<u16>::from_top_field(signs, 4);
    assert_eq!(signed, ReprRange::from_signed(-16, 31), "signed: its top bit above it");
    let offsets = ReprRange::<i8>::from_signed(-3, -1);
    let negative = ReprRange::<u8>::from_top_field(offsets, 5);
    assert_eq!(negative, ReprRange::from_signed(-96, -1), "all negative: it does not wrap");
    let both_ways = ReprRange::<u8>::from_bounds(100, 0);
    let hull = ReprRange::<u16>::from_top_field(both_ways, 4);
    assert_eq!(hull, ReprRange::new(0, 0xFFF), "wrapping both ways: the hull");
    let exact = ReprRange::<u16>::from_top_field(both_ways, 8);
    assert_eq!(exact.span(), span(100 << 8, 0xFF, 16), "at the top bit: it wraps as the field");
    let layout = FieldLayout::from_top_field(signs.field_layout(), 4);
    assert_eq!(layout, FieldLayout { width: 6, signed: true }, "and its layout, the field's end");
}

/// The reprs `units` unit variants take beside `payload`, whose numbers' extremes are
/// `payload_extremes` and whose membership is `inside`, and the field they and the payload's share,
/// found by ranking the units below it and above it by search; `spare` reprs lie outside it, at
/// least `units`.
fn units_by_ranking(
    payload: Span, payload_extremes: Extremes, inside: &[bool], spare: u128, units: u128,
) -> (Vec<u128>, FieldLayout) {
    let largest = payload.largest();
    let index = |number: u128| usize::try_from(number).expect("a test's width indexes a `Vec`");
    let ranked = |unit_bits: Vec<u128>, is_above: bool| {
        let has = |number: u128| unit_bits.contains(&number) || inside[index(number)];
        let wraps = spare != units && has(0) && has(largest);
        let layout =
            layout_holding(extremes(&unit_bits, payload.width, payload_extremes), payload.width);
        ((layout.width, unit_bits != [0], wraps, is_above), unit_bits, layout)
    };
    let below = (1..=units).rev().map(|step| payload.start.wrapping_sub(step) & largest).collect();
    let above = (1..=units).map(|step| payload.end.wrapping_add(step) & largest).collect();
    let (below, above) = (ranked(below, false), ranked(above, true));
    let (_, unit_bits, layout) = if above.0 < below.0 { above } else { below };
    (unit_bits, layout)
}

/// `units` unit variants beside `span` take the reprs the ranking by search gives them, where they
/// fit and a tag is no narrower.
fn niche_takes_the_ranked_reprs(span: Span, units: u128) {
    let niche = NicheLayout::beside(span, units);
    let payload_extremes = extremes(&numbers(span), span.width, NO_EXTREMES);
    let inside = membership(span);
    let spare = inside.iter().filter(|&&is| !is).count();
    let spare = u128::try_from(spare).expect("a count of a test's width");
    if spare < units {
        assert_eq!(niche, None, "{span:?}: {units} units do not fit");
        return;
    }
    let (unit_bits, layout) = units_by_ranking(span, payload_extremes, &inside, spare, units);
    let tag = (0..=u128::BITS).find(|&bits| units.unbounded_shr(bits) == 0);
    let tagged = layout_holding(payload_extremes, span.width)
        .width
        .wrapping_add(tag.expect("a count has bits"));
    if layout.width > tagged {
        assert_eq!(niche, None, "{span:?}: {units} units take a narrower tag");
        return;
    }
    let niche = niche.expect("the search fills the niche");
    let found: Vec<u128> = (0..units).map(|unit| niche.unit_bits(unit)).collect();
    assert_eq!(found, unit_bits, "{span:?}: {units} units take the ranked reprs");
    let taken = |(bits, &is): (u128, &bool)| is || unit_bits.contains(&bits);
    assert!(
        (0..).zip(&inside).all(|number| niche.span.contains(number.0) == taken(number)),
        "{span:?}: {units} units and the payload, and nothing else"
    );
    assert_eq!(niche.field_layout(), layout, "{span:?}: in the field they share");
}

/// One unit variant beside `span` takes the repr `Option`'s `None` would.
fn lone_unit_takes_options_none(span: Span) {
    let niche = NicheLayout::beside(span, 1);
    let Some(none) = span.spare_for_none() else {
        assert_eq!(niche, None, "{span:?}: no repr spare");
        return;
    };
    let niche = niche.expect("a lone unit is never wider than its tag");
    assert_eq!(niche.unit_bits(0), none, "{span:?}: the unit takes `None`'s repr");
    assert_eq!(niche.span, span.including(none).normalized(), "{span:?}: and the range grows so");
}

#[test]
fn one_unit_beside_every_span_takes_the_repr_a_search_ranks_first() {
    each_span(|span| niche_takes_the_ranked_reprs(span, 1));
}

#[test]
fn two_units_beside_every_span_take_the_reprs_a_search_ranks_first() {
    each_span(|span| niche_takes_the_ranked_reprs(span, 2));
}

#[test]
fn three_units_beside_every_span_take_the_reprs_a_search_ranks_first() {
    each_span(|span| niche_takes_the_ranked_reprs(span, 3));
}

#[test]
fn a_lone_unit_beside_every_span_takes_options_none() {
    each_span(lone_unit_takes_options_none);
}

#[test]
fn units_fill_the_niche_beside_a_payload() {
    let owner = ReprRange::<u64>::new(3, u128::from(u64::MAX));
    let lock = NicheLayout::new(2, owner).expect("two units fit below an owner");
    assert_eq!((lock.unit_bits(0), lock.unit_bits(1)), (1, 2), "`Uninit` and `Free` take 1 and 2");
    assert_eq!(lock.range::<u64>(), ReprRange::NONZERO, "leaving 0 to `Option`'s `None`");
    let side = NicheLayout::new(1, ReprRange::<u8>::new(0, 1)).expect("a unit fits beside a side");
    assert_eq!(side.unit_bits(0), 2, "after the end of a range from zero, as `None` would");
    let signs = NicheLayout::new(1, ReprRange::<i8>::from_signed(-1, 1)).expect("one fits");
    assert_eq!(signs.unit_bits(0), 0xFE, "below -1, as `None` would");
    assert_eq!(signs.range::<u8>(), ReprRange::from_signed(-2, 1), "in two bits, signed");
    assert_eq!(NicheLayout::new(2, ReprRange::<u8>::NONZERO), None, "two units need a tag");
}

#[test]
fn a_niche_narrower_than_its_payload_reads_back_to_the_payloads_primitive() {
    let level = ReprRange::<i16>::from_signed(-100, 100);
    let niche = NicheLayout::new(1, level).expect("a unit fits beside a level");
    assert_eq!(niche.unit_bits(0), 0xFF9B, "-101, an `i16`'s");
    let layout = niche.field_layout();
    assert_eq!(layout, FieldLayout { width: 8, signed: true }, "in a byte");
    assert_eq!(niche.range::<u8>(), ReprRange::from_signed(-101, 100), "a `u8` repr holds it");
    assert_eq!(layout.unpack(0x9B, 0, 16), niche.unit_bits(0), "read back to the `i16`'s");
    assert_eq!(layout.unpack(niche.unit_bits(0), 0, 8), 0x9B, "and stored as the `u8`'s");
}

/// -2 as 128 bits of two's complement.
const MINUS_TWO: u128 = (-2_i128).cast_unsigned();

// The 128-bit boundary, in const eval, where an overflow is an error.
const _: () = {
    let every = span(0, u128::MAX, 128);
    assert!(every.contains(u128::MAX) && every.contains(0), "every number of 128 bits");
    assert!(every.spare_for_none().is_none(), "none spare");
    assert!(
        every.field_layout().width == 128 && !every.field_layout().signed,
        "128 bits, unsigned"
    );
    let signs = span(u128::MAX, 1, 128);
    assert!(signs.contains(u128::MAX) && signs.contains(0), "-1 through zero");
    assert!(!signs.contains(2) && !signs.contains(MINUS_TWO), "to 1");
    assert!(matches!(signs.spare_for_none(), Some(MINUS_TWO)), "`None` takes -2");
    assert!(signs.field_layout().width == 2 && signs.field_layout().signed, "in 2 bits, signed");
    assert!(matches!(span(1, u128::MAX, 128).spare_for_none(), Some(0)), "from 1: zero");
    let top = span(u128::MAX, u128::MAX, 128);
    assert!(matches!(top.spare_for_none(), Some(0)), "the largest alone: zero, after it");
    let with_zero = top.including(0);
    assert!(with_zero.start == u128::MAX && with_zero.end == 0, "grown up, through zero");
    let below_top = span(0, MINUS_TWO, 128);
    assert!(matches!(below_top.spare_for_none(), Some(u128::MAX)), "from zero: the largest");
    let low_half = span(0, i128::MAX.cast_unsigned(), 128);
    assert!(low_half.field_layout().width == 127, "the low half, 127 bits unsigned");
    let high_half = span(i128::MIN.cast_unsigned(), u128::MAX, 128);
    assert!(high_half.field_layout().width == 128, "the high half, 128 bits either way");
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

// The layouts at the 128-bit boundary, in const eval, where an overflow is an error.
const _: () = {
    let minus_five = (-5_i128).cast_unsigned();
    let fives = span(minus_five, 5, 128);
    let field = fives.field_layout();
    assert!(field.width == 4 && field.signed, "-5 to 5 in 4 bits, signed");
    assert!(field.pack(minus_five, 124) == 0b1011 << 124, "packed into the top four bits");
    assert!(field.unpack(0b1011 << 124, 124, 128) == minus_five, "and read back");
    assert!(field.holds(minus_five, 128) && !field.holds(1 << 127, 128), "holding -5, not -2^127");
    let top = Span::from_top_field(fives, 124, 128);
    assert!(top.start == minus_five << 124 && top.end == (5 << 124) | mask(124), "at the top");
    let both_ways = span(100, 0, 64);
    let hull = Span::from_top_field(both_ways, 0, 128);
    assert!(hull.start == 0 && hull.end == mask(64), "wrapping both ways below the top: the hull");
    let exact = Span::from_top_field(both_ways, 64, 128);
    assert!(exact.start == 100 << 64 && exact.end == mask(64), "at the top: it wraps");
    let mut signs = [u128::MAX, 0, 1];
    let Some(enclosed) = Span::enclosing(&mut signs, 128) else { panic!("three numbers") };
    assert!(enclosed.start == u128::MAX && enclosed.end == 1, "-1, 0 and 1, through zero");
    let mut ends = [u128::MAX, 0];
    let Some(enclosed) = Span::enclosing(&mut ends, 128) else { panic!("two numbers") };
    assert!(enclosed.start == u128::MAX && enclosed.end == 0, "the largest and zero");
    let owner = span(3, u128::MAX, 128);
    let Some(lock) = NicheLayout::beside(owner, 2) else { panic!("two units fit below") };
    assert!(lock.unit_bits(0) == 1 && lock.unit_bits(1) == 2, "below the owner");
    assert!(lock.field_layout().width == 128, "in 128 bits");
    let Some(below_top) = NicheLayout::beside(span(0, MINUS_TWO, 128), 1) else { panic!("fits") };
    assert!(below_top.unit_bits(0) == u128::MAX, "the largest, after the end of a span from zero");
    assert!(below_top.span.is_full() && below_top.span.start == 0, "filling every number");
    assert!(NicheLayout::beside(span(0, u128::MAX, 128), 1).is_none(), "and none beside them all");
};

/// How many discriminants the sort test encloses: eight times where an insertion sort's steps stop
/// const eval, and a quarter of what the heapsort's fit.
const DISCRIMINANTS: usize = 4096;

// An enum of thousands of discriminants, in descending order, so the heapsort runs, in const eval,
// whose step limit fails the build where a quadratic sort's steps run out.
const _: () = {
    let mut descending = [0_u128; DISCRIMINANTS];
    let mut next = 0_u128;
    let mut index = DISCRIMINANTS;
    while index > 0 {
        index -= 1;
        descending[index] = next;
        next += 1;
    }
    let Some(enclosed) = Span::enclosing(&mut descending, 16) else { panic!("4,096 numbers") };
    assert!(enclosed.start == 0 && enclosed.end == next - 1, "from zero to the last");
    let mut index = 1;
    while index < DISCRIMINANTS {
        assert!(descending[index - 1] < descending[index], "left ascending");
        index += 1;
    }
};
