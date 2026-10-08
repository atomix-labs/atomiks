//! `Atom` for a pair of values each stored as a pointer, in a [`DoubleWord`] of the two.

#![expect(unsafe_code, reason = "the `Atom` impl promises what loads rely on")]

use super::{Atom, PtrAtom};
use crate::primitive::DoubleWord;
use crate::range::{PointeeAlignment, ReprRange, Tags, assert_aligned};
use crate::validity::{Validity, ZeroValid};

/// What a pair of a `P` and a `Q` promises of zero: that it decodes where each one's does.
type ZeroValidity<P, Q> = <<P as Atom>::Validity as Validity>::ZeroValidityWith<
    <<Q as Atom>::Validity as Validity>::ZeroValidityWith<ZeroValid>,
>;

// A pair's tags, and those of a word that holds it, lie in its first pointer's low bits, as
// `Tags::set_in` sets them in a double word's: the first value takes them, beside its own.
//
// SAFETY: the repr is each value's own, the first's in the first word with the tags
// `to_tagged_repr` passes it, as its own impl promises, so its bits lie in every repr, or every one
// but zero where either value's range leaves zero out, as its pointer then has a bit set; `to_repr`
// refuses a pointer the first value finds misaligned for its own tags; `from_repr` decodes the repr
// exactly where each word decodes as its value, by the repr alone, and `from_repr_unchecked`
// decodes each word unchecked, which gives what `from_repr` does; the validity promises zero
// decodes only where each value's does; and each value may cross threads, as its own impl promises.
const unsafe impl<P: [const] PtrAtom, Q: [const] PtrAtom> Atom for (P, Q) {
    type Repr = DoubleWord<*mut P::Pointee, *mut Q::Pointee>;
    type Validity = ZeroValidity<P, Q>;
    const REPRS: ReprRange<Self::Repr> = if P::REPRS.contains(0) && Q::REPRS.contains(0) {
        ReprRange::FULL
    } else {
        ReprRange::NONZERO
    };
    const TAG_WIDTH: u32 = P::TAG_WIDTH;
    const POINTEE_ALIGNMENT: PointeeAlignment = P::POINTEE_ALIGNMENT;
    #[inline]
    #[track_caller]
    fn to_repr(self) -> Self::Repr {
        let (repr, misaligned) = self.to_tagged_repr(Tags::EMPTY);
        assert_aligned::<Self>(misaligned);
        repr
    }
    // Each value finds its own misaligned pointer, so one test of both stands on the path.
    #[inline]
    fn to_tagged_repr(self, tags: Tags) -> (Self::Repr, usize) {
        let (first, first_misaligned) = self.0.to_tagged_repr(tags);
        let (second, second_misaligned) = self.1.to_tagged_repr(Tags::EMPTY);
        (DoubleWord { first, second }, first_misaligned | second_misaligned)
    }
    #[inline]
    fn from_repr(repr: Self::Repr) -> Option<Self> {
        match (P::from_repr(repr.first), Q::from_repr(repr.second)) {
            (Some(first), Some(second)) => Some((first, second)),
            _ => None,
        }
    }
    #[inline]
    unsafe fn from_repr_unchecked(repr: Self::Repr) -> Self {
        // SAFETY: the caller's repr decodes, so its first word decodes as a `P`.
        let first = unsafe { P::from_repr_unchecked(repr.first) };
        // SAFETY: and its second as a `Q`.
        let second = unsafe { Q::from_repr_unchecked(repr.second) };
        (first, second)
    }
}
