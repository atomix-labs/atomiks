//! `#[derive(Atom)]` on a generic type, which needs `const_trait_impl` in the crate that derives:
//! its impl bounds each field that names a parameter, so each instance takes its field's layout.

#![cfg(feature = "derive")]
// Loom's `Atomic::new` is not `const`, so no static of one builds under it; `derive_laws.rs` builds
// the derive's code there.
#![cfg(not(loom))]
#![feature(const_trait_impl)]

// atomiks-core's, by its path; its `//!` says why.
#[cfg(test)]
#[path = "../../atomiks-core/tests/testing/mod.rs"]
mod testing;

#[cfg(test)]
mod tests {
    use core::num::NonZero;

    use atomiks::ordering::{Acquire, Relaxed, Release};
    use atomiks::validity::{Total, TotalZeroNiche};
    use atomiks::{Atom, AtomAdd, AtomBitwise, AtomOrd, Atomic, ReprRange};

    use crate::testing::atom::repr_and_validity_are;

    /// A value of any atom.
    #[derive(
        Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Atom, AtomAdd, AtomOrd, AtomBitwise,
    )]
    struct Wrap<T>(T);

    /// A value of any atom stored as a `u32`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u32)]
    struct Word<T>(T);

    static COUNT: Atomic<Wrap<u32>> = Atomic::new(Wrap(1));
    static OWNER: Atomic<Option<Wrap<NonZero<u64>>>> = Atomic::new(None);

    /// Compiles only where `T` has every capability.
    const fn has_every_capability<T: AtomAdd + AtomOrd + AtomBitwise>() {}

    #[test]
    fn each_instance_takes_its_fields_repr_range_and_validity() {
        repr_and_validity_are::<Wrap<u8>, u8, Total>();
        repr_and_validity_are::<Wrap<NonZero<u64>>, u64, TotalZeroNiche>();
        repr_and_validity_are::<Word<NonZero<u32>>, u32, TotalZeroNiche>();
        assert_eq!(Wrap::<NonZero<u64>>::REPRS, ReprRange::NONZERO, "every repr but zero");
    }

    #[test]
    fn an_instance_round_trips_as_its_field() {
        let id = Wrap(NonZero::<u64>::MAX);
        assert_eq!(Wrap::from_repr(id.to_repr()), Some(id), "to its repr and back");
        assert_eq!(Wrap::<NonZero<u64>>::from_repr(0), None, "refusing what the field refuses");
        assert_eq!(Word(7_u32).to_repr(), 7, "in the repr stated");
    }

    #[test]
    fn none_takes_zero_where_the_field_never_does() {
        assert_eq!(OWNER.load(Acquire), None, "stored as zero");
        let id = Wrap(NonZero::<u64>::MIN);
        OWNER.store(Some(id), Release);
        assert_eq!(OWNER.load(Acquire), Some(id), "beside every id");
        assert_eq!(None::<Wrap<NonZero<u64>>>.to_repr(), 0, "`None`");
    }

    #[test]
    fn an_instance_takes_its_fields_capabilities() {
        has_every_capability::<Wrap<u32>>();
        COUNT.add(2, Relaxed);
        assert_eq!(COUNT.load(Relaxed), Wrap(3), "add");
        COUNT.or(Wrap(0b100), Relaxed);
        assert_eq!(COUNT.load(Relaxed), Wrap(0b111), "then or");
    }

    // x86_64 has no atomic maximum, so `max` exists on aarch64 alone.
    #[cfg(target_arch = "aarch64")]
    #[test]
    fn atom_ord_brings_max_where_the_target_has_one() {
        let count = Atomic::new(Wrap(1_u32));
        count.max(Wrap(9), Relaxed);
        assert_eq!(count.load(Relaxed), Wrap(9), "the larger");
    }
}
