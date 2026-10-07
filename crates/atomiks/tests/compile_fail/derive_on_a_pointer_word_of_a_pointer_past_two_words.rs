//! A double word holds two pointers, or one to a slice, a `str` or a trait object: a pointer past
//! those two words is refused at its type.

use core::ptr::NonNull;

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
struct Span {
    bytes: NonNull<[u8]>,
    owner: NonNull<u64>,
}

fn main() {}
