//! A union cannot derive `Atom`: no repr could say which field it holds.

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
union Bits {
    word: u32,
    bytes: [u8; 4],
}

fn main() {}
