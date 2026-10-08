//! Deriving `Atom` for an enum of no variants is refused: it has no value to store.

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
enum Never {}

fn main() {}
