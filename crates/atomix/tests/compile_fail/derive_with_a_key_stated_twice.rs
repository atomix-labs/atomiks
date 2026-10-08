//! `#[atom]` states each key once.

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
#[atom(repr = u64)]
#[atom(repr = u32)]
struct Seq(u64);

fn main() {}
