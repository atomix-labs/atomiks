//! An instance of an enum with parameters is checked against the repr it states once it is built:
//! one wider is refused, naming the instance. A `u64` leaves no repr spare beside it, so the
//! units take a tag of two bits above it.

#![feature(const_trait_impl)]

use atomix::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
#[atom(repr = u64)]
enum Lock<O> {
    Uninit,
    Free,
    Owned(O),
}

static LOCK: Atomic<Lock<u64>> = Atomic::new(Lock::Free);

fn main() {
    let _ = &LOCK;
}
