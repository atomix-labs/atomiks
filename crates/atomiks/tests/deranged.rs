//! `#[derive(Atom)]` over a field of deranged's ranged integers: it takes the bits its range needs,
//! as a field of atomiks' own does, and a derived value holding one sits in a `static`.

#![cfg(all(feature = "derive", feature = "deranged-05"))]
// Loom's `Atomic::new` is not `const`, so no static of one builds under it.
#![cfg(not(loom))]

// atomiks-core's, by its path; its `//!` says why.
#[cfg(test)]
#[path = "../../atomiks-core/tests/testing/mod.rs"]
mod testing;

#[cfg(test)]
mod tests {
    use atomiks::ordering::{Acquire, Release};
    use atomiks::validity::Partial;
    use atomiks::{Atom, Atomic, ReprRange};
    use deranged::RangedU8;

    use crate::testing::atom::repr_and_validity_are;

    /// How deep a level lies in the book, from 1 to 10.
    type LevelDepth = RangedU8<1, 10>;
    /// The same, as atomiks' own ranged integer.
    type OwnLevelDepth = atomiks::RangedU8<1, 10>;

    /// A book level below a 16-bit sequence number.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Level {
        /// Which level.
        depth: LevelDepth,
        /// The sequence number.
        seq: u16,
    }

    /// The same level, with atomiks' own ranged integer.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct OwnLevel {
        /// Which level.
        depth: OwnLevelDepth,
        /// The sequence number.
        seq: u16,
    }

    /// The last level read, or `None` before the first.
    static LAST: Atomic<Option<Level>> = Atomic::new(None);

    #[test]
    fn a_field_takes_the_bits_its_range_needs() {
        const { repr_and_validity_are::<Level, u32, Partial>() };
        let level = Level { depth: LevelDepth::MAX, seq: u16::MAX };
        assert_eq!(level.to_repr(), 0xA | 0xFFFF << 4, "the depth in four bits, then 16");
        assert_eq!(Level::REPRS, ReprRange::new(0, (1 << 20) - 1), "4 bits of depth, then 16");
        assert_eq!(Level::REPRS, OwnLevel::REPRS, "as with atomiks' own ranged integer");
        assert_eq!(Level::from_repr(level.to_repr()), Some(level), "and back");
        assert_eq!(Level::from_repr(11), None, "refusing depth 11");
    }

    #[test]
    fn a_static_holds_the_level_stored() {
        assert_eq!(LAST.load(Acquire), None, "`None` built in const");
        let level = Level { depth: LevelDepth::new_static::<3>(), seq: 7 };
        LAST.store(Some(level), Release);
        assert_eq!(LAST.load(Acquire), Some(level), "the level stored");
    }
}
