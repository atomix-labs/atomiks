//! A derived atom is `Send` and `Sync`, as an `Atomic` of it crosses threads: a type whose negative
//! impl says it is not is refused, though each of its fields is.

#![feature(negative_impls)]

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
struct Token(u64);

impl !Send for Token {}

fn main() {}
