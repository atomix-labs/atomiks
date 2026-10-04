//! bytemuck's `Zeroable` for an atomic whose zero repr decodes, and `NoUninit`,
//! `CheckedBitPattern` and `Contiguous` for each ranged integer.
//!
//! An atomic has none of the traits that lend or read its bytes, `Pod`, `AnyBitPattern`,
//! `NoUninit` or `CheckedBitPattern`: each needs `Copy`, which no cell is, since its shared
//! reference writes.

#![expect(unsafe_code, reason = "each of bytemuck's traits is an unsafe promise of layout")]

#[cfg(not(loom))]
use bytemuck::Zeroable;
use bytemuck::{CheckedBitPattern, Contiguous, NoUninit};

#[cfg(not(loom))]
use crate::atom::Atom;
#[cfg(not(loom))]
use crate::atomic::Atomic;
use crate::ranged::{self, each_ranged_integer};
#[cfg(not(loom))]
use crate::validity::{Validity, ZeroValid};

/// All zeros, where the zero repr decodes: `0`, `false`, `'\0'`, a null pointer, or the `None` of
/// a `NonZero` or a `NonNull`.
///
/// Not under loom, whose cells are not plain memory.
// SAFETY: all-zero bytes are a cell holding the zero repr, since core's cells, `Opaque` over one,
// and `Wide` each hold their bits as plain memory, and the marker has no bytes. That repr decodes,
// as `ZeroValid` says, so the field INVARIANT holds.
#[cfg(not(loom))]
unsafe impl<T: Atom> Zeroable for Atomic<T> where T::Validity: Validity<ZeroValidity = ZeroValid> {}

/// Implements `NoUninit`, `CheckedBitPattern` and `Contiguous` for a ranged integer, `$name`, over
/// `$int`.
macro_rules! ranged {
    ($name:ident($int:ident)) => {
        #[doc = concat!("Its bytes, a `", stringify!($int), "`'s.")]
        // SAFETY: `repr(transparent)` over a pattern type of the integer, which has the integer's
        // layout: no padding, every byte initialized, and nothing a shared reference writes. It is
        // inhabited, since `MIN` is at most `MAX` wherever it exists: rustc refuses a pattern type
        // whose range wraps.
        unsafe impl<const MIN: $int, const MAX: $int> NoUninit for ranged::$name<MIN, MAX> {}

        #[doc = concat!("A `", stringify!($int), "`'s bytes, checked against the range.")]
        // SAFETY: it has the integer's layout, and each integer from `MIN` to `MAX` is a value.
        unsafe impl<const MIN: $int, const MAX: $int> CheckedBitPattern
            for ranged::$name<MIN, MAX>
        {
            type Bits = $int;

            #[inline]
            fn is_valid_bit_pattern(bits: &$int) -> bool {
                (MIN..=MAX).contains(bits)
            }
        }

        /// The integers from `MIN` to `MAX`.
        // SAFETY: it has the integer's size, which is not zero; `MIN` is at most `MAX`, since rustc
        // refuses a pattern type whose range wraps; each integer from `MIN` to `MAX` is one value,
        // no value is any other integer, and `from_integer` and `into_integer` are bytemuck's own.
        unsafe impl<const MIN: $int, const MAX: $int> Contiguous for ranged::$name<MIN, MAX> {
            type Int = $int;
            const MAX_VALUE: $int = MAX;
            const MIN_VALUE: $int = MIN;
        }
    };
}

each_ranged_integer!(ranged);
