//! `#[atom]` takes `crate` and `repr` alone; each derive reports the same error, which the compiler
//! shows once.

use atomiks::{Atom, AtomAdd};

#[derive(Clone, Copy, Atom, AtomAdd)]
#[atom(width = 8)]
struct Seq(u64);

fn main() {}
