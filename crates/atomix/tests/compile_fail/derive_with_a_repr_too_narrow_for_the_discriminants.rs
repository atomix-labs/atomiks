//! The repr a fieldless enum without a `#[repr]` states must hold each discriminant: one too narrow
//! is refused, naming the bits the discriminants need.

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
#[atom(repr = u8)]
enum Swing {
    Low = -200,
    High = 200,
}

fn main() {}
