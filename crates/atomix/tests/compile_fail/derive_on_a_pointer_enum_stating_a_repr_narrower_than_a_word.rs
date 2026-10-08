//! A pointer enum is stored as its pointers, one word: a repr of fewer bits names no pointer's
//! width, and is refused at the repr.

use core::ptr::NonNull;

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
#[atom(repr = u32)]
enum Next {
    End,
    Node(NonNull<u64>),
}

fn main() {}
