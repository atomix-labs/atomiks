//! A deprecated tuple field's place, through the projection, is deprecated as the field is.

#![deny(deprecated)]

use atomix::ordering::Acquire;
use atomix::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
struct Pair(#[deprecated = "read the count"] bool, u8);

fn is_flagged(pair: &Atomic<Pair>) -> bool {
    pair.fields().0.load(Acquire)
}

fn main() {}
