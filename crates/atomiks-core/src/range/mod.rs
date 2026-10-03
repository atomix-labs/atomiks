//! [`ReprRange`], the reprs a value takes, and the math of a range of numbers of any width that
//! wraps through zero.

use core::any::type_name;
use core::fmt;
use core::marker::PhantomData;

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

    /// The smallest range that holds this one and `bits`, a repr's unsigned bits: grown down to
    /// them or up to them, whichever adds fewer reprs, down on a tie.
    #[inline]
    #[must_use]
    pub(crate) const fn including(self, bits: u128) -> Self {
        let span = self.span().including(bits);
        Self::from_bounds(span.start, span.end)
    }

    /// The repr beside the range that `Option`'s `None` takes where the range alone decides, as
    /// [`Span::spare_for_none`] picks it, or `None` where the range is full.
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
        let span = Span { start, end, width: R::BITS };
        if span.is_full() {
            return Self { start: 0, end: span.largest(), marker: PhantomData };
        }
        Self { start, end, marker: PhantomData }
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

    /// The number `Option`'s `None` takes beside the span, or `None` where the span is full.
    ///
    /// Zero where it is beside the span; else, where the span starts at zero, the number after its
    /// end; else whichever of the number before its start and the one after its end leaves the
    /// narrower field, the one before on a tie.
    #[inline]
    #[must_use]
    const fn spare_for_none(self) -> Option<u128> {
        if self.is_full() {
            return None;
        }
        // Not full, a span that starts at 1 or ends at the largest number does not wrap.
        if self.start == 1 || self.end == self.largest() {
            return Some(0);
        }
        let above = self.end.wrapping_add(1);
        if self.start == 0 {
            return Some(above);
        }
        let below = self.start.wrapping_sub(1);
        let narrower_above =
            Self { end: above, ..self }.field_width() < Self { start: below, ..self }.field_width();
        Some(if narrower_above { above } else { below })
    }

    /// The fewest bits a field takes to hold each number of the span: unsigned, or two's
    /// complement where that is narrower; the whole width where the span wraps both through zero
    /// and through the signed numbers' ends.
    #[inline]
    #[must_use]
    const fn field_width(self) -> u32 {
        let first = sign_extend(self.start, self.width);
        let last = sign_extend(self.end, self.width);
        let signed = if first > last {
            self.width
        } else {
            let (low, high) = (signed_bit_length(first), signed_bit_length(last));
            if low > high { low } else { high }
        };
        let unsigned = if self.wraps() { self.width } else { bit_length(self.end) };
        if signed < unsigned { signed } else { unsigned }
    }
}

/// The low `width` bits set: none for 0, all 128 for 128 or more.
#[inline]
#[must_use]
const fn mask(width: u32) -> u128 {
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
const fn sign_extend(bits: u128, width: u32) -> i128 {
    let above = u128::BITS.saturating_sub(width);
    bits.unbounded_shl(above).cast_signed().unbounded_shr(above)
}

#[cfg(test)]
mod tests;
