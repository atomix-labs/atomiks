//! A crate under the workspace's lints, `unsafe_code` forbidden, with `const_trait_impl`, derives
//! `Atom` for generic types and stores an instance of each in a static: a newtype, a struct of
//! several fields, enums with fields, niche-filling and tagged, a pointer word, pointer enums, one
//! with a variant of data, and structs of two words: two pointers, a pointer beside the integer
//! word `repr = u128` states, and a slice's pointer beside a tag.
//!
//! trybuild runs rustc alone, so the clippy lints here hold only where clippy builds the same code,
//! as it does in `tests/derive_generic.rs`.

#![feature(const_trait_impl)]
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

use core::num::NonZero;
use core::ptr::NonNull;

use atomiks::ordering::{Acquire, Relaxed};
use atomiks::{Atom, Atomic};

/// A value of any atom.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct Wrap<T>(T);

/// Two values of any atoms, packed in a `u64`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
#[atom(repr = u64)]
pub struct Pair<A, B> {
    /// The low one.
    first: A,
    /// The one above it.
    second: B,
}

/// A lock, free or held by its owner: `Free` fills a niche beside the owner's reprs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
#[atom(repr = u64)]
pub enum Lock<O> {
    /// Free to take.
    Free,
    /// Held by its owner.
    Held(O),
}

/// A ring buffer's slot, tagged by the discriminants it states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
#[atom(repr = u64)]
#[repr(u8)]
pub enum Slot<T> {
    /// Nothing written.
    Empty   = 0,
    /// Being written.
    Writing(T) = 1,
    /// Written.
    Ready(T) = 2,
}

/// A pointer to any `T` aligned to 2 or more, and a mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct Tagged<T> {
    /// The pointer.
    pub pointer: NonNull<T>,
    /// The mark.
    pub marked: bool,
}

/// One of two pointers, by one tag bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub enum Union<A, B> {
    /// The first.
    First(NonNull<A>),
    /// The second.
    Second(NonNull<B>),
}

/// A table's entry over any pointee: vacant, beside the next vacant entry's index, or occupied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub enum Entry<T> {
    /// Vacant, beside the next vacant entry's index.
    Vacant(u32),
    /// Occupied.
    Occupied(NonNull<T>),
}

/// Two pointers of any two types, and a mark in the first one's low bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct PointerPair<A, B> {
    /// The first.
    pub first: NonNull<A>,
    /// The second.
    pub second: NonNull<B>,
    /// The mark.
    pub marked: bool,
}

/// A pointer to any `T` beside a counter, in two words, as stated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
#[atom(repr = u128)]
pub struct Counted<T> {
    /// The top.
    pub top: Option<NonNull<T>>,
    /// The counter.
    pub version: u64,
}

/// A slice of any element, and a seal in its data pointer's low bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct Chunk<T> {
    /// The elements.
    pub elements: NonNull<[T]>,
    /// Whether the chunk is sealed.
    pub sealed: bool,
}

/// Two pointers, or `None`.
pub static PAIR: Atomic<Option<PointerPair<u32, u64>>> = Atomic::new(None);

/// A head, empty.
pub static COUNTED: Atomic<Counted<u64>> = Atomic::new(Counted { top: None, version: 0 });

/// A chunk, or `None`.
pub static CHUNK: Atomic<Option<Chunk<u32>>> = Atomic::new(None);

/// The next sequence number.
pub static NEXT: Atomic<Wrap<u32>> = Atomic::new(Wrap(0));

/// A marked pointer, or `None`.
pub static MARKED: Atomic<Option<Tagged<u32>>> = Atomic::new(None);

/// One of two pointers, or `None`.
pub static EITHER: Atomic<Option<Union<u32, u64>>> = Atomic::new(None);

/// The best price and whether it is live.
pub static BEST: Atomic<Pair<u32, bool>> = Atomic::new(Pair { first: 0, second: false });

/// The lock's owner, or `Free`.
pub static LOCK: Atomic<Lock<NonZero<u32>>> = Atomic::new(Lock::Free);

/// The last slot written, or `None` before the first.
pub static SLOT: Atomic<Option<Slot<u32>>> = Atomic::new(None);

/// An entry, vacant until it is filled.
pub static ENTRY: Atomic<Entry<u64>> = Atomic::new(Entry::Vacant(0));

fn main() {
    assert_eq!(PAIR.load_rmw(Acquire), None, "no pair");
    assert_eq!(COUNTED.load_rmw(Acquire), Counted { top: None, version: 0 }, "an empty head");
    assert_eq!(CHUNK.load_rmw(Acquire), None, "no chunk");
    assert_eq!(NEXT.swap(Wrap(1), Relaxed), Wrap(0), "one taken");
    BEST.store(Pair { first: 7, second: true }, Relaxed);
    assert_eq!(BEST.load(Acquire), Pair { first: 7, second: true }, "the best price");
    assert_eq!(LOCK.load(Acquire), Lock::Free, "free");
    assert_eq!(Lock::<NonZero<u32>>::Free.to_repr(), 0, "at zero, which no owner is");
    SLOT.store(Some(Slot::Writing(3)), Relaxed);
    assert_eq!(SLOT.load(Acquire), Some(Slot::Writing(3)), "a slot");
    assert_eq!(MARKED.load(Acquire), None, "no marked pointer");
    assert_eq!(EITHER.load(Acquire), None, "and neither of two");
    assert_eq!(ENTRY.load(Acquire), Entry::Vacant(0), "a vacant entry");
}
