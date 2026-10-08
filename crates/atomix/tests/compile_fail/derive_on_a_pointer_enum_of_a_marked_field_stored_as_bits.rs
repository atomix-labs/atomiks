//! `#[atom(ptr)]` marks a pointer whose type shows none: a variant's field stored as an integer is
//! no pointer. A pointer enum's repr is `*mut ()`, and a variant's pointer is cast to and from it
//! where it is encoded and decoded, checked and unchecked, each place asking it to be one: each
//! refuses it at its type, with one message.

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
enum Count {
    Empty,
    Counted(#[atom(ptr)] u64),
}

fn main() {}
