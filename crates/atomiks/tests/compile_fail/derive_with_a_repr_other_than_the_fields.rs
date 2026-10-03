//! The repr a newtype states is its field's: one the field does not have is refused where the
//! newtype derives.

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
#[atom(repr = u32)]
struct Seq(u64);

fn main() {}
