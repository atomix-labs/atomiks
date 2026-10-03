//! `#[atom]` goes on the type, not on a variant.

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
enum Side {
    #[atom(repr = u8)]
    Bid,
    Ask,
}

fn main() {}
