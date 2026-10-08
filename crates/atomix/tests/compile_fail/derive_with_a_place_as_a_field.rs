//! A derived atom is a value that one atomic holds, so a field written as a place, an atomic or a
//! cell, is refused once, at its type, with no error of `Atom`'s bounds after it: in a struct of
//! several fields, whose atomic lends the field's place, in a newtype and in a variant.

use core::cell::Cell;

use atomix::{Atom, AtomicU32};

#[derive(Atom)]
struct Pair {
    seq: core::sync::atomic::AtomicU64,
    live: bool,
}

#[derive(Atom)]
struct Seq(AtomicU32);

#[derive(Atom)]
enum Slot {
    Empty,
    Full { lap: Cell<u32> },
}

fn main() {}
