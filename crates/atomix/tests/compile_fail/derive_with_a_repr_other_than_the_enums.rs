//! The repr a fieldless enum states is the one its `#[repr]` names: another is
//! refused where it is stated.

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
#[repr(u8)]
#[atom(repr = u16)]
enum Side {
    Bid,
    Ask,
}

fn main() {}
