//! Above the bits its pointers' alignment leaves clear, a pointer enum holds a data variant's
//! fields: 64 bits do not fit above 3, and the refusal points at the variant.

use core::ptr::NonNull;

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
enum Entry {
    Vacant(u64),
    Occupied(NonNull<u64>),
}

fn main() {}
