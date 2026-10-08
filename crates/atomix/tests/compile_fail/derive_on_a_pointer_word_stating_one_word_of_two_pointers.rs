//! Two pointers take two words: `repr = u64`, one word, is refused at the repr.

use core::ptr::NonNull;

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
#[atom(repr = u64)]
struct Pair {
    first: NonNull<u64>,
    second: NonNull<u64>,
}

fn main() {}
