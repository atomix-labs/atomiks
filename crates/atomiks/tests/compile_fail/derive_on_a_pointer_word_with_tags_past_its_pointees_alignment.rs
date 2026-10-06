//! A pointer word keeps its tags in the low bits its pointee's alignment leaves clear: a `u64`,
//! aligned to 8, leaves three, too few for a byte of tags, and the build is refused at the word.

use core::ptr::NonNull;

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
struct Node {
    next: NonNull<u64>,
    tag: u8,
}

fn main() {}
