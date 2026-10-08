//! A packed value whose parameters reach markers alone is laid out once, and refused once where it
//! is wider than 128 bits, naming the type, as one without parameters is.

use core::marker::PhantomData;

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
struct Wide<T> {
    low: u128,
    live: bool,
    kind: PhantomData<T>,
}

fn main() {}
