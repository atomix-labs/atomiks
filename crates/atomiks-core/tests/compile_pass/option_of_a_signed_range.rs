//! A range through zero, -1 to 1 as an `i8`, leaves `Option`'s `None` a repr beside it, -2, and an
//! `Option` of that `Option` the next, -3, each built into a `static`.

#![feature(const_trait_impl)]

use atomiks_core::ordering::{Acquire, Release};
use atomiks_core::{Atom, Atomic, ReprRange};

/// Which way a price moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sign {
    Minus,
    Flat,
    Plus,
}

// SAFETY: `to_repr` gives -1, 0 or 1, within `REPRS`, and `from_repr` decodes each as the sign it
// came from, by the repr alone; the default `from_repr_unchecked` unwraps `from_repr`; `Partial`,
// the default validity, promises no other repr; and a `Sign` holds no data, so it may cross
// threads.
const unsafe impl Atom for Sign {
    type Repr = i8;
    const REPRS: ReprRange<i8> = ReprRange::from_signed(-1, 1);
    fn to_repr(self) -> i8 {
        match self {
            Self::Minus => -1,
            Self::Flat => 0,
            Self::Plus => 1,
        }
    }
    fn from_repr(repr: i8) -> Option<Self> {
        match repr {
            -1 => Some(Self::Minus),
            0 => Some(Self::Flat),
            1 => Some(Self::Plus),
            _ => None,
        }
    }
}

static LAST_MOVE: Atomic<Option<Sign>> = Atomic::new(None);
static FIRST_MOVE: Atomic<Option<Option<Sign>>> = Atomic::new(None);

fn main() {
    assert_eq!(<Option<Sign> as Atom>::to_repr(None), -2, "`None` takes -2, below the range");
    assert_eq!(<Option<Option<Sign>> as Atom>::to_repr(None), -3, "and the outer `None` -3");
    LAST_MOVE.store(Some(Sign::Minus), Release);
    assert_eq!(LAST_MOVE.load(Acquire), Some(Sign::Minus), "a move stored");
    FIRST_MOVE.store(Some(None), Release);
    assert_eq!(FIRST_MOVE.load(Acquire), Some(None), "and the inner `None`, apart from the outer");
}
