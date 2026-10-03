//! An `Option` of a zero-niche value promises repr 0 decodes, as its `None`: with `MIN_REPR` 2,
//! `None` would take repr 1, and the promise would lie.

#![feature(const_trait_impl)]

use atomiks_core::validity::ZeroNiche;
use atomiks_core::{Atom, Atomic};

/// An even byte above zero: its validity says `None` takes zero, but its lowest repr is 2.
#[derive(Clone, Copy)]
struct Even(u8);

const unsafe impl Atom for Even {
    type Repr = u8;
    type Validity = ZeroNiche;
    const MIN_REPR: u128 = 2;
    const MAX_REPR: u128 = 254;
    fn to_repr(self) -> u8 {
        self.0
    }
    fn from_repr(repr: u8) -> Option<Self> {
        if repr != 0 && repr % 2 == 0 { Some(Self(repr)) } else { None }
    }
}

static SLOT: Atomic<Option<Even>> = Atomic::new(None);

fn main() {
    let _ = &SLOT;
}
