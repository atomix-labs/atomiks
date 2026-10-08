//! Each instance of a generic pointer enum has its data checked where its code is built, since the
//! enum's layout reads no pointee's alignment: 64 bits do not fit above the three a `u64` leaves
//! clear, and that instance is refused there.

#![feature(const_trait_impl)]

use core::ptr::NonNull;

use atomix::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
enum Slot<T> {
    Inline(u64),
    Node(NonNull<T>),
}

fn main() {
    let _ = Atomic::new(Slot::<u64>::Inline(0));
}
