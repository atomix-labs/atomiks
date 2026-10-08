//! `crate = …` names atomix by a path, not a string.

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
#[atom(crate = "atomix")]
struct Seq(u64);

fn main() {}
