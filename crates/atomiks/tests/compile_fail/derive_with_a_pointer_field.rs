//! A packed value stores each field as its bits, which do not hold a pointer's provenance: a field
//! stored as a pointer is refused, once.

use core::ptr::NonNull;

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
struct Node {
    next: NonNull<u64>,
    tag: u8,
}

fn main() {}
