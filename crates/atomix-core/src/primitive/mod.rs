//! The primitives atomic cells hold, and which of their operations need no compare-exchange loop.
//!
//! Library code names `core`'s and loom's atomic types here and nowhere else.

use core::marker::Destruct;
use core::panic::RefUnwindSafe;
use core::sync::atomic::Ordering as CoreOrdering;

use crate::atomic::FieldPath;

#[cfg(wide)]
mod double;
mod narrow;
#[cfg(wide)]
mod wide;

#[cfg(wide)]
pub use self::double::{DoubleWord, Word};
pub(crate) use self::narrow::{address, subtract_tags};

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
}

/// Declares `Primitive`, whose cell access is const exactly where `CellAccess` is.
macro_rules! primitive {
    ($($cell:tt)+) => {
        /// A primitive an atomic cell holds: `bool`, an integer, a `*mut T`, or, where the target
        /// has 16-byte atomics, a `DoubleWord` of two pointers or integers.
        ///
        /// Every primitive has a compare-exchange; [`Load`], [`Store`], [`Swap`], [`FetchAdd`],
        /// [`MaskBitwise`], [`BitTest`], [`FetchBitwise`] and [`MinMax`] say which other operations
        /// the target runs without a compare-exchange loop.
        #[diagnostic::on_unimplemented(
            message = "`{Self}` is not a primitive an atomic cell holds on this target",
            label = "expected `bool`, an integer or a `*mut T`"
        )]
        #[cfg_attr(
            x86_64_without_cmpxchg16b,
            diagnostic::on_unimplemented(
                note = "a 128-bit integer needs `cmpxchg16b` on x86_64: build with `-C target-cpu=x86-64-v2` or newer"
            )
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
            /// The bits a packed value's fields lie in: an integer's unsigned bits, `bool`'s 0 or
            /// 1, or a pointer's address, where a pointer word keeps its tags.
            ///
            /// A constant reads no pointer's address but null's.
            #[doc(hidden)]
            fn packed_bits(self) -> u128;
            /// The value whose packed bits are `bits`: the integer of them, or this pointer at the
            /// address `bits`, its provenance kept.
            #[doc(hidden)]
            #[must_use]
            fn with_packed_bits(self, bits: u128) -> Self;
            /// The value with the packed bits set in `mask` cleared: a pointer's, at run time,
            /// through the pointer's `mask`, which tells LLVM they are clear.
            ///
            /// A constant reads no pointer's address but null's.
            #[doc(hidden)]
            #[inline]
            #[must_use]
            fn clear_packed_bits(self, mask: u128) -> Self {
                self.with_packed_bits(self.packed_bits() & !mask)
            }
        }
    };
}

#[cfg(not(loom))]
primitive!([const] CellAccess);
#[cfg(loom)]
primitive!(CellAccess);

/// A primitive whose cell holds it as its own type lays it out, so the place the cell lends is the
/// primitive's: every primitive but a `DoubleWord`.
///
/// [`Atomic::as_ptr`](crate::Atomic::as_ptr), [`Atomic::from_ptr`](crate::Atomic::from_ptr),
/// [`Atomic::get_mut`](crate::Atomic::get_mut), [`Atomic::from_mut`](crate::Atomic::from_mut) and
/// the slice conversions need it. A double word's cell holds its pointers' addresses, their
/// provenance exposed, so a pointer read from its place would have no provenance, and one written
/// there unexposed none for a load to take back: its words are reached through its atomic
/// operations alone.
///
/// # Safety
/// Outside loom, the cell has a `Self`'s size and bit validity, and holds the `Self` at its start
/// as `Self` lays it out, which [`get_mut`](Self::get_mut) and [`as_ptr`](Self::as_ptr) reach: a
/// place of a `Self` aligned as the cell is, is a place of the cell. It promises nothing of
/// alignment, which code that takes a `Self`'s place as the cell's checks. Under loom it promises
/// nothing, and nothing that relies on it exists there.
#[diagnostic::on_unimplemented(
    message = "`{Self}` lends no place of its own",
    label = "expected a primitive other than a `DoubleWord`",
    note = "a double word's cell holds each pointer's address, its provenance exposed, so a pointer read through a place would have none: reach two words through `load`, `store` and the compare-exchanges"
)]
#[expect(unsafe_code, reason = "`Atomic::from_mut` takes a value's place as the cell's")]
pub impl(crate) const unsafe trait RawAccess: [const] Primitive {
    /// The value's place, through exclusive access.
    #[cfg(not(loom))]
    #[doc(hidden)]
    fn get_mut(cell: &mut Self::Cell) -> &mut Self;
    /// The value's address.
    #[cfg(not(loom))]
    #[doc(hidden)]
    fn as_ptr(cell: &Self::Cell) -> *mut Self;
}

