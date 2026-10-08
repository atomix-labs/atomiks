//! A pointer's bits are unreadable in const, so only null can mark `None`: where null is a value,
//! `None` has no repr, though others lie outside the value's range.

#![feature(const_trait_impl)]

use atomix_core::{Atom, Atomic, ReprRange};

/// A pointer, null included, whose reprs stop below the top of memory: the reprs above are spare,
/// but a pointer marks `None` only with null.
#[derive(Clone, Copy)]
struct Low(*mut u64);

const unsafe impl Atom for Low {
    type Repr = *mut u64;
    const REPRS: ReprRange<*mut u64> = ReprRange::new(0, (usize::MAX - 7) as u128);
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
