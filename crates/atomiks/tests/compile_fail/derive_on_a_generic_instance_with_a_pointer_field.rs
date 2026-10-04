//! An instance of a struct with parameters stores each field as its bits, which do not hold a
//! pointer's provenance: one with a field stored as a pointer is refused.

#![feature(const_trait_impl)]

use core::ptr::NonNull;

use atomiks::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
#[atom(repr = u64)]
struct Pair<A, B> {
    first: A,
    second: B,
}

fn main() {
    let _ = Atomic::new(Pair { first: NonNull::<u64>::dangling(), second: 0_u8 });
}
