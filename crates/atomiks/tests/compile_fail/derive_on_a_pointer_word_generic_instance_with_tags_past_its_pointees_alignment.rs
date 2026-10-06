//! Each instance of a generic pointer word has its tags checked where its code is built, since the
//! word's layout reads no pointee's alignment: a `u16`, aligned to 2, leaves one bit, too few for
//! two, and that instance is refused there.

#![feature(const_trait_impl)]

use core::ptr::NonNull;

use atomiks::{Atom, Atomic, RangedU8};

#[derive(Clone, Copy, Atom)]
struct Tagged<T> {
    pointer: NonNull<T>,
    tag: RangedU8<0, 3>,
}

fn main() {
    let mut value = 7_u16;
    let tag = RangedU8::new(1).expect("1 lies in 0 to 3");
    let _ = Atomic::new(Tagged { pointer: NonNull::from(&mut value), tag });
}
