//! Each instance of a struct of several fields with parameters outside its markers lays its fields
//! out by them, so no constant knows its width to select a repr from: the struct states one, which
//! each instance is checked against.

use core::marker::PhantomData;

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
struct Pair<A, B> {
    first: A,
    second: B,
}

// Its marker's parameter comes first, and the error points past it, at `V`.
#[derive(Clone, Copy, Atom)]
struct Tagged<K, V> {
    value: V,
    live: bool,
    kind: PhantomData<K>,
}

fn main() {}
