//! The ranged integers, [`RangedU8`] to [`RangedIsize`]: each an integer held to its range.

use core::cmp::Ordering;
use core::fmt::{self, Binary, Debug, Display, Formatter, LowerHex, Octal, UpperHex};
use core::hash::{Hash, Hasher};
use core::iter::Step;
use core::marker::StructuralPartialEq;
use core::mem::transmute_neo;
use core::pattern_type;
use core::str::FromStr;

use crate::errors::{ParseRangeError, RangeError};

/// Calls `$macro!($name($int))` for each ranged integer, `$name`, over its integer, `$int`: the
/// one list of the twelve, which defines them and gives them each integration's traits.
macro_rules! each_ranged_integer {
    ($macro:ident) => {
        $macro!(RangedU8(u8));
        $macro!(RangedU16(u16));
        $macro!(RangedU32(u32));
        $macro!(RangedU64(u64));
        $macro!(RangedU128(u128));
        $macro!(RangedUsize(usize));
        $macro!(RangedI8(i8));
        $macro!(RangedI16(i16));
        $macro!(RangedI32(i32));
        $macro!(RangedI64(i64));
        $macro!(RangedI128(i128));
        $macro!(RangedIsize(isize));
    };
}

// For the integrations, which name it by its path.
#[cfg(any(
    feature = "arbitrary",
    feature = "bytemuck",
    feature = "deranged-05",
    feature = "serde"
))]
pub(crate) use each_ranged_integer;

