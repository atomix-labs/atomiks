//! A pointer word holds one pointer, whose low bits hold its tags: a second is refused at its
//! type, since two take two words.

use core::ptr::NonNull;

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
struct Pair {
    first: NonNull<u64>,
    second: *mut u64,
    marked: bool,
}

fn main() {}
