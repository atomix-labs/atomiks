//! The primitives atomic cells hold, and which of their operations need no compare-exchange loop.
//!
//! Library code names `core`'s and loom's atomic types here and nowhere else.

use core::marker::Destruct;
use core::panic::RefUnwindSafe;
use core::sync::atomic::Ordering as CoreOrdering;

mod native;

/// A primitive's cell, and every access to it that is not an atomic instruction.
///
/// Const off loom, where `core`'s cells are; plain under loom, whose cells are not.
#[doc(hidden)]
pub impl(crate) const trait CellAccess: Sized {
    /// The atomic cell holding the primitive.
    type Cell: Send + Sync + RefUnwindSafe + Unpin + const Destruct;
    /// A cell holding `self`.
    fn into_cell(self) -> Self::Cell;
    /// The value a cell holds, consuming it.
    fn from_cell(cell: Self::Cell) -> Self;
    /// The value, through exclusive access.
    fn get(cell: &mut Self::Cell) -> Self;
    /// Replaces the value, through exclusive access.
    fn set(cell: &mut Self::Cell, value: Self);
    /// The value's place, through exclusive access.
    #[cfg(not(loom))]
    fn get_mut(cell: &mut Self::Cell) -> &mut Self;
    /// The value's address.
    #[cfg(not(loom))]
    fn as_ptr(cell: &Self::Cell) -> *mut Self;
}

/// Declares `Primitive`, whose cell access is const exactly where `CellAccess` is.
macro_rules! primitive {
    ($($cell:tt)+) => {
        /// A primitive an atomic cell holds: `bool`, an integer, or a `*mut T`.
        ///
        /// Every primitive has a compare-exchange; [`Load`], [`Store`], [`Swap`], [`FetchBitwise`]
        /// and [`MinMax`] say which other operations the target runs without a compare-exchange
        /// loop. Only atomiks implements it.
        #[diagnostic::on_unimplemented(
            message = "`{Self}` is not a primitive an atomic cell holds on this target",
            label = "expected `bool`, an integer or a `*mut T`",
            note = "a 128-bit integer needs `cmpxchg16b` on x86_64: build with `-C target-cpu=x86-64-v2` or newer"
        )]
        pub impl(crate) const trait Primitive: Copy + $($cell)+ {
            /// How many bits of value it holds: 1 for `bool`, else its width.
            #[doc(hidden)]
            const BITS: u32;
            /// Whether `is_bits` decides every `bits`; a pointer's decides only zero.
            #[doc(hidden)]
            const IS_BITS_EXACT: bool = true;
            /// The value of the low `BITS` of `bits`; a pointer built so has no provenance.
            #[doc(hidden)]
            fn from_bits(bits: u128) -> Self;
            /// Whether the value's unsigned bits are `bits`.
            ///
            /// A pointer's address is unreadable in const, so only a null pointer and zero match.
            #[doc(hidden)]
            fn is_bits(self, bits: u128) -> bool;
        }
    };
}

#[cfg(not(loom))]
primitive!([const] CellAccess);
#[cfg(loom)]
primitive!(CellAccess);

/// An integer primitive, or `bool`: one whose bits pack into a field of a wider word.
///
/// Only atomiks implements it.
pub impl(crate) const trait Integer: [const] Primitive {
    /// The value as unsigned bits: an integer's two's complement, `bool`'s 0 or 1.
    #[doc(hidden)]
    fn to_bits(self) -> u128;
}

/// The operations every primitive's cell has.
///
/// Every operation here and in the capabilities below takes the `core` spelling of an atomiks
/// ordering its caller's bound admits, so `core` never refuses one.
#[doc(hidden)]
pub impl(crate) trait CellOps: Primitive {
    /// The first read of an update loop: a load where there is one, else a compare-exchange.
    fn read_for_rmw(cell: &Self::Cell, order: CoreOrdering) -> Self;
    /// Writes `new` if the cell holds `current`; returns the value before, `Ok` if `current`.
    fn compare_exchange(
        cell: &Self::Cell, current: Self, new: Self, success: CoreOrdering, failure: CoreOrdering,
    ) -> Result<Self, Self>;
    /// As `compare_exchange`, but may fail while the cell holds `current`.
    fn compare_exchange_weak(
        cell: &Self::Cell, current: Self, new: Self, success: CoreOrdering, failure: CoreOrdering,
    ) -> Result<Self, Self>;
}

/// A primitive whose atomic load never writes.
///
/// Only atomiks implements it.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no pure-read atomic load on this target",
    label = "this load would be a read-modify-write: it writes the line and faults on read-only pages",
    note = "a 128-bit load is one instruction with FEAT_LSE2 (aarch64: `-C target-cpu=neoverse-v1` or newer) or AVX (x86_64: `-C target-cpu=x86-64-v3`)",
    note = "to accept a load that writes the cache line, call `load_rmw`"
)]
pub impl(crate) trait Load: CellOps {
    /// Reads the cell.
    #[doc(hidden)]
    fn load(cell: &Self::Cell, order: CoreOrdering) -> Self;
}

