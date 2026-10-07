//! A value is stored in one atomic, of at most 128 bits: an enum whose tag lies above 128 bits
//! of fields is refused once, naming the bits it needs.

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
enum Wide {
    Low(u128),
    High(u128),
}

fn main() {}
