//! A pointer to a slice holds its length beside its address, two words: a pointer enum's variant of
//! one is refused at the pointee.

use core::ptr::NonNull;

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
enum Buffer {
    Empty,
    Bytes(NonNull<[u8]>),
}

fn main() {}
