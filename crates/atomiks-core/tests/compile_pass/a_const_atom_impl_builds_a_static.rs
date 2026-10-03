//! A value's own `const unsafe impl Atom` builds a `static` of it, and of its `Option`.

#![feature(const_trait_impl)]

use atomiks_core::ordering::{Acquire, Release};
use atomiks_core::{Atom, Atomic};

/// The side of the book an order rests on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Bid,
    Ask,
}

// SAFETY: `to_repr` gives 0 or 1, within `MIN_REPR..=MAX_REPR`, and `from_repr` decodes each as the
// side it came from, by the repr alone; the default `from_repr_unchecked` unwraps `from_repr`;
// `Partial`, the default validity, promises no other repr; and a `Side` holds no data, so it may
// cross threads.
const unsafe impl Atom for Side {
    type Repr = u8;
    const MIN_REPR: u128 = 0;
    const MAX_REPR: u128 = 1;
    fn to_repr(self) -> u8 {
        match self {
            Self::Bid => 0,
            Self::Ask => 1,
        }
    }
    fn from_repr(repr: u8) -> Option<Self> {
        match repr {
            0 => Some(Self::Bid),
            1 => Some(Self::Ask),
            _ => None,
        }
    }
}

static SIDE: Atomic<Side> = Atomic::new(Side::Bid);
static LAST_FILL: Atomic<Option<Side>> = Atomic::new(None);

fn main() {
    assert_eq!(SIDE.load(Acquire), Side::Bid, "the side the static was built with");
    LAST_FILL.store(Some(Side::Ask), Release);
    assert_eq!(LAST_FILL.load(Acquire), Some(Side::Ask), "and the side stored in its `Option`");
}