/// A primitive whose bits are its whole value: `bool` or an integer, never a pointer, whose
/// provenance its bits do not hold.
// `Primitive::IS_BITS_EXACT` is true exactly for the primitives that implement this.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a primitive whose bits are its whole value",
    label = "expected `bool` or an integer",
    note = "a pointer's bits do not hold its provenance: for a pure-read load of a pointer, call `load`"
)]
pub impl(crate) const trait ExactBits: [const] Primitive {
    /// The value as unsigned bits: an integer's two's complement, `bool`'s 0 or 1.
    #[doc(hidden)]
    fn to_bits(self) -> u128;
}

/// The compare-exchange every primitive's cell has, and the read that starts a loop of them.
///
/// Every operation here and in the capabilities below takes the `core` spelling of an atomix
/// ordering its caller's bound admits, so `core` never refuses one.
// `Atom::Repr` is bound by this and `Primitive`, and for a repr that is neither, rustc reports only
// this bound, so its diagnostic repeats `Primitive`'s.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a primitive an atomic cell holds on this target",
    label = "expected `bool`, an integer or a `*mut T`"
)]
#[cfg_attr(
    x86_64_without_cmpxchg16b,
    diagnostic::on_unimplemented(
        note = "a 128-bit integer needs `cmpxchg16b` on x86_64: build with `-C target-cpu=x86-64-v2` or newer"
    )
)]
#[doc(hidden)]
pub impl(crate) trait CompareExchange: Primitive {
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

/// A primitive a compare-exchange reads without changing: one of zero for zero, which writes zero
/// over zero alone, and strips no provenance.
///
/// [`Atomic::load_rmw`](crate::Atomic::load_rmw) and a field's `load_rmw` need it, so generic
/// code that reads with one states it: `bool`, an integer, or a `DoubleWord`. An integer's exchange
/// compares every bit, and a double word's cell holds its pointers' addresses, their provenance
/// exposed. A pointer's compares its address alone, so one of null for null would write a null
/// without the provenance of the null it found; a pointer has a [`Load`] instead.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a primitive a compare-exchange reads without changing",
    label = "expected `bool`, an integer or a `DoubleWord`",
    note = "a pointer's compare-exchange compares its address alone, so one of null for null would strip a null's provenance: for a pure-read load of a pointer, call `load`"
)]
pub impl(crate) trait ReadByExchange: CompareExchange {}

// `do_not_recommend`, so a pointer reports this trait's message, not `ExactBits`'.
#[diagnostic::do_not_recommend]
impl<R: ExactBits + CompareExchange> ReadByExchange for R {}

// `do_not_recommend`, so a pointer's refusal lists no double word's impl as a fix.
#[cfg(wide)]
#[diagnostic::do_not_recommend]
impl<A: Word, B: Word> ReadByExchange for DoubleWord<A, B> {}

/// A primitive whose atomic load never writes.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no pure-read atomic load on this target",
    label = "this load would be a read-modify-write: it writes the line and faults on read-only pages",
    note = "a 128-bit load is one instruction with FEAT_LSE2 (aarch64: `-C target-cpu` of a CPU with LSE2) or AVX (x86_64: `-C target-cpu=x86-64-v3`)",
    note = "to accept a load that writes the cache line, call `load_rmw`"
)]
pub impl(crate) trait Load: CompareExchange {
    /// Reads the cell.
    #[doc(hidden)]
    fn load(cell: &Self::Cell, order: CoreOrdering) -> Self;
}

