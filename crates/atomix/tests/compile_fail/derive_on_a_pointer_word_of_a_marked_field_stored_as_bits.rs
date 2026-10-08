//! `#[atom(ptr)]` marks a pointer whose type shows none: a field stored as an integer is no pointer,
//! and is refused once, at its type, the one place a pointer word asks it to be one.

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
struct Counted {
    #[atom(ptr)]
    count: u64,
    marked: bool,
}

fn main() {}
