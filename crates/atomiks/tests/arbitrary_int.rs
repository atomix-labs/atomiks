//! `#[derive(Atom)]` over fields of arbitrary-int's integers: each takes its width alone, a signed
//! one extending its sign; and a newtype over one derives `AtomOrd`.

#![cfg(all(feature = "derive", feature = "arbitrary-int"))]
// Loom's `Atomic::new` is not `const`, so no static of one builds under it.
#![cfg(not(loom))]

// atomiks-core's, by its path; its `//!` says why.
#[cfg(test)]
#[path = "../../atomiks-core/tests/testing/mod.rs"]
mod testing;

#[cfg(test)]
mod tests {
    use arbitrary_int::traits::Integer;
    use arbitrary_int::{i3, u3, u20};
    #[cfg(target_arch = "aarch64")]
    use atomiks::ordering::Relaxed;
    use atomiks::ordering::{Acquire, Release};
    use atomiks::validity::ZeroValid;
    use atomiks::{Atom, AtomOrd, Atomic, ReprRange};

    use crate::testing::atom::{decodes_exactly_its_values, field_width, repr_and_validity_are};

    /// A header: three bits of kind, a bit of whether it is live, then three bits of signed delta,
    /// its top field, so seven bits that extend the sign.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Header {
        /// What kind.
        kind: u3,
        /// Whether it is live.
        live: bool,
        /// How far, and which way.
        delta: i3,
    }

    /// A sequence number, ordered as its integer.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Atom, AtomOrd)]
    struct Seq(u20);

    /// The last header read, or `None` before the first.
    static HEADER: Atomic<Option<Header>> = Atomic::new(None);
    /// The next sequence number.
    static SEQ: Atomic<Seq> = Atomic::new(Seq(u20::new(0)));

    #[test]
    fn each_field_takes_its_width_alone() {
        const { repr_and_validity_are::<Header, u8, ZeroValid>() };
        let header = Header { kind: u3::new(5), live: true, delta: i3::new(-1) };
        assert_eq!(header.to_repr(), 0b1111_1101, "5, then 1, then -1 extended from bit 4");
        assert_eq!(Header::REPRS, ReprRange::from_signed(-64, 63), "seven bits, sign-extended");
        assert_eq!(field_width(Header::REPRS), 7, "3 for the kind, 1 for live, 3 for the delta");
        assert_eq!(Header::from_repr(header.to_repr()), Some(header), "and back");
        decodes_exactly_its_values::<Header, _>(0..=u8::MAX, 8 * 2 * 8);
    }

    #[test]
    fn statics_of_derived_values_hold_the_values_stored() {
        assert_eq!(None::<Header>.to_repr(), 0xBF, "`None` below the range, at -65");
        assert_eq!(HEADER.load(Acquire), None, "`None` built in const");
        let header = Header { kind: u3::new(7), live: false, delta: i3::new(3) };
        HEADER.store(Some(header), Release);
        assert_eq!(HEADER.load(Acquire), Some(header), "the header stored");
        assert_eq!(SEQ.load(Acquire), Seq(u20::new(0)), "zero built in const");
        SEQ.store(Seq(u20::MAX), Release);
        assert_eq!(SEQ.load(Acquire), Seq(u20::MAX), "the largest");
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn a_newtype_keeps_the_order() {
        let seq = Atomic::new(Seq(u20::new(0)));
        seq.max(Seq(u20::new(9)), Relaxed);
        seq.max(Seq(u20::new(4)), Relaxed);
        assert_eq!(seq.load(Relaxed), Seq(u20::new(9)), "the larger kept");
    }
}
