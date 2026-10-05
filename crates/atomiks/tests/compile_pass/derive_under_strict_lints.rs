//! A crate under the workspace's lints, `unsafe_code` forbidden, derives `Atom` and the
//! capabilities for documented newtypes, a pointer's among them, and `Atom` for fieldless enums, a
//! marker, a struct of several fields and enums with fields, and stores each in a static, with no
//! feature gate.
//!
//! trybuild runs rustc alone, so the clippy lints here hold only where clippy builds the same code,
//! as it does in `tests/derive_newtype.rs`.

#![forbid(unsafe_code)]
#![deny(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    single_use_lifetimes,
    unreachable_pub,
    unused,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    variant_size_differences,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::absolute_paths,
    clippy::allow_attributes,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::indexing_slicing,
    clippy::missing_docs_in_private_items,
    clippy::multiple_unsafe_ops_per_block,
    clippy::std_instead_of_core,
    clippy::undocumented_unsafe_blocks,
    clippy::wildcard_enum_match_arm
)]

use core::marker::PhantomData;
use core::ptr::NonNull;

use atomiks::ordering::{Acquire, Relaxed};
use atomiks::{Atom, AtomAdd, AtomBitwise, AtomOrd, Atomic};

/// A sequence number.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Atom, AtomAdd, AtomOrd, AtomBitwise,
)]
pub struct Seq(u64);

/// An id of something of kind `K`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct Id<K> {
    /// The id.
    value: u32,
    /// What it names.
    kind: PhantomData<K>,
}

/// The side of the book an order rests on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub enum Side {
    /// A buy.
    Bid,
    /// A sell.
    Ask,
}

/// Which way a price moved, stored as its discriminant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
#[repr(i8)]
pub enum Sign {
    /// Down.
    Minus = -1,
    /// Neither.
    Flat,
    /// Up.
    Plus,
}

/// Marks the book, storing nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct Marker;

/// A resting quote, its fields packed into one word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct Quote {
    /// How many.
    quantity: u32,
    /// Which side.
    side: Side,
    /// Which way the price last moved.
    sign: Sign,
}

/// A ring buffer's slot: a tag above a lap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub enum Slot {
    /// Nothing written.
    Empty,
    /// Being written, in a lap.
    Writing {
        /// The lap.
        lap: u32,
    },
    /// Written, in a lap.
    Ready(u32),
}

/// A reading of a sign or none, which takes the repr below the sign's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub enum Reading {
    /// No reading yet.
    Missing,
    /// The sign read.
    Present(Sign),
}

/// How far a discriminant moves.
const OFFSET: i8 = 2;

/// A shift by a byte, tagged by the discriminants it states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
#[repr(i8)]
pub enum Shift {
    /// Back.
    Back(u8) = -1,
    /// Still.
    Still,
    /// Ahead.
    Ahead(u8) = OFFSET + 3,
}

/// The head of a list another thread may take.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct Head(NonNull<u64>);

/// The next sequence number.
pub static NEXT: Atomic<Seq> = Atomic::new(Seq(0));

/// The last id taken.
pub static LAST: Atomic<Id<Seq>> = Atomic::new(Id { value: 0, kind: PhantomData });

/// The list's head, `None` while it is empty.
pub static HEAD: Atomic<Option<Head>> = Atomic::new(None);

/// The side of the last fill, or `None` before the first.
pub static FILL: Atomic<Option<Side>> = Atomic::new(None);

/// The last move.
pub static MOVE: Atomic<Sign> = Atomic::new(Sign::Flat);

/// Whether the book is marked.
pub static MARK: Atomic<Option<Marker>> = Atomic::new(Some(Marker));

/// The best quote, or `None` before the first.
pub static BEST: Atomic<Option<Quote>> = Atomic::new(None);

/// The last slot written, or `None` before the first.
pub static SLOT: Atomic<Option<Slot>> = Atomic::new(None);

/// The last reading.
pub static READING: Atomic<Reading> = Atomic::new(Reading::Missing);

/// The last shift.
pub static SHIFT: Atomic<Shift> = Atomic::new(Shift::Still);

fn main() {
    assert_eq!(NEXT.fetch_add(1, Relaxed), Seq(0), "zero taken");
    assert_eq!(NEXT.load(Acquire), Seq(1), "one next");
    assert_eq!(LAST.load(Acquire).value, 0, "and no id yet");
    assert_eq!(HEAD.load(Acquire), None, "nor a list's head");
    assert_eq!(FILL.load(Acquire), None, "no fill");
    assert_eq!(MOVE.swap(Sign::Minus, Relaxed), Sign::Flat, "a move down");
    assert_eq!(MARK.load(Acquire), Some(Marker), "the mark");
    let quote = Quote { quantity: 3, side: Side::Ask, sign: Sign::Minus };
    BEST.store(Some(quote), Relaxed);
    assert_eq!(BEST.load(Acquire), Some(quote), "and the quote");
    SLOT.store(Some(Slot::Writing { lap: 1 }), Relaxed);
    assert_eq!(SLOT.load(Acquire), Some(Slot::Writing { lap: 1 }), "a slot");
    assert_eq!(SLOT.swap(Some(Slot::Ready(1)), Relaxed), Some(Slot::Writing { lap: 1 }), "swapped");
    READING.store(Reading::Present(Sign::Plus), Relaxed);
    assert_eq!(READING.load(Acquire), Reading::Present(Sign::Plus), "a reading");
    SHIFT.store(Shift::Back(3), Relaxed);
    assert_eq!(SHIFT.load(Acquire), Shift::Back(3), "and a shift");
    assert_eq!(Shift::Ahead(0).to_repr(), 5 << 8, "tagged by its discriminant");
}
