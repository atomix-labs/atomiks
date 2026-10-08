//! `ReprRange::from_signed` takes two's complement bounds the repr's width holds, so a bound past them,
//! which would wrap onto another repr, is refused where the impl's range is built.

#![feature(const_trait_impl)]

use atomix_core::{Atom, Atomic, ReprRange};

/// No value: its range ends past the largest `i8`.
#[derive(Clone, Copy)]
enum Never {}

const unsafe impl Atom for Never {
    type Repr = i8;
    const REPRS: ReprRange<i8> = ReprRange::from_signed(-1, 128);
    fn to_repr(self) -> i8 {
        match self {}
    }
    fn from_repr(_: i8) -> Option<Self> {
        None
    }
}

static SLOT: Atomic<Option<Never>> = Atomic::new(None);

fn main() {
    let _ = &SLOT;
}
