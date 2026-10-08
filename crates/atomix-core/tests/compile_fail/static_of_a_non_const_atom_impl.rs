//! A `static` is built at compile time, so the `Atom` impl of its atomic's value must be `const`.

use atomix_core::{Atom, Atomic, ReprRange};

/// An id whose `Atom` impl is not `const`, so a `static` cannot build its atomic; following
/// rustc's help also takes `#![feature(const_trait_impl)]` in this crate.
#[derive(Clone, Copy)]
struct Id(u32);

unsafe impl Atom for Id {
    type Repr = u32;
    const REPRS: ReprRange<u32> = ReprRange::FULL;
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
