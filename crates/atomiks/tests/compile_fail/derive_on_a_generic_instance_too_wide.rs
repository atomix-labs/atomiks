//! An instance of a struct with parameters is checked against the repr it states once it is built:
//! one wider is refused, naming the instance.

#![feature(const_trait_impl)]

use atomiks::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
#[atom(repr = u64)]
struct Pair<A, B> {
    first: A,
    second: B,
}

static PAIR: Atomic<Pair<u64, bool>> = Atomic::new(Pair { first: 0, second: false });

fn main() {
    let _ = &PAIR;
}
