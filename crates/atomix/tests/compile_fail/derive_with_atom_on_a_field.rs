//! `#[atom]` goes on the type: a field's reprs are its type's.

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
struct Seq(#[atom(repr = u32)] u64);

fn main() {}