/// Defines a ranged integer, `$name`, over its integer, `$int`, one section at a time.
macro_rules! ranged {
    ($name:ident($int:ident)) => {
        ranged!(@type $name $int);
        ranged!(@construct $name $int);
        ranged!(@arithmetic $name $int);
        // The traits are written over `get()`: the field's pattern type implements only `Clone`
        // and `Copy`, so no derive, std's or derive_more's, can read it.
        ranged!(@compare $name $int);
        ranged!(@format $name $int: Debug, Display, Binary, Octal, LowerHex, UpperHex);
        ranged!(@convert $name $int);
        ranged!(@step $name $int);
    };

    // The type, its docs, and the niche its range leaves an `Option`.
    (@type $name:ident $int:ident) => {
        #[doc = concat!("A `", stringify!($int), "` held to `MIN..=MAX`.")]
        ///
        /// Built as core's [`NonZero`] is: the private field is a pattern type, so the range is a
        /// validity invariant, which Miri checks, and where the range leaves an integer out, an
        /// `Option` keeps its `None` there, in the integer's own width. As with `NonZero`, an
        /// operation is infallible only where the range is closed under it: [`min`](Ord::min),
        /// [`max`](Ord::max) and [`clamp`](Ord::clamp) are, and arithmetic is `checked_` or
        /// `saturating_`. There is no `Default`, since zero may lie outside the range.
        ///
        /// Wherever its integer is an [`Atom`](crate::Atom), so is it: an [`Atomic`](crate::Atomic)
        /// stores it as its integer, and an `Option`'s `None` as an integer outside the range, so
        /// an atomic of a full range's `Option` is refused. It has [`AtomOrd`](crate::AtomOrd), as
        /// the larger or smaller of two values in the range is in it, but neither
        /// [`AtomAdd`](crate::AtomAdd) nor [`AtomBitwise`](crate::AtomBitwise): a sum or an or of
        /// two can leave it.
        ///
        #[doc = concat!(
            "`MAX` defaults to [`", stringify!($int), "::MAX`]. ",
            "A `MIN` above `MAX` names no type: rustc refuses it where it is used, in a message ",
            "that quotes the bounds but names no file or line, so search the code for them. ",
            "A crate that uses the type needs no `#![feature]`."
        )]
        ///
        /// # Examples
        /// ```
        /// # extern crate atomiks_core as atomiks;
        #[doc = concat!("use atomiks::{ParseRangeError, ", stringify!($name), "};")]
        ///
        /// /// How many levels of an order book a feed sends, 1 to 10.
        #[doc = concat!("type Depth = ", stringify!($name), "<1, 10>;")]
        ///
        /// let depth = Depth::new(4).expect("4 levels is from 1 to 10");
        /// assert_eq!(depth.get(), 4, "the integer back");
        #[doc = concat!(
            "assert_eq!(size_of::<Option<Depth>>(), size_of::<", stringify!($int), ">(), ",
            "\"an `Option` in the integer's width\");"
        )]
        /// assert_eq!(depth.checked_add(7), None, "11 levels, past 10");
        /// assert_eq!(depth.saturating_add(7), Depth::MAX, "or 10, saturating");
        /// assert_eq!("10".parse::<Depth>()?, Depth::MAX, "and 10 parsed from text");
        /// let refused = Depth::try_from(11).expect_err("11 is past 10");
        /// assert_eq!(refused.to_string(), "range error: 11 is not in 1..=10", "refused by range");
        #[doc = concat!("# Ok::<(), ParseRangeError<", stringify!($int), ">>(())")]
        /// ```
        ///
        /// [`NonZero`]: core::num::NonZero
        #[repr(transparent)]
        #[derive(Clone, Copy)]
        pub struct $name<const MIN: $int, const MAX: $int = { $int::MAX }>(
            /// The integer, as a pattern type whose values are those from `MIN` to `MAX`.
            pattern_type!($int is MIN..=MAX),
        );

        const _: () = assert!(
            size_of::<Option<$name<1>>>() == size_of::<$int>(),
            "an `Option` keeps its `None` outside the range, in the integer's width"
        );
    };

    // Building a value, and reading its integer back.
    (@construct $name:ident $int:ident) => {
        impl<const MIN: $int, const MAX: $int> $name<MIN, MAX> {
            /// The smallest value, `MIN`.
            pub const MIN: Self = Self::new_saturating(MIN);
            /// The largest value, `MAX`.
            pub const MAX: Self = Self::new_saturating(MAX);

            /// The value of `integer`, or `None` where it lies outside the range.
            #[expect(unsafe_code, reason = "builds the value once its range is checked")]
            #[inline]
            #[must_use]
            pub const fn new(integer: $int) -> Option<Self> {
                if MIN <= integer && integer <= MAX {
                    // SAFETY: `integer` lies from `MIN` to `MAX`, just checked.
                    Some(unsafe { Self::new_unchecked(integer) })
                } else {
                    None
                }
            }

            /// The value of `integer`, unchecked.
            ///
            /// # Safety
            /// `integer` lies from `MIN` to `MAX`.
            #[expect(unsafe_code, reason = "a pattern type is built only by a transmute")]
            #[inline]
            #[must_use]
            pub const unsafe fn new_unchecked(integer: $int) -> Self {
                debug_assert!(MIN <= integer && integer <= MAX, "an integer outside `MIN..=MAX`");
                // SAFETY: `Self` is a transparent pattern type over the integer, so of its size,
                // and the caller's `integer` lies in the pattern's range, `MIN` to `MAX`.
                unsafe { transmute_neo::<$int, Self>(integer) }
            }

            /// The value in the range nearest `integer`: `MIN` below it, `MAX` above it.
            #[expect(unsafe_code, reason = "builds the value once it is moved into the range")]
            #[inline]
            #[must_use]
            pub const fn new_saturating(integer: $int) -> Self {
                let nearest = if integer < MIN {
                    MIN
                } else if integer > MAX {
                    MAX
                } else {
                    integer
                };
                // SAFETY: `nearest` lies from `MIN` to `MAX`, since `MIN` is at most `MAX` wherever
                // `Self` exists: rustc refuses a pattern type whose range wraps.
                unsafe { Self::new_unchecked(nearest) }
            }

            /// The integer, from `MIN` to `MAX`.
            #[expect(unsafe_code, reason = "a pattern type is read only by a transmute")]
            #[inline]
            #[must_use]
            pub const fn get(self) -> $int {
                // SAFETY: `Self` is a transparent pattern type over the integer, so of its size,
                // and each of its values is one of the integer's.
                unsafe { transmute_neo::<Self, $int>(self) }
            }
        }
    };

    // Arithmetic, which can leave the range, so checked or saturating.
    (@arithmetic $name:ident $int:ident) => {
        impl<const MIN: $int, const MAX: $int> $name<MIN, MAX> {
            /// Adds `rhs`, or `None` where the sum lies outside the range.
            #[inline]
            #[must_use]
            pub const fn checked_add(self, rhs: $int) -> Option<Self> {
                self.get().checked_add(rhs).and_then(Self::new)
            }

            /// Subtracts `rhs`, or `None` where the difference lies outside the range.
            #[inline]
            #[must_use]
            pub const fn checked_sub(self, rhs: $int) -> Option<Self> {
                self.get().checked_sub(rhs).and_then(Self::new)
            }

            /// Multiplies by `rhs`, or `None` where the product lies outside the range.
            #[inline]
            #[must_use]
            pub const fn checked_mul(self, rhs: $int) -> Option<Self> {
                self.get().checked_mul(rhs).and_then(Self::new)
            }

            /// Adds `rhs`, saturating at `MIN` and `MAX`.
            #[inline]
            #[must_use]
            pub const fn saturating_add(self, rhs: $int) -> Self {
                Self::new_saturating(self.get().saturating_add(rhs))
            }

            /// Subtracts `rhs`, saturating at `MIN` and `MAX`.
            #[inline]
            #[must_use]
            pub const fn saturating_sub(self, rhs: $int) -> Self {
                Self::new_saturating(self.get().saturating_sub(rhs))
            }

            /// Multiplies by `rhs`, saturating at `MIN` and `MAX`.
            #[inline]
            #[must_use]
            pub const fn saturating_mul(self, rhs: $int) -> Self {
                Self::new_saturating(self.get().saturating_mul(rhs))
            }
        }
    };

    // Equality, order and hashing, the integer's, within one range.
    (@compare $name:ident $int:ident) => {
        impl<const MIN: $int, const MAX: $int> PartialEq for $name<MIN, MAX> {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.get() == other.get()
            }
        }

        impl<const MIN: $int, const MAX: $int> Eq for $name<MIN, MAX> {}

        // A constant is a pattern only where its `PartialEq` is structural, as a derived one is.
        impl<const MIN: $int, const MAX: $int> StructuralPartialEq for $name<MIN, MAX> {}

        impl<const MIN: $int, const MAX: $int> PartialOrd for $name<MIN, MAX> {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl<const MIN: $int, const MAX: $int> Ord for $name<MIN, MAX> {
            #[inline]
            fn cmp(&self, other: &Self) -> Ordering {
                self.get().cmp(&other.get())
            }
        }

        impl<const MIN: $int, const MAX: $int> Hash for $name<MIN, MAX> {
            #[inline]
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.get().hash(state);
            }
        }
    };

    // Each formatting trait, the integer's.
    (@format $name:ident $int:ident: $($format:ident),+) => {$(
        impl<const MIN: $int, const MAX: $int> $format for $name<MIN, MAX> {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                $format::fmt(&self.get(), formatter)
            }
        }
    )+};

    // To the integer, and from it or from text, checked against the range.
    (@convert $name:ident $int:ident) => {
        impl<const MIN: $int, const MAX: $int> From<$name<MIN, MAX>> for $int {
            #[inline]
            fn from(value: $name<MIN, MAX>) -> Self {
                value.get()
            }
        }

        impl<const MIN: $int, const MAX: $int> TryFrom<$int> for $name<MIN, MAX> {
            type Error = RangeError<$int>;

            #[inline]
            fn try_from(integer: $int) -> Result<Self, Self::Error> {
                Self::new(integer).ok_or(RangeError::new(integer, MIN, MAX))
            }
        }

        impl<const MIN: $int, const MAX: $int> FromStr for $name<MIN, MAX> {
            type Err = ParseRangeError<$int>;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                let integer = $int::from_str(text).map_err(ParseRangeError::Integer)?;
                Self::try_from(integer).map_err(ParseRangeError::Range)
            }
        }
    };

    // Steps through the range, so `MIN..=MAX` iterates; a step past either end saturates there.
    (@step $name:ident $int:ident) => {
        impl<const MIN: $int, const MAX: $int> Step for $name<MIN, MAX> {
            #[inline]
            fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
                Step::steps_between(&start.get(), &end.get())
            }

            #[inline]
            fn forward_checked(start: Self, count: usize) -> Option<Self> {
                Step::forward_checked(start.get(), count).and_then(Self::new)
            }

            #[inline]
            fn backward_checked(start: Self, count: usize) -> Option<Self> {
                Step::backward_checked(start.get(), count).and_then(Self::new)
            }

            #[inline]
            fn forward_overflowing(start: Self, count: usize) -> (Self, bool) {
                match Self::forward_checked(start, count) {
                    Some(end) => (end, false),
                    None => (Self::MAX, true),
                }
            }

            #[inline]
            fn backward_overflowing(start: Self, count: usize) -> (Self, bool) {
                match Self::backward_checked(start, count) {
                    Some(end) => (end, false),
                    None => (Self::MIN, true),
                }
            }

            #[inline]
            fn forward(start: Self, count: usize) -> Self {
                Self::forward_overflowing(start, count).0
            }

            #[inline]
            fn backward(start: Self, count: usize) -> Self {
                Self::backward_overflowing(start, count).0
            }
        }
    };
}

each_ranged_integer!(ranged);

#[cfg(test)]
mod tests;
