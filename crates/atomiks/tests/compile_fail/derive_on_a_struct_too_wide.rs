//! A packed value is stored in one atomic, of at most 128 bits: a wider one is refused once,
//! naming the bits it needs.

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
struct Wide {
    low: u128,
    live: bool,
}

fn main() {}