/// A primitive whose atomic store needs no compare-exchange loop.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic store without a compare-exchange loop on this target",
    label = "this store would be a compare-exchange loop",
    note = "a 128-bit store is one instruction with FEAT_LSE2 (aarch64: `-C target-cpu` of a CPU with LSE2) or AVX (x86_64: `-C target-cpu=x86-64-v3`)",
    note = "to accept a compare-exchange loop, call `store_rmw`"
)]
pub impl(crate) trait Store: CompareExchange {
    /// Writes `value` to the cell.
    #[doc(hidden)]
    fn store(cell: &Self::Cell, value: Self, order: CoreOrdering);
}

/// A primitive whose atomic exchange needs no compare-exchange loop.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic exchange without a compare-exchange loop on this target",
    label = "this exchange would be a compare-exchange loop",
    note = "atomix has no 128-bit exchange: call `update` with `|_| new`, a compare-exchange loop that returns the value before"
)]
pub impl(crate) trait Swap: CompareExchange {
    /// Writes `value` to the cell, and returns the value before.
    #[doc(hidden)]
    fn swap(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self;
}

/// A primitive whose wrapping add and subtract need no compare-exchange loop: `lock xadd` on
/// `x86_64`, `ldadd` on `aarch64` (without LSE, an outline call or an LL/SC pair).
///
/// A field's `fetch_add` and `fetch_sub` ask it of the container's repr, so generic code over a
/// field states it; a value's [`AtomAdd`](crate::AtomAdd) implies it.
///
/// # Examples
/// [`#[derive(Atom)]`'s example][generic] states it in generic code over a field.
///
/// [generic]: https://docs.rs/atomix-rs/latest/atomix/derive.Atom.html#changing-a-field-through-generic-code
// A value's `AtomAdd` names this bound first, so only a field's add, whose value may be a 128-bit
// container's field, reaches this message.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic add without a compare-exchange loop on this target",
    label = "this would be a compare-exchange loop",
    note = "atomix has no 128-bit add",
    note = "to accept a compare-exchange loop, call `update`"
)]
pub impl(crate) trait FetchAdd: CompareExchange {
    /// Adds `delta`, wrapping, and returns the value before.
    #[doc(hidden)]
    fn fetch_add(cell: &Self::Cell, delta: Self, order: CoreOrdering) -> Self;
    /// Subtracts `delta`, wrapping, and returns the value before.
    #[doc(hidden)]
    fn fetch_sub(cell: &Self::Cell, delta: Self, order: CoreOrdering) -> Self;
}

/// A pointer primitive whose offsets, by elements or by bytes, need no compare-exchange loop:
/// `lock xadd` on `x86_64`, `ldadd` on `aarch64` (without LSE, an outline call or an LL/SC pair).
///
/// Each keeps the pointer's provenance. Loom's pointer cell has no arithmetic, so under loom each
/// offset is a compare-exchange loop.
pub(crate) trait PtrOffset: CompareExchange {
    /// Offsets the pointer by `count` elements, wrapping, and returns the pointer before.
    fn fetch_ptr_add(cell: &Self::Cell, count: usize, order: CoreOrdering) -> Self;
    /// Offsets the pointer back by `count` elements, wrapping, and returns the pointer before.
    fn fetch_ptr_sub(cell: &Self::Cell, count: usize, order: CoreOrdering) -> Self;
    /// Offsets the pointer by `bytes`, wrapping, and returns the pointer before.
    fn fetch_byte_add(cell: &Self::Cell, bytes: usize, order: CoreOrdering) -> Self;
    /// Offsets the pointer back by `bytes`, wrapping, and returns the pointer before.
    fn fetch_byte_sub(cell: &Self::Cell, bytes: usize, order: CoreOrdering) -> Self;
}

