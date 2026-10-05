//! [`ReprRange`], the reprs a value takes, the math of a range of numbers of any width that wraps
//! through zero, and how a packed value lays out its fields and an enum its variants.

mod layout;

use core::any::type_name;
use core::fmt;
use core::marker::PhantomData;

pub(crate) use self::layout::FieldLayout;
pub use self::layout::{EnumLayout, PackedField, PackedLayout};
use crate::message::{Message, refuse};
use crate::primitive::Primitive;

/// The reprs a value takes: those of the primitive `R` from `start` up to `end`, as unsigned bits,
/// wrapping through zero where `start` is above `end`, as rustc's [`WrappingRange`] does.
///
/// Each value's [`REPRS`](crate::Atom::REPRS) is one. A range holds at least one repr, and where it
/// holds every one it runs from zero, so ranges that hold the same reprs are equal.
/// [`new`](Self::new) builds a range in unsigned order and [`from_signed`](Self::from_signed) one
/// in signed order, through zero where it starts below zero and ends above it; each refuses a range
/// it cannot hold, at compile time in a constant.
///
/// # Examples
/// ```
/// # extern crate atomiks_core as atomiks;
/// use atomiks::ReprRange;
///
/// // The ten digits.
/// const DIGITS: ReprRange<u8> = ReprRange::new(0, 9);
/// assert!(DIGITS.contains(9) && !DIGITS.contains(10), "0 to 9");
///
/// // -1, 0 and 1 as an `i8`, whose bits run from 0xFF through zero to 0x01.
/// const SIGNS: ReprRange<i8> = ReprRange::from_signed(-1, 1);
/// assert!(SIGNS.contains(0xFF) && SIGNS.contains(0x01), "-1 and 1");
/// assert!(!SIGNS.contains(0xFE) && !SIGNS.contains(0x02), "and neither -2 nor 2");
/// ```
///
/// [`WrappingRange`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_abi/struct.WrappingRange.html
pub struct ReprRange<R> {
    /// The first repr, as unsigned bits.
    start: u128,
    /// The last repr, as unsigned bits.
    end: u128,
    /// The primitive whose reprs the range holds; a `fn() -> R` leaves the range `Send` and `Sync`
    /// whatever `R` is.
    marker: PhantomData<fn() -> R>,
}

impl<R: Primitive> ReprRange<R> {
    /// Every repr.
    pub const FULL: Self = Self::from_bounds(0, mask(R::BITS));
    /// Every repr but zero.
    pub const NONZERO: Self = Self::from_bounds(1, mask(R::BITS));

    /// The reprs from `start` up to `end`, as unsigned bits: an `i8`'s -1 is `0xFF`.
    ///
    /// # Panics
    /// Where `start` is above `end`, or `end` above the repr's largest bits; in a constant, the
    /// build fails instead. [`from_signed`](Self::from_signed) builds a range through zero.
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(start: u128, end: u128) -> Self {
        if start > end {
            refuse_new::<R>(
                start,
                end,
                &Message::new().text(concat!(
                    "the start is above the end: list the bounds in unsigned order, ",
                    "or call `ReprRange::from_signed` for a range through zero"
                )),
            );
        }
        let largest = mask(R::BITS);
        if end > largest {
            refuse_new::<R>(
                start,
                end,
                &Message::new().text("the end is above the repr's largest bits, ").number(largest),
            );
        }
        Self::from_bounds(start, end)
    }

    /// The reprs from `start` up to `end` in signed order, as two's complement: through zero where
    /// `start` is negative and `end` is not.
    ///
    /// # Panics
    /// Where `start` is above `end`, or a bound lies outside the signed values of the repr's
    /// width; in a constant, the build fails instead.
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn from_signed(start: i128, end: i128) -> Self {
        if start > end {
            refuse_from_signed::<R>(
                start,
                end,
                &Message::new().text("the start is above the end: list the bounds in signed order"),
            );
        }
        let lowest = i128::MIN.unbounded_shr(i128::BITS.saturating_sub(R::BITS));
        if start < lowest {
            refuse_from_signed::<R>(
                start,
                end,
                &Message::new()
                    .text("the start is below the repr's lowest signed value, ")
                    .signed_number(lowest),
            );
        }
        let highest = !lowest;
        if end > highest {
            refuse_from_signed::<R>(
                start,
                end,
                &Message::new()
                    .text("the end is above the repr's highest signed value, ")
                    .signed_number(highest),
            );
        }
        let largest = mask(R::BITS);
        Self::from_bounds(start.cast_unsigned() & largest, end.cast_unsigned() & largest)
    }

