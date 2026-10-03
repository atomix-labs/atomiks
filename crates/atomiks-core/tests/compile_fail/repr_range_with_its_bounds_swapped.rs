//! `ReprRange::new` takes its bounds in unsigned order, so a start above the end, which would claim
//! a range through zero, is refused where the impl's range is built.

#![feature(const_trait_impl)]

use atomiks_core::{Atom, Atomic, ReprRange};

/// No value: its range's bounds are swapped.
#[derive(Clone, Copy)]
enum Never {}

const unsafe impl Atom for Never {
    type Repr = u8;
    const REPRS: ReprRange<u8> = ReprRange::new(5, 3);
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