/// A primitive whose and, or, xor and not need no compare-exchange loop once an optimized build
/// discards the value before: `lock or` on `x86_64`, `ldset` on `aarch64` (without LSE, an outline
/// call or an LL/SC pair).
#[doc(hidden)]
pub impl(crate) trait Bitwise: CompareExchange {
    /// Applies `& value`, and returns the value before.
    fn fetch_and(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self;
    /// Applies `| value`, and returns the value before.
    fn fetch_or(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self;
    /// Applies `^ value`, and returns the value before.
    fn fetch_xor(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self;
    /// Inverts every bit, and returns the value before.
    fn fetch_not(cell: &Self::Cell, order: CoreOrdering) -> Self;
}

/// A primitive whose bits a mask sets, clears or flips without a compare-exchange loop, once an
/// optimized build discards the value before.
///
/// Each is `lock or`, `lock and` and `lock xor` on `x86_64`, `ldset`, `ldclr` and `ldeor` on
/// `aarch64` (without LSE, an outline call or an LL/SC pair). A field's bitwise operations go
/// through it, each with a mask its path confines to the field, and ask it of the container's repr,
/// so generic code over a field states it, or [`BitTest`], which implies it.
///
/// A pointer's mask is a `usize`, which changes its address and keeps its provenance, as
/// [`AtomicPtr::fetch_or`](core::sync::atomic::AtomicPtr::fetch_or) does: a pointer word's tags
/// change so.
///
/// # Examples
/// [`#[derive(Atom)]`'s example][generic] states [`BitTest`], which implies it, in generic code
/// over a field.
///
/// [generic]: https://docs.rs/atomix-rs/latest/atomix/derive.Atom.html#changing-a-field-through-generic-code
// An integer is its own mask; a pointer's is its address's.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no bitwise operation without a compare-exchange loop on this target",
    label = "this would be a compare-exchange loop",
    note = "atomix has no 128-bit bitwise operation",
    note = "to accept a compare-exchange loop, call `update`"
)]
pub impl(crate) trait MaskBitwise: CompareExchange {
    /// The mask: the integer itself, or a pointer's address.
    #[doc(hidden)]
    type Mask: ExactBits;
    /// The mask of the one bit at `position`, modulo `BITS`, as `wrapping_shl` counts a shift:
    /// `Atomic::bit_set` and its kin pass any position.
    #[doc(hidden)]
    fn bit(position: u32) -> Self::Mask;
    /// Turns on the bits of `mask`, and returns the value before.
    #[doc(hidden)]
    fn fetch_or_mask(cell: &Self::Cell, mask: Self::Mask, order: CoreOrdering) -> Self;
    /// Clears every bit outside `mask`, and returns the value before.
    #[doc(hidden)]
    fn fetch_and_mask(cell: &Self::Cell, mask: Self::Mask, order: CoreOrdering) -> Self;
    /// Flips the bits of `mask`, and returns the value before.
    #[doc(hidden)]
    fn fetch_xor_mask(cell: &Self::Cell, mask: Self::Mask, order: CoreOrdering) -> Self;
}

/// A primitive whose one bit a test-and-set, -clear or -toggle reads back without a
/// compare-exchange loop.
///
/// Each is `lock bts`, `btr` and `btc` on `x86_64`, of 16 bits or more, and `ldset`, `ldclr` and
/// `ldeor` on `aarch64` (without LSE, an outline call or an LL/SC pair). A `bool` field's
/// `test_and_set`, `test_and_clear` and `test_and_toggle` ask it of the container's repr, so
/// generic code over a field states it; an atomic's `bit_set`, `bit_clear` and `bit_toggle` ask
/// it of their own repr.
///
/// On `x86_64` each is an `asm!`, so it stays one instruction wherever the bit goes: LLVM widens
/// its own test of a `fetch_or` into a shift where the bit lands in an `Option`, and loses
/// `lock bts` to a compare-exchange loop.
///
/// # Examples
/// [`#[derive(Atom)]`'s example][generic] states it in generic code over a field.
///
/// [generic]: https://docs.rs/atomix-rs/latest/atomix/derive.Atom.html#changing-a-field-through-generic-code
// Above the target-neutral attribute, so the target's notes come before its fallback.
#[cfg_attr(
    target_arch = "x86_64",
    diagnostic::on_unimplemented(
        note = "x86_64's `lock bts`, `btr` and `btc` take 16, 32 or 64 bits: an 8-bit or a 128-bit word's bit would be a compare-exchange loop",
        note = "for an 8-bit word, use a 16-bit one, `AtomicU16` or `#[atom(repr = u16)]`, or call `or`, `and` or `xor`, or a field's `set`, `clear` or `toggle`, which discard the bit before"
    )
)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no bit test-and-set without a compare-exchange loop on this target",
    label = "this would be a compare-exchange loop",
    note = "to accept a compare-exchange loop, call `update`"
)]
pub impl(crate) trait BitTest: MaskBitwise {
    /// Turns on the bit at `position`, modulo `BITS`, and returns it before.
    #[doc(hidden)]
    #[inline]
    fn test_and_set_bit(cell: &Self::Cell, position: u32, order: CoreOrdering) -> bool {
        let bit = Self::bit(position);
        has_bit(Self::fetch_or_mask(cell, bit, order), bit)
    }
    /// Turns off the bit at `position`, modulo `BITS`, and returns it before.
    #[doc(hidden)]
    #[inline]
    fn test_and_clear_bit(cell: &Self::Cell, position: u32, order: CoreOrdering) -> bool {
        let bit = Self::bit(position);
        let others = Self::Mask::from_bits(!bit.to_bits());
        has_bit(Self::fetch_and_mask(cell, others, order), bit)
    }
    /// Inverts the bit at `position`, modulo `BITS`, and returns it before.
    #[doc(hidden)]
    #[inline]
    fn test_and_toggle_bit(cell: &Self::Cell, position: u32, order: CoreOrdering) -> bool {
        let bit = Self::bit(position);
        has_bit(Self::fetch_xor_mask(cell, bit, order), bit)
    }
    /// Turns on the bit of `P`, a one-bit field of a container whose repr is `Self`, and returns
    /// it before: [`test_and_set_bit`](Self::test_and_set_bit) at a position the compiler knows.
    #[doc(hidden)]
    #[inline]
    fn test_and_set_field<P: FieldPath>(cell: &Self::Cell, order: CoreOrdering) -> bool {
        Self::test_and_set_bit(cell, P::OFFSET, order)
    }
    /// Turns off the bit of `P`, as [`test_and_set_field`](Self::test_and_set_field) turns it on:
    /// [`test_and_clear_bit`](Self::test_and_clear_bit) at a position the compiler knows.
    #[doc(hidden)]
    #[inline]
    fn test_and_clear_field<P: FieldPath>(cell: &Self::Cell, order: CoreOrdering) -> bool {
        Self::test_and_clear_bit(cell, P::OFFSET, order)
    }
    /// Inverts the bit of `P`, as [`test_and_set_field`](Self::test_and_set_field) turns it on:
    /// [`test_and_toggle_bit`](Self::test_and_toggle_bit) at a position the compiler knows.
    #[doc(hidden)]
    #[inline]
    fn test_and_toggle_field<P: FieldPath>(cell: &Self::Cell, order: CoreOrdering) -> bool {
        Self::test_and_toggle_bit(cell, P::OFFSET, order)
    }
}

