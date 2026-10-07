//! No target has a 16-byte bit test-and-set: a `bool` field of a two-word value changes with
//! `update`.

use core::ptr::NonNull;

use atomiks::ordering::{AcqRel, Acquire};
use atomiks::{Atom, Atomic};

/// A node each pair points to.
struct Node(u64);

/// Two nodes, and a mark in the first one's low bits.
#[derive(Clone, Copy, Atom)]
struct Edge {
    source: NonNull<Node>,
    target: NonNull<Node>,
    deleted: bool,
}

fn probe(edge: &Atomic<Edge>) {
    edge.fields().deleted.update(AcqRel, Acquire, |_| true);
    let _ = edge.fields().deleted.test_and_set(AcqRel);
}

fn main() {
    let _ = probe;
}
