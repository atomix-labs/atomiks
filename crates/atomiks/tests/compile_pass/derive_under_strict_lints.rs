//! A crate under the workspace's lints, `unsafe_code` and `dead_code` forbidden, with no feature
//! gate, derives `Atom` and the capabilities for documented newtypes, a pointer's among them, and
//! `Atom` for fieldless enums, a marker, a struct of several fields, whose projection it changes a
//! field through, a private one, whose projection it never uses, enums with fields, a pointer word,
//! whose tag it sets through its projection, a pointer enum, and structs of two words: a pointer
//! beside a counter, a link beside the integer word `repr = u128` states, two pointers, a slice's
//! pointer beside a tag, and a newtype of a trait object's pointer; and stores each in a static.
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
// After `unused`, which it raises for dead code alone: the derive writes nothing that is dead, or
// that allows it.
#![forbid(dead_code)]

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

/// The best bid and ask, which no code projects onto its fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
struct Spread {
    /// The best bid's price.
    bid: u16,
    /// The best ask's price.
    ask: u16,
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

/// The quote on the book.
pub static QUOTE: Atomic<Quote> =
    Atomic::new(Quote { quantity: 3, side: Side::Bid, sign: Sign::Flat });

/// The spread on the book.
static SPREAD: Atomic<Spread> = Atomic::new(Spread { bid: 99, ask: 101 });

/// A node of a list, aligned to 8.
#[derive(Debug)]
#[repr(align(8))]
pub struct Node {
    /// What it holds.
    pub value: u64,
}

/// A link to a node, and whether the node that holds it is deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct Link {
    /// The next node.
    pub next: NonNull<Node>,
    /// Whether it is deleted.
    pub deleted: bool,
}

/// The next node, or the end of the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub enum Next {
    /// The end.
    End,
    /// A node.
    Node(NonNull<Node>),
}

/// The first link of a list, or `None` before the first.
pub static FIRST: Atomic<Option<Link>> = Atomic::new(None);

/// The next node of a list, its end before the first.
pub static TAIL: Atomic<Next> = Atomic::new(Next::End);

/// A stack's head: its top node beside a counter no alignment holds, in two words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct Counted {
    /// The top node.
    pub top: Option<NonNull<Node>>,
    /// How many times the head changed.
    pub version: u64,
}

/// A link and a count beside it, in the integer word `repr = u128` states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
#[atom(repr = u128)]
pub struct StatedLink {
    /// The next node.
    pub next: NonNull<Node>,
    /// A count.
    pub count: u16,
}

/// Two nodes, and a mark in the first one's low bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct Pair {
    /// A node.
    pub first: NonNull<Node>,
    /// Its partner.
    pub second: NonNull<Node>,
    /// Whether the pair is marked.
    pub marked: bool,
}

/// A slice of words, and a seal in its data pointer's low bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct Chunk {
    /// The words.
    pub words: NonNull<[u64]>,
    /// Whether the chunk is sealed.
    pub sealed: bool,
}

/// A callback, a trait object's pointer.
#[derive(Clone, Copy, Debug, Atom)]
pub struct Callback(pub NonNull<dyn Fn() + Send + Sync>);

/// A stack's head, empty.
pub static COUNTED: Atomic<Counted> = Atomic::new(Counted { top: None, version: 0 });

/// A link and its count, or `None`.
pub static STATED: Atomic<Option<StatedLink>> = Atomic::new(None);

/// A pair of nodes, or `None`.
pub static PAIR: Atomic<Option<Pair>> = Atomic::new(None);

/// A chunk, or `None`.
pub static CHUNK: Atomic<Option<Chunk>> = Atomic::new(None);

/// A callback, or `None`.
pub static CALLBACK: Atomic<Option<Callback>> = Atomic::new(None);

fn main() {
    let empty = Counted { top: None, version: 0 };
    assert_eq!(
        COUNTED.update(Relaxed, Relaxed, |head| Counted { version: 1, ..head }),
        empty,
        "a head"
    );
    assert_eq!(STATED.load_rmw(Acquire), None, "no link");
    assert_eq!(PAIR.load_rmw(Acquire), None, "no pair");
    assert_eq!(CHUNK.load_rmw(Acquire), None, "no chunk");
    assert!(CALLBACK.load_rmw(Acquire).is_none(), "no callback");
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
    let fields = QUOTE.fields();
    let before = fields.quantity.update(Relaxed, Relaxed, |quantity| quantity + 1);
    assert_eq!(before.quantity, 3, "a field changed");
    assert_eq!(fields.side.load(Acquire), Side::Bid, "and another as it was");
    let spread = SPREAD.load(Acquire);
    assert_eq!((spread.bid, spread.ask), (99, 101), "and a spread, read whole");
    assert_eq!(FIRST.load(Acquire), None, "no first link");
    let link = Atomic::new(Link { next: NonNull::dangling(), deleted: false });
    link.fields().deleted.set(Relaxed);
    assert!(link.load(Acquire).deleted, "a link deleted through its tag");
    assert_eq!(TAIL.load(Acquire), Next::End, "and the end of a list");
}