/// Whether the bit of `mask` is set in `value`.
#[inline]
fn has_bit<R: MaskBitwise>(value: R, mask: R::Mask) -> bool {
    value.packed_bits() & mask.to_bits() != 0
}

/// A primitive whose and, or, xor and not return the value before without a compare-exchange
/// loop: `ldclr`, `ldset` and `ldeor` on `aarch64` (without LSE, an outline call or an LL/SC pair).
///
/// An integer's or a `bool`'s takes a value or a mask; a pointer's, through which a pointer word's
/// tags change, a mask.
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
pub impl(crate) trait FetchBitwise: MaskBitwise {}

/// A primitive whose maximum and minimum, in its own signed or unsigned order, need no
/// compare-exchange loop: `ldsmax`, `ldumin` and the rest on `aarch64`
/// (without LSE, an LL/SC pair).
// Above the target-neutral attribute, so the target's notes come before its fallback.
#[cfg_attr(
    target_arch = "x86_64",
    diagnostic::on_unimplemented(note = "x86_64 has no atomic maximum or minimum")
)]
#[cfg_attr(
    aarch64_code,
    diagnostic::on_unimplemented(
        note = "aarch64's atomic maximum and minimum take an integer of at most 64 bits"
    )
)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no atomic maximum or minimum without a compare-exchange loop on this target",
    label = "this would be a compare-exchange loop",
    note = "to accept a compare-exchange loop, call `update`"
)]
pub impl(crate) trait MinMax: CompareExchange {
    /// Keeps the larger, and returns the value before.
    #[doc(hidden)]
    fn fetch_max(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self;
    /// Keeps the smaller, and returns the value before.
    #[doc(hidden)]
    fn fetch_min(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self;
}
