//! A value whose zero does not decode gives `Option`'s `None` repr 0, even where zero lies inside
//! its range.

#![feature(const_trait_impl)]

use atomix_core::ordering::{Acquire, Release};
use atomix_core::validity::ZeroNiche;
use atomix_core::{Atom, Atomic, ReprRange};

/// A step back or forward, -1 or 1: its range runs through zero, which no step takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Back,
    Forward,
}

// SAFETY: `to_repr` gives -1 or 1, within `REPRS`, and `from_repr` decodes each as the step it came
// from, by the repr alone, and zero as none, as `ZeroNiche` promises; the default
// `from_repr_unchecked` unwraps `from_repr`; and a `Step` holds no data, so it may cross threads.
const unsafe impl Atom for Step {
    type Repr = i8;
    type Validity = ZeroNiche;
    const REPRS: ReprRange<i8> = ReprRange::from_signed(-1, 1);
    fn to_repr(self) -> i8 {
        match self {
            Self::Back => -1,
            Self::Forward => 1,
        }
    }
    fn from_repr(repr: i8) -> Option<Self> {
        match repr {
            -1 => Some(Self::Back),
            1 => Some(Self::Forward),
            _ => None,
        }
    }
}

static LAST_STEP: Atomic<Option<Step>> = Atomic::new(None);

fn main() {
    assert_eq!(<Option<Step> as Atom>::to_repr(None), 0, "`None` takes zero, inside the range");
    assert_eq!(<Option<Step> as Atom>::REPRS, <Step as Atom>::REPRS, "so the range stays as it is");
    assert_eq!(LAST_STEP.load(Acquire), None, "the static built with `None`");
    LAST_STEP.store(Some(Step::Back), Release);
    assert_eq!(LAST_STEP.load(Acquire), Some(Step::Back), "and the step stored");
}
