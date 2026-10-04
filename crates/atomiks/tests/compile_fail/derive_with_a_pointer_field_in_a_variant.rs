//! Each field of an enum with fields is stored as its bits, which do not hold a pointer's
//! provenance: a variant's field stored as a pointer is refused, once.

use core::ptr::NonNull;

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
enum Link {
    End,
    Next(NonNull<u64>),
}

fn main() {}