/// A primitive whose atomic store needs no compare-exchange loop.
///
/// Only atomiks implements it.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic store without a compare-exchange loop on this target",
    label = "this store would be a compare-exchange loop",
    note = "a 128-bit store is one instruction with FEAT_LSE2 (aarch64: `-C target-cpu=neoverse-v1` or newer) or AVX (x86_64: `-C target-cpu=x86-64-v3`)",
    note = "to accept a compare-exchange loop, call `store_rmw`"
)]
pub impl(crate) trait Store: CellOps {
    /// Writes `value` to the cell.
    #[doc(hidden)]
    fn store(cell: &Self::Cell, value: Self, order: CoreOrdering);
}

/// A primitive whose atomic exchange needs no compare-exchange loop.
///
/// Only atomiks implements it.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic exchange without a compare-exchange loop on this target",
    label = "this exchange would be a compare-exchange loop",
    note = "a 128-bit exchange is a compare-exchange loop on every supported CPU: call `update` with `|_| new`, which returns the value before"
)]
pub impl(crate) trait Swap: CellOps {
    /// Writes `value` to the cell, and returns the value before.
    #[doc(hidden)]
    fn swap(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self;
}

/// A primitive whose wrapping add and subtract need no compare-exchange loop: `lock xadd` on
/// `x86_64`, `ldadd` on `aarch64` (without LSE, an outline call or an LL/SC pair).
#[doc(hidden)]
pub impl(crate) trait FetchAdd: CellOps {
    /// Adds `delta`, wrapping, and returns the value before.
    fn fetch_add(cell: &Self::Cell, delta: Self, order: CoreOrdering) -> Self;
    /// Subtracts `delta`, wrapping, and returns the value before.
    fn fetch_sub(cell: &Self::Cell, delta: Self, order: CoreOrdering) -> Self;
}

/// A primitive whose and, or, xor and not need no compare-exchange loop once an optimized build
/// discards the value before: `lock or` on `x86_64`, `ldset` on `aarch64` (without LSE, an outline
/// call or an LL/SC pair).
#[doc(hidden)]
pub impl(crate) trait Bitwise: CellOps {
    /// Applies `& value`, and returns the value before.
    fn fetch_and(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self;
    /// Applies `| value`, and returns the value before.
    fn fetch_or(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self;
    /// Applies `^ value`, and returns the value before.
    fn fetch_xor(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self;
    /// Inverts every bit, and returns the value before.
    fn fetch_not(cell: &Self::Cell, order: CoreOrdering) -> Self;
}

/// A primitive whose and, or, xor and not return the value before without a compare-exchange
/// loop: `ldclr`, `ldset` and `ldeor` on `aarch64` (without LSE, an outline call or an LL/SC pair).
///
/// Only atomiks implements it.
// Above the target-neutral attribute, so the target's notes come before its fallback.
#[cfg_attr(
    target_arch = "x86_64",
    diagnostic::on_unimplemented(
        note = "x86_64's `lock and`, `lock or` and `lock xor` cannot return the value before",
        note = "`and`, `or`, `xor` and `not` discard it, and are one instruction in an optimized build"
    )
)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no `fetch_and`, `fetch_or`, `fetch_xor` or `fetch_not` without a compare-exchange loop on this target",
    label = "this would be a compare-exchange loop",
    note = "to accept a compare-exchange loop, call `update`"
)]
pub impl(crate) trait FetchBitwise: Bitwise {}

/// A primitive whose maximum and minimum, in its own signed or unsigned order, need no
/// compare-exchange loop: `ldsmax`, `ldumin` and the rest on `aarch64`
/// (without LSE, an LL/SC pair).
///
/// Only atomiks implements it.
// Above the target-neutral attribute, so the target's notes come before its fallback.
#[cfg_attr(
    target_arch = "x86_64",
    diagnostic::on_unimplemented(note = "x86_64 has no atomic maximum or minimum")
)]
#[cfg_attr(
    target_arch = "aarch64",
    diagnostic::on_unimplemented(
        note = "aarch64's atomic maximum and minimum take an integer of at most 64 bits"
    )
)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic maximum or minimum without a compare-exchange loop on this target",
    label = "this would be a compare-exchange loop",
    note = "to accept a compare-exchange loop, call `update`"
)]
pub impl(crate) trait MinMax: CellOps {
    /// Keeps the larger, and returns the value before.
    #[doc(hidden)]
    fn fetch_max(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self;
    /// Keeps the smaller, and returns the value before.
    #[doc(hidden)]
    fn fetch_min(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self;
}
