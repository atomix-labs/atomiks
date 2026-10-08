//! A pointer enum held in its own pointee is laid out by its pointers' tag widths alone, then
//! checked against their pointees' alignment: 64 bits do not fit above the three a bucket's
//! alignment leaves clear, and the refusal points at the variant, not at a cycle.

use core::ptr::NonNull;

use atomix::{Atom, Atomic};

struct Bucket {
    slot: Atomic<Slot>,
}

#[derive(Clone, Copy, Atom)]
enum Slot {
    Vacant(u64),
    Occupied(NonNull<Bucket>),
}

fn main() {}
