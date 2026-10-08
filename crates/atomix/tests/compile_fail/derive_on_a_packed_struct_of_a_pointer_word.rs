//! A packed struct stores each field as its bits, which hold no pointer's provenance: a pointer word
//! beside an integer field is refused, at the word's type.

use core::ptr::NonNull;

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
struct Link {
    next: NonNull<u64>,
    deleted: bool,
}

#[derive(Clone, Copy, Atom)]
struct Counted {
    count: u32,
    link: Link,
}

fn main() {}
