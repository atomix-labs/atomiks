//! A range with `MIN_REPR` above `MAX_REPR` bounds no repr at all, so the repr `None` takes
//! beside it could be one a value takes.

#![feature(const_trait_impl)]

use atomiks_core::{Atom, Atomic};

/// No value: its range claims none.
#[derive(Clone, Copy)]
enum Never {}

const unsafe impl Atom for Never {
    type Repr = u8;
    const MIN_REPR: u128 = 5;
    const MAX_REPR: u128 = 3;
    fn to_repr(self) -> u8 {
        match self {}
    }
    fn from_repr(_: u8) -> Option<Self> {
        None
    }
}

static SLOT: Atomic<Option<Never>> = Atomic::new(None);

fn main() {
    let _ = &SLOT;
}
