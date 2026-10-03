//! A repr is what an atomic instruction reads and writes, so it is a primitive; a float has no
//! atomic instruction of its own, and is stored as its bits.

use atomiks_core::Atom;

/// A price in dollars, its repr the float itself rather than its bits.
#[derive(Clone, Copy)]
struct Price(f32);

unsafe impl Atom for Price {
    type Repr = f32;
    const MIN_REPR: u128 = 0;
    const MAX_REPR: u128 = 0xFFFF_FFFF;
    fn to_repr(self) -> f32 {
        self.0
    }
    fn from_repr(repr: f32) -> Option<Self> {
        Some(Self(repr))
    }
}

fn main() {}
