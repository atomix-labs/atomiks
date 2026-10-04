//! Why a ranged integer refused an integer or a text, one type for each question:
//!
//! | Type                | Asks                                 | Returned By |
//! | ------------------- | ------------------------------------ | ----------- |
//! | [`RangeError`]      | is the integer in the range?         | `try_from`  |
//! | [`ParseRangeError`] | is the text an integer in the range? | `from_str`  |
//!
//! `Display` and `Error` are written by hand: atomiks-core depends on no crate outside loom's
//! model, so no derive writes them.

use core::any::type_name;
use core::error::Error;
use core::fmt::{self, Debug, Display, Formatter};
use core::num::ParseIntError;
use core::ops::RangeInclusive;

/// Why an integer is no ranged integer: it lies outside the range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeError<T> {
    /// The integer refused.
    integer: T,
    /// The range's smallest integer, `MIN`.
    min: T,
    /// The range's largest integer, `MAX`.
    max: T,
}

impl<T: Copy> RangeError<T> {
    /// The refusal of `integer`, which lies outside `min..=max`.
    pub(crate) const fn new(integer: T, min: T, max: T) -> Self {
        Self { integer, min, max }
    }

    /// The integer refused.
    #[inline]
    #[must_use]
    pub const fn integer(self) -> T {
        self.integer
    }

    /// The range the integer lies outside, `MIN..=MAX`.
    #[inline]
    #[must_use]
    pub const fn bounds(self) -> RangeInclusive<T> {
        RangeInclusive::new(self.min, self.max)
    }
}

impl<T: Display> Display for RangeError<T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let Self { integer, min, max } = self;
        write!(formatter, "range error: {integer} is not in {min}..={max}")
    }
}

impl<T: Debug + Display> Error for RangeError<T> {}

/// Why a text is no ranged integer: it is no `T`, or a `T` outside the range.
///
/// An enum, so a match takes the cause whole: core's [`ParseIntError`], or the [`RangeError`] with
/// the integer refused. [`source`](Error::source) returns it too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseRangeError<T> {
    /// The text is no `T`: it is empty, holds a character that is no digit, or names an integer
    /// past `T`'s own bounds.
    Integer(ParseIntError),
    /// The text is a `T` outside the range.
    Range(RangeError<T>),
}

impl<T> Display for ParseRangeError<T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(_) => {
                write!(formatter, "parse range error: the text is no `{}`", type_name::<T>())
            },
            Self::Range(_) => {
                formatter.write_str("parse range error: the integer is outside the range")
            },
        }
    }
}

impl<T: Debug + Display + 'static> Error for ParseRangeError<T> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Integer(cause) => Some(cause),
            Self::Range(cause) => Some(cause),
        }
    }
}