    /// Whether `bits`, a repr's unsigned bits, lie in the range.
    #[inline]
    #[must_use]
    pub const fn contains(self, bits: u128) -> bool {
        self.span().contains(bits)
    }

    /// The narrowest range that holds each of `bits`, reprs as unsigned bits, such as an enum's
    /// discriminants: every repr of `R` but the longest run, counted round through zero, that holds
    /// none of them.
    ///
    /// Where runs tie, the one through zero is left out, so the range runs from the smallest of
    /// `bits` to the largest; else the lowest. Bits above `R`'s width are dropped first, and
    /// duplicates count once. `bits` is left sorted, by a heapsort that passes over bits already in
    /// order, so a constant evaluates thousands of discriminants in `k log k` steps.
    ///
    /// # Panics
    /// Where `bits` is empty, since a range holds at least one repr; in a constant, the build fails
    /// instead.
    #[inline]
    #[must_use]
    #[track_caller]
    pub(crate) const fn enclosing(bits: &mut [u128]) -> Self {
        let Some(span) = Span::enclosing(bits, R::BITS) else {
            refuse_call::<R>(
                &Message::new().text("enclosing(&mut [])"),
                &Message::new().text("no bits to enclose, and a range holds at least one repr"),
            );
        };
        Self::from_bounds(span.start, span.end)
    }

    /// The range of a packed value, of repr `R`, whose top field is `top`, beside any bits below
    /// it.
    ///
    /// Named for the one field that decides it, the last of nonzero width: the fields below it fill
    /// the low bits, and a range, which has no holes, holds their every pattern, so the value's
    /// range is the field's moved up to its offset, its low bits all clear at its start and all
    /// set at its end. Above the field, the value's bits extend it as its [`FieldLayout`] reads it
    /// back: with zeros, or with copies of its top bit where it is signed, so the range may wrap.
    ///
    /// Where the field wraps both through zero and through the signed numbers' ends, it takes its
    /// repr's every bit, and the range is every repr up to the field's end; at `R`'s top bit that
    /// is exact, and the range wraps as the field does. Bits past `R`'s width are dropped, not
    /// refused: the derive refuses a value wider than its repr where it checks the width, once.
    #[inline]
    #[must_use]
    const fn from_top_field(top: PackedField) -> Self {
        let span = Span::from_top_field(top.reprs, top.offset, R::BITS);
        Self::from_bounds(span.start, span.end)
    }

    /// The smallest range that holds this one and `bits`, a repr's unsigned bits: grown down to
    /// them or up to them, whichever adds fewer reprs, down on a tie.
    #[inline]
    #[must_use]
    pub(crate) const fn including(self, bits: u128) -> Self {
        let span = self.span().including(bits);
        Self::from_bounds(span.start, span.end)
    }

    /// The repr beside the range that `Option`'s `None` takes where the range alone decides, as
    /// [`Span::grown_by`] picks one, or `None` where the range is full.
    #[inline]
    #[must_use]
    pub(crate) const fn spare_for_none(self) -> Option<u128> {
        self.span().spare_for_none()
    }

    /// The range from `start` to `end`, which the caller keeps within `R`'s bits; from zero where
    /// it holds every repr, so ranges that hold the same reprs are equal.
    #[inline]
    #[must_use]
    const fn from_bounds(start: u128, end: u128) -> Self {
        let span = Span { start, end, width: R::BITS }.normalized();
        Self { start: span.start, end: span.end, marker: PhantomData }
    }

