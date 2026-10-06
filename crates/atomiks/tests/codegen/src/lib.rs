//! One operation on a derived value per function, unmangled, for `tests/compiled.rs` to read the
//! assembly of: the load of a packed struct, a fieldless enum and a niche-filling enum, and each
//! field operation through a packed struct's projection.
//!
//! The empty `[workspace]` in its manifest makes it a workspace of its own: the repository's does
//! not list it, and the test builds it alone.

#![no_std]

use atomiks::ordering::{AcqRel, Acquire, Release};
use atomiks::{Atom, AtomBitwise, Atomic, RangedU32};

/// The side of the book an order rests on: one bit, its discriminant.
#[derive(Clone, Copy, Atom)]
pub enum Side {
    /// A buy.
    Bid,
    /// A sell.
    Ask,
}

/// Which way a price moved: two bits, signed.
#[derive(Clone, Copy, Atom)]
#[repr(i8)]
pub enum Sign {
    /// Down.
    Minus = -1,
    /// Neither.
    Flat,
    /// Up.
    Plus,
}

/// A resting quote: 32 bits of quantity, then a bit of side, then one of whether it is live.
#[derive(Clone, Copy, Atom)]
pub struct Quote {
    /// How many.
    pub quantity: u32,
    /// Which side.
    pub side: Side,
    /// Whether it may fill.
    pub live: bool,
}

/// A reading of a sign, missing or stale: the two units take the reprs beside the sign's range.
#[derive(Clone, Copy, Atom)]
pub enum Reading {
    /// No reading yet.
    Missing,
    /// A reading too old to use.
    Stale,
    /// The sign read.
    Present(Sign),
}

/// Flags of an order, each pattern of a byte a value.
#[derive(Clone, Copy, Atom, AtomBitwise)]
pub struct Flags(pub u8);

/// An order: 32 bits of quantity, a bit of side, a bit of whether it is live (bit 33), then flags.
#[derive(Clone, Copy, Atom)]
pub struct Order {
    /// How many.
    pub quantity: u32,
    /// Which side.
    pub side: Side,
    /// Whether it may fill.
    pub live: bool,
    /// Its flags, at bits 34 to 41.
    pub flags: Flags,
}

/// A count in the top half of the word, flags and a bit below it.
#[derive(Clone, Copy, Atom)]
pub struct Counted {
    /// Its flags.
    pub flags: Flags,
    /// Whether it is ready.
    pub ready: bool,
    /// Bits no one uses.
    pub spare: RangedU32<0, 0x7F_FFFF>,
    /// The count, at bits 32 to 63.
    pub count: u32,
}

/// A packed struct a field of another holds.
#[derive(Clone, Copy, Atom)]
pub struct Inner {
    /// Whether it is ready, at bit 8 of `Outer`.
    pub ready: bool,
    /// Its flags.
    pub flags: Flags,
}

/// A byte, the inner struct at bit 8, then a bit.
#[derive(Clone, Copy, Atom)]
pub struct Outer {
    /// A byte.
    pub low: u8,
    /// The inner struct.
    pub inner: Inner,
    /// Whether it is done.
    pub done: bool,
}

#[unsafe(no_mangle)]
pub fn packed_struct_load(atomic: &Atomic<Quote>) -> Quote {
    atomic.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn fieldless_enum_load(atomic: &Atomic<Side>) -> Side {
    atomic.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn niche_filling_enum_load(atomic: &Atomic<Reading>) -> Reading {
    atomic.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn field_set(atomic: &Atomic<Order>) {
    atomic.fields().live.set(Release);
}

#[unsafe(no_mangle)]
pub fn field_clear(atomic: &Atomic<Order>) {
    atomic.fields().live.clear(Release);
}

#[unsafe(no_mangle)]
pub fn field_test_and_set(atomic: &Atomic<Order>) -> bool {
    atomic.fields().live.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn field_load(atomic: &Atomic<Order>) -> bool {
    atomic.fields().live.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn flags_or(atomic: &Atomic<Order>, flags: Flags) {
    atomic.fields().flags.or(flags, Release);
}

#[unsafe(no_mangle)]
pub fn nested_clear(atomic: &Atomic<Outer>) {
    atomic.fields().inner.fields().ready.clear(Release);
}

#[unsafe(no_mangle)]
pub fn top_fetch_add(atomic: &Atomic<Counted>, delta: u32) -> u32 {
    atomic.fields().count.fetch_add(delta, AcqRel).count
}
