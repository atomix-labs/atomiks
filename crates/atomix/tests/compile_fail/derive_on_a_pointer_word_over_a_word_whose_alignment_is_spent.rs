//! A word whose pointer is a word keeps its tags above the inner word's: a pointee aligned to 2
//! leaves one low bit, which the inner word's mark spends, and the outer word is refused.

use core::ptr::NonNull;

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
struct Inner {
    node: NonNull<u16>,
    marked: bool,
}

#[derive(Clone, Copy, Atom)]
struct Outer {
    #[atom(ptr)]
    inner: Inner,
    locked: bool,
}

fn main() {}
