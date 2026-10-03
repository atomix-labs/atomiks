//! A capability derives only for a newtype, which takes it from its one field.

use atomiks::AtomAdd;

#[derive(Clone, Copy, AtomAdd)]
struct Pair(u32, u32);

fn main() {}
