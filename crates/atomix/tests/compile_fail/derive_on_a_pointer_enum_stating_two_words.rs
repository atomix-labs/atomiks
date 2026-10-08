//! One word, a pointer, holds a pointer enum: a repr of two is refused at the repr.

use core::ptr::NonNull;

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
#[atom(repr = u128)]
enum Next {
    End,
    Node(NonNull<u64>),
}

fn main() {}
