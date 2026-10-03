//! A pointer's bits are unreadable in const, so only null can mark `None`: at any other repr,
//! `None` would load as a pointer.

#![feature(const_trait_impl)]

use atomiks::{Atom, Atomic};

/// A pointer, null included, whose reprs stop below the top of memory: `None` would take a
/// nonzero repr, which a pointer cannot mark.
#[derive(Clone, Copy)]
struct Low(*mut u64);

const unsafe impl Atom for Low {
    type Repr = *mut u64;
    const MIN_REPR: u128 = 0;
    const MAX_REPR: u128 = (usize::MAX - 7) as u128;
    fn to_repr(self) -> *mut u64 {
        self.0
    }
    fn from_repr(repr: *mut u64) -> Option<Self> {
        Some(Self(repr))
    }
}

static SLOT: Atomic<Option<Low>> = Atomic::new(None);

fn main() {
    let _ = &SLOT;
}
