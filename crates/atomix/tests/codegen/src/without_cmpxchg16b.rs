//! Each shape a derive lays out in two words, where no atomic holds them, `x86_64` without
//! `cmpxchg16b`: a pointer beside tags no alignment holds, a pointer beside the integer word
//! `repr = u128` states, two pointers, and a slice's pointer, each refused once, at its derive,
//! naming the CPU it needs.

use core::ptr::NonNull;

use atomix::ordering::{AcqRel, Acquire};
use atomix::{Atom, Atomic};

/// A node each shape points to.
pub struct Node {
    /// What it holds.
    pub value: u64,
}

/// A Treiber stack's head: the top node beside a counter no alignment holds.
#[derive(Clone, Copy, Atom)]
pub struct Head {
    /// The top node.
    pub top: Option<NonNull<Node>>,
    /// How many times the head changed.
    pub version: u64,
}

/// A link and a mark, in two words, as stated.
#[derive(Clone, Copy, Atom)]
#[atom(repr = u128)]
pub struct StatedLink {
    /// The next node.
    pub next: Option<NonNull<Node>>,
    /// Whether the link is marked.
    pub marked: bool,
}

/// Two nodes, and a mark in the first one's low bits.
#[derive(Clone, Copy, Atom)]
pub struct MarkedPair {
    /// A node.
    pub first: NonNull<Node>,
    /// Its partner.
    pub second: NonNull<Node>,
    /// Whether the pair is deleted.
    pub marked: bool,
}

/// A slice of words, and a seal in its data pointer's low bits.
#[derive(Clone, Copy, Atom)]
pub struct Chunk {
    /// The words.
    pub words: NonNull<[u64]>,
    /// Whether the chunk is sealed.
    pub sealed: bool,
}

#[unsafe(no_mangle)]
pub fn head_compare_exchange_without_cmpxchg16b(
    atomic: &Atomic<Head>, current: Head, new: Head,
) -> Result<Head, Head> {
    atomic.compare_exchange(current, new, AcqRel, Acquire)
}

#[unsafe(no_mangle)]
pub fn stated_link_compare_exchange_without_cmpxchg16b(
    atomic: &Atomic<StatedLink>, current: StatedLink, new: StatedLink,
) -> Result<StatedLink, StatedLink> {
    atomic.compare_exchange(current, new, AcqRel, Acquire)
}

#[unsafe(no_mangle)]
pub fn marked_pair_compare_exchange_without_cmpxchg16b(
    atomic: &Atomic<MarkedPair>, current: MarkedPair, new: MarkedPair,
) -> Result<MarkedPair, MarkedPair> {
    atomic.compare_exchange(current, new, AcqRel, Acquire)
}

#[unsafe(no_mangle)]
pub fn chunk_compare_exchange_without_cmpxchg16b(
    atomic: &Atomic<Chunk>, current: Chunk, new: Chunk,
) -> Result<Chunk, Chunk> {
    atomic.compare_exchange(current, new, AcqRel, Acquire)
}
