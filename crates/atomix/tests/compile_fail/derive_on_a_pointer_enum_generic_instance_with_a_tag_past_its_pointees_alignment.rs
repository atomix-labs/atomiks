//! Each instance of a generic pointer enum has its tag checked where its code is built, since the
//! enum's layout reads no pointee's alignment: two pointers to bytes, aligned to 1, leave no bit
//! for the tag, and that instance is refused there.

#![feature(const_trait_impl)]

use core::ptr::NonNull;

use atomix::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
enum Union<A, B> {
    Left(#[atom(ptr)] A),
    Right(#[atom(ptr)] B),
}

fn main() {
    let mut byte = 7_u8;
    let _ = Atomic::new(Union::<NonNull<u8>, NonNull<u8>>::Left(NonNull::from(&mut byte)));
}
