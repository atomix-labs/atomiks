//! `#[atom]`'s keys each take a value, `repr = u64`.

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
#[atom(repr)]
struct Seq(u64);

fn main() {}
