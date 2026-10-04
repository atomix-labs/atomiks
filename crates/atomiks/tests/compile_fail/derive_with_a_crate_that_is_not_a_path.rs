//! `crate = …` names atomiks by a path, not a string.

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
#[atom(crate = "atomiks")]
struct Seq(u64);

fn main() {}
