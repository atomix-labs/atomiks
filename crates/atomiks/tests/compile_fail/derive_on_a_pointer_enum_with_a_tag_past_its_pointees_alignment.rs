//! Each pointer's alignment leaves clear the low bits where a pointer enum keeps its tag: a pointee
//! aligned to 1 leaves none, and the refusal points at that pointer.

use core::ptr::NonNull;

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
enum Child {
    Byte(NonNull<u8>),
    Word(NonNull<u64>),
}

fn main() {}
