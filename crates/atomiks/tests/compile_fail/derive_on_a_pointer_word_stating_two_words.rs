//! A pointer word is one word, its pointer: a repr of two is refused at the repr, since atomiks
//! does not yet hold a pointer in two.

use core::ptr::NonNull;

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
#[atom(repr = u128)]
struct Link {
    next: NonNull<u64>,
    deleted: bool,
}

fn main() {}
