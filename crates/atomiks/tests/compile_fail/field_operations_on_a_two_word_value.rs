//! No target has a 16-byte bitwise, add or exchange instruction: a field of a two-word value sets
//! and adds with `update`, and the value swaps with `update` too.

use core::ptr::NonNull;

use atomiks::ordering::{AcqRel, Acquire, Release};
use atomiks::{Atom, Atomic};

/// A node each value points to.
struct Node(u64);

/// Two nodes, and a mark in the first one's low bits.
#[derive(Clone, Copy, Atom)]
struct Edge {
    source: NonNull<Node>,
    target: NonNull<Node>,
    deleted: bool,
}

/// A Treiber stack's head: the top node beside a counter no alignment holds, which ends at the
/// value's top bit.
#[derive(Clone, Copy, Atom)]
struct Head {
    top: Option<NonNull<Node>>,
    version: u64,
}

fn probe(edge: &Atomic<Edge>, head: &Atomic<Head>) {
    edge.fields().deleted.update(AcqRel, Acquire, |_| true);
    edge.fields().deleted.set(Release);
    head.fields().version.update(AcqRel, Acquire, |version| version.wrapping_add(1));
    head.fields().version.fetch_add(1, AcqRel);
    let _ = edge.swap(edge.load_rmw(Acquire), AcqRel);
}

fn main() {
    let _ = probe;
}