    /// The range as a span of `R::BITS`-bit numbers.
    #[inline]
    #[must_use]
    const fn span(self) -> Span {
        Span { start: self.start, end: self.end, width: R::BITS }
    }
}

// The five impls below are written out: std's derive bounds `R` by its trait, and `Primitive`
// implies neither `Debug` nor `PartialEq`, so generic code could not print or compare a range; no
// derive crate that leaves `R` unbounded is a dependency.
impl<R> Clone for ReprRange<R> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for ReprRange<R> {}

impl<R> PartialEq for ReprRange<R> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}

impl<R> Eq for ReprRange<R> {}

impl<R> fmt::Debug for ReprRange<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReprRange").field("start", &self.start).field("end", &self.end).finish()
    }
}

/// Refuses `ReprRange::<R>::new(start, end)` with `advice`.
#[track_caller]
const fn refuse_new<R>(start: u128, end: u128, advice: &Message) -> ! {
    refuse_call::<R>(
        &Message::new().text("new(").number(start).text(", ").number(end).text(")"),
        advice,
    )
}

/// Refuses `ReprRange::<R>::from_signed(start, end)` with `advice`.
#[track_caller]
const fn refuse_from_signed<R>(start: i128, end: i128, advice: &Message) -> ! {
    refuse_call::<R>(
        &Message::new()
            .text("from_signed(")
            .signed_number(start)
            .text(", ")
            .signed_number(end)
            .text(")"),
        advice,
    )
}

/// Refuses the range of `R` that `call`, a constructor and its bounds, builds, with `advice`.
///
/// Cuts `R`'s name short where the rest would not fit after it.
#[track_caller]
const fn refuse_call<R>(call: &Message, advice: &Message) -> ! {
    let rest = Message::new().text(">::").text(call.as_str()).text("`: ").text(advice.as_str());
    let rest = rest.as_str();
    refuse(&Message::new().text("`ReprRange::<").name(type_name::<R>(), rest.len()).text(rest))
}

/// The numbers of `width` bits from `start` up to `end`, wrapping through zero where `start` is
/// above `end`: a [`ReprRange`]'s math, for any width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Span {
    /// The first number.
    start: u128,
    /// The last number.
    end: u128,
    /// How many bits each number has, 1 to 128.
    width: u32,
}

impl Span {
    /// The largest number of the width, every bit set.
    #[inline]
    #[must_use]
    const fn largest(self) -> u128 {
        mask(self.width)
    }

    /// Whether the span wraps through zero.
    #[inline]
    #[must_use]
    const fn wraps(self) -> bool {
        self.start > self.end
    }

    /// Whether the span holds every number of its width: the number after its end is its start.
    #[inline]
    #[must_use]
    const fn is_full(self) -> bool {
        self.end.wrapping_add(1) & self.largest() == self.start
    }

    /// How many numbers of the width lie outside the span.
    #[inline]
    #[must_use]
    const fn spare_count(self) -> u128 {
        // The span holds one number more than `end - start`, modulo the width.
        let largest = self.largest();
        largest.wrapping_sub(self.end.wrapping_sub(self.start) & largest)
    }

    /// The span, from zero where it holds every number, as a [`ReprRange`] keeps it.
    #[inline]
    #[must_use]
    const fn normalized(self) -> Self {
        if self.is_full() { Self { start: 0, end: self.largest(), ..self } } else { self }
    }

    /// Whether `bits` lie in the span: never where they are wider than it.
    #[inline]
    #[must_use]
    const fn contains(self, bits: u128) -> bool {
        if self.wraps() {
            (bits >= self.start && bits <= self.largest()) || bits <= self.end
        } else {
            bits >= self.start && bits <= self.end
        }
    }

