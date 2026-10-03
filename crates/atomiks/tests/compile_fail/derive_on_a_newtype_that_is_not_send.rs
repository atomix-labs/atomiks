//! A derived atom is `Send` and `Sync`: a marker that keeps a type on one thread is not defeated
//! by an `Atomic`, which crosses threads.

use core::marker::PhantomData;

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
struct Token(u64, PhantomData<*const ()>);

fn main() {}
