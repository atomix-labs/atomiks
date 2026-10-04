//! Each instance of a struct of several fields with parameters lays its fields out by them, so no
//! constant knows its width to select a repr from: the struct states one, which each instance is
//! checked against.

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
struct Pair<A, B> {
    first: A,
    second: B,
}

fn main() {}
