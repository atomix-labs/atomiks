//! A pointer word is stored as its pointer, one word: a repr of fewer bits names no pointer's
//! width, and is refused at the repr.

use core::ptr::NonNull;

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
#[atom(repr = u32)]
struct Link {
    next: NonNull<u64>,
    deleted: bool,
}

fn main() {}
