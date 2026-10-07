//! A newtype's field marked `#[atom(ptr)]` skips the thread checks a pointer's own impl answers for:
//! one stored as an integer is no pointer, and is refused at its type.

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
struct Seq(#[atom(ptr)] u64);

fn main() {}
