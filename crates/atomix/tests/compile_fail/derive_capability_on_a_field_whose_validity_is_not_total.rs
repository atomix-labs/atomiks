//! A newtype takes a capability only from a field that has it, so one over a `char`, which has no
//! add, is refused once, at the field, with the capability's message rather than its validity's.

use atomix::{Atom, AtomAdd};

#[derive(Clone, Copy, Atom, AtomAdd)]
struct Letter(char);

fn main() {}
