//! A pointer word is stored in one word or two: a repr of fewer bits than a word names neither,
//! and is refused at the repr.

use core::ptr::NonNull;

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
#[atom(repr = u32)]
struct Link {
    next: NonNull<u64>,
    deleted: bool,
}

fn main() {}
