//! The repr a struct of several fields states may be wider than its fields need, but not narrower:
//! one too narrow is refused, naming the bits they need.

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
#[atom(repr = u32)]
struct Quote {
    qty: u32,
    live: bool,
}

fn main() {}
