//! A newtype with parameters is bounded `Send` and `Sync`, as an `Atomic` crosses threads: an
//! instance of a raw pointer, which is neither, has no impl.

#![feature(const_trait_impl)]

use core::ptr;

use atomix::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
struct Wrap<T>(T);

fn main() {
    let _ = Atomic::new(Wrap(ptr::null_mut::<u8>()));
}
