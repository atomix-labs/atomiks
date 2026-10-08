//! A field of a derived atom takes no default: each field is read from the repr.

#![feature(default_field_values)]

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
struct Seq {
    value: u64 = 1,
}

fn main() {}