    /// The smallest span that holds this one and `bits`, a number of its width: grown down to them
    /// or up to them, whichever adds fewer numbers, down on a tie.
    #[inline]
    #[must_use]
    const fn including(self, bits: u128) -> Self {
        if self.contains(bits) {
            return self;
        }
        let largest = self.largest();
        let down = self.start.wrapping_sub(bits) & largest;
        let up = bits.wrapping_sub(self.end) & largest;
        if up < down { Self { end: bits, ..self } } else { Self { start: bits, ..self } }
    }

    /// The number `Option`'s `None` takes beside the span, as [`grown_by`](Self::grown_by) picks
    /// one, or `None` where the span is full.
    #[inline]
    #[must_use]
    const fn spare_for_none(self) -> Option<u128> {
        match self.grown_by(1) {
            Some((_, none)) => Some(none),
            None => None,
        }
    }

    /// The span grown by `count` numbers beside it, and the first of them, each next one after it;
    /// `None` where fewer than `count` lie outside.
    ///
    /// They go below the start or above the end, whichever leaves the narrower field; on a tie, to
    /// the side that puts one number alone at zero, then to the side whose span does not wrap, then
    /// below. So `Option`'s `None` takes zero where it is beside the span, and the number after the
    /// end of a span from zero.
    #[inline]
    #[must_use]
    const fn grown_by(self, count: u128) -> Option<(Self, u128)> {
        if self.spare_count() < count {
            return None;
        }
        let largest = self.largest();
        let below = Self { start: self.start.wrapping_sub(count) & largest, ..self };
        let above = Self { end: self.end.wrapping_add(count) & largest, ..self };
        let after_end = self.end.wrapping_add(1) & largest;
        let (below_width, above_width) = (below.field_layout().width, above.field_layout().width);
        let goes_above = if below_width != above_width {
            above_width < below_width
        } else if count == 1 && (below.start == 0) != (after_end == 0) {
            after_end == 0
        } else {
            below.wraps() && !above.wraps()
        };
        Some(if goes_above { (above, after_end) } else { (below, below.start) })
    }

    /// How a packed value stores a field of the span's numbers: unsigned, or two's complement where
    /// that is narrower, unsigned on a tie; the whole width where the span wraps both through zero
    /// and through the signed numbers' ends.
    #[inline]
    #[must_use]
    const fn field_layout(self) -> FieldLayout {
        let first = sign_extend(self.start, self.width);
        let last = sign_extend(self.end, self.width);
        let signed = if first > last {
            self.width
        } else {
            let (low, high) = (signed_bit_length(first), signed_bit_length(last));
            if low > high { low } else { high }
        };
        let unsigned = if self.wraps() { self.width } else { bit_length(self.end) };
        if signed < unsigned {
            FieldLayout { width: signed, signed: true }
        } else {
            FieldLayout { width: unsigned, signed: false }
        }
    }

    /// The narrowest span of `width`-bit numbers that holds each of `bits`, which it masks to the
    /// width and sorts, as [`ReprRange::enclosing`]; `None` where `bits` is empty.
    #[inline]
    #[must_use]
    const fn enclosing(bits: &mut [u128], width: u32) -> Option<Self> {
        let largest = mask(width);
        let mut rest: &mut [u128] = bits;
        while let [number, after @ ..] = rest {
            *number &= largest;
            rest = after;
        }
        sort(bits);
        let (Some(&smallest), Some(&greatest)) = (bits.first(), bits.last()) else {
            return None;
        };
        // First the run from the greatest round through zero to the smallest, so a tie leaves the
        // span from the smallest to the greatest; then the run between each pair of neighbours.
        let mut run = smallest.wrapping_sub(greatest).wrapping_sub(1) & largest;
        let (mut start, mut end) = (smallest, greatest);
        let mut rest: &[u128] = bits;
        while let [low, after @ ..] = rest {
            rest = after;
            let [high, ..] = *rest else { break };
            let between = if high == *low { 0 } else { high.wrapping_sub(*low).wrapping_sub(1) };
            if between > run {
                (start, end, run) = (high, *low, between);
            }
        }
        Some(Self { start, end, width })
    }

