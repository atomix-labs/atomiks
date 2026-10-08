//! A range past its primitive's largest bits holds reprs the primitive cannot, so `None` could
//! take one that wraps onto a value's; it is refused where the impl's range is built.

#![feature(const_trait_impl)]

use atomix_core::{Atom, Atomic, ReprRange};

/// No value: its range lies past its repr.
#[derive(Clone, Copy)]
enum Never {}

const unsafe impl Atom for Never {
    type Repr = u8;
    const REPRS: ReprRange<u8> = ReprRange::new(0, 300);
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
