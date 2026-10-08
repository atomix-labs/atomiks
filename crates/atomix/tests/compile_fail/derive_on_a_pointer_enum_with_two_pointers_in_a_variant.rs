//! A variant of a pointer enum holds one pointer: the second is refused at its type, since
//! two take two words.

use core::ptr::NonNull;

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
enum Pair {
    Empty,
    Both(NonNull<u64>, NonNull<u64>),
}

fn main() {}
