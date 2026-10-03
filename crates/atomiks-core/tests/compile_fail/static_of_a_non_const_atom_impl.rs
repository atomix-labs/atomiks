//! A `static` is built at compile time, so the `Atom` impl of its atomic's value must be `const`.

use atomiks_core::{Atom, Atomic};

/// An id whose `Atom` impl is not `const`, so a `static` cannot build its atomic; following
/// rustc's help also takes `#![feature(const_trait_impl)]` in this crate.
#[derive(Clone, Copy)]
struct Id(u32);

unsafe impl Atom for Id {
    type Repr = u32;
    const MIN_REPR: u128 = 0;
    const MAX_REPR: u128 = 0xFFFF_FFFF;
    fn to_repr(self) -> u32 {
        self.0
    }
    fn from_repr(repr: u32) -> Option<Self> {
        Some(Self(repr))
    }
}

static ID: Atomic<Id> = Atomic::new(Id(0));

fn main() {
    let _ = &ID;
}
