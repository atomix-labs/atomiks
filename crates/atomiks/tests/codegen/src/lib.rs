//! One derived value's load per function, unmangled, for `tests/codegen.rs` to read the assembly
//! of: a packed struct's, a fieldless enum's and a niche-filling enum's.
//!
//! The empty `[workspace]` in its manifest makes it a workspace of its own: the repository's does
//! not list it, and the test builds it alone.

#![no_std]

use atomiks::ordering::Acquire;
use atomiks::{Atom, Atomic};

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