    /// The span of a packed value `width` bits wide whose top field holds `field`'s numbers at
    /// `offset`, as [`ReprRange::from_top_field`].
    #[inline]
    #[must_use]
    const fn from_top_field(field: Self, offset: u32, width: u32) -> Self {
        let layout = field.field_layout();
        let field_end = offset.saturating_add(layout.width);
        let (first, last) = if layout.signed {
            (
                sign_extend(field.start, field.width).cast_unsigned(),
                sign_extend(field.end, field.width).cast_unsigned(),
            )
        } else if field.wraps() && field_end < width {
            // An unsigned field that wraps takes its repr's every bit, and with zeros above it,
            // the value's numbers lie on both sides of a run that no one span leaves out.
            return Self { start: 0, end: mask(field_end), width };
        } else {
            (field.start, field.end)
        };
        let largest = mask(width);
        Self {
            start: first.unbounded_shl(offset) & largest,
            end: (last.unbounded_shl(offset) | mask(offset)) & largest,
            width,
        }
    }
}

/// Sorts `values` ascending, in place: one pass where they ascend already, as a fieldless enum's
/// implicit discriminants do, else a heapsort, whose `k log k` steps sort 16,384 descending values
/// within const eval's step limit, where an insertion sort's run out before 512.
const fn sort(values: &mut [u128]) {
    if is_ascending(values) {
        return;
    }
    let count = values.len();
    let mut root = count.wrapping_div(2);
    while root > 0 {
        root = root.wrapping_sub(1);
        sift_down(values, root, count);
    }
    let mut end = count;
    while end > 1 {
        end = end.wrapping_sub(1);
        values.swap(0, end);
        sift_down(values, 0, end);
    }
}

/// Whether `values` ascend.
const fn is_ascending(values: &[u128]) -> bool {
    let mut rest = values;
    while let [low, after @ ..] = rest {
        if let [high, ..] = after
            && *low > *high
        {
            return false;
        }
        rest = after;
    }
    true
}

/// Moves the value at `root` down the max-heap `values[..end]`, whose subtrees below it are heaps,
/// until no child is above it; `end` lies within `values`.
///
/// It indexes, since through `get` and `swap` the sort runs out of const eval's steps before 8,192
/// descending values.
#[expect(
    clippy::indexing_slicing,
    reason = "each index is below `end`, which lies within `values`"
)]
const fn sift_down(values: &mut [u128], mut root: usize, end: usize) {
    let value = values[root];
    loop {
        // A slice of `u128` holds fewer than `usize::MAX / 16` values, so a child's index never
        // wraps.
        let left = root.wrapping_mul(2).wrapping_add(1);
        if left >= end {
            break;
        }
        let right = left.wrapping_add(1);
        let child = if right < end && values[right] > values[left] { right } else { left };
        if values[child] <= value {
            break;
        }
        values[root] = values[child];
        root = child;
    }
    values[root] = value;
}

/// The low `width` bits set: none for 0, all 128 for 128 or more.
#[inline]
#[must_use]
pub(crate) const fn mask(width: u32) -> u128 {
    u128::MAX.unbounded_shr(u128::BITS.saturating_sub(width))
}

/// How many bits `value` needs unsigned: none for zero.
#[inline]
#[must_use]
const fn bit_length(value: u128) -> u32 {
    u128::BITS.wrapping_sub(value.leading_zeros())
}

/// How many bits `value` needs in two's complement: one for 0 and -1.
#[inline]
#[must_use]
const fn signed_bit_length(value: i128) -> u32 {
    // A negative value needs what its complement, which is not negative, needs.
    let magnitude = value ^ value.unbounded_shr(i128::BITS.wrapping_sub(1));
    bit_length(magnitude.cast_unsigned()).wrapping_add(1)
}

/// The two's complement value of the low `width` bits of `bits`.
#[inline]
#[must_use]
pub(crate) const fn sign_extend(bits: u128, width: u32) -> i128 {
    let above = u128::BITS.saturating_sub(width);
    bits.unbounded_shl(above).cast_signed().unbounded_shr(above)
}

#[cfg(test)]
mod tests;
