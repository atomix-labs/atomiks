//! `repr = …` states an integer primitive.

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
#[atom(repr = f64)]
struct Price(f64);

fn main() {}
