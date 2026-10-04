//! `#[derive(Atom)]` on a generic type, which needs `const_trait_impl` in the crate that derives:
//! its impl bounds each field that names a parameter, so each instance takes its field's
//! layout, or lays its fields out by their reprs in the repr it states, where zero decodes only
//! if each field's does.

#![cfg(feature = "derive")]
// Loom's `Atomic::new` is not `const`, so no static of one builds under it; `derive_laws.rs` builds
// the derive's code there.
#![cfg(not(loom))]
#![feature(const_trait_impl)]

// The crate under another name, which `#[atom(crate = …)]` names.
extern crate atomiks as renamed;

// atomiks-core's, by its path; its `//!` says why.
#[cfg(test)]
#[path = "../../atomiks-core/tests/testing/mod.rs"]
mod testing;

#[cfg(test)]
mod tests {
    use core::marker::PhantomData;
    use core::num::NonZero;

    use atomiks::ordering::{Acquire, Relaxed, Release};
    use atomiks::validity::{Partial, Total, TotalZeroNiche, ZeroValid};
    use atomiks::{Atom, AtomAdd, AtomBitwise, AtomOrd, Atomic, ReprRange};

    use crate::testing::atom::{decodes_exactly_its_values, repr_and_validity_are};

    /// A value of any atom.
    #[derive(
        Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Atom, AtomAdd, AtomOrd, AtomBitwise,
    )]
    struct Wrap<T>(T);

    /// A value of any atom stored as a `u32`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u32)]
    struct Word<T>(T);

    /// Two values of any atoms, packed in a `u64`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u64)]
    struct Pair<A, B> {
        /// The low one.
        first: A,
        /// The one above it.
        second: B,
    }

    /// A count of things of kind `K`, beside a flag, by position.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u32)]
    struct Counted<K>(u16, bool, PhantomData<K>);

    /// What a count counts.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Venue {}

    /// Which way a price moved: two bits, signed.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[repr(i8)]
    enum Sign {
        /// Down.
        Minus = -1,
        /// Neither.
        Flat,
        /// Up.
        Plus,
    }

    /// Two values of any atoms, naming atomiks by the crate's other name.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(crate = renamed, repr = u32)]
    struct Duo<A, B>(A, B);

    static COUNT: Atomic<Wrap<u32>> = Atomic::new(Wrap(1));
    static PAIRED: Atomic<Pair<u32, bool>> = Atomic::new(Pair { first: 0, second: false });
    static COUNTED: Atomic<Option<Counted<Venue>>> = Atomic::new(None);
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
    fn an_instance_lays_its_fields_out_by_their_reprs_in_the_repr_stated() {
        let pair = Pair { first: 0xDEAD_u32, second: true };
        assert_eq!(pair.to_repr(), 0xDEAD | 1 << 32, "the flag above the 32 bits");
        assert_eq!(Pair::from_repr(pair.to_repr()), Some(pair), "and back");
        assert_eq!(Pair::<u32, bool>::from_repr(1 << 33), None, "refusing a bit above");
        assert_eq!(Pair::<u32, bool>::REPRS, ReprRange::new(0, (1 << 33) - 1), "33 bits");
        let falling = Pair { first: true, second: Sign::Minus };
        assert_eq!(falling.to_repr(), u64::MAX, "a signed top field extends its sign");
        let counted = Counted::<Venue>(7, true, PhantomData);
        assert_eq!(counted.to_repr(), 7 | 1 << 16, "and fields by position, a marker of none");
    }

    #[test]
    fn an_instance_promises_zero_decodes_where_each_fields_zero_does() {
        repr_and_validity_are::<Pair<u32, bool>, u64, ZeroValid>();
        repr_and_validity_are::<Pair<u32, NonZero<u8>>, u64, Partial>();
        repr_and_validity_are::<Pair<NonZero<u8>, u32>, u64, Partial>();
        repr_and_validity_are::<Counted<Venue>, u32, ZeroValid>();
    }

    #[test]
    fn statics_hold_instances() {
        PAIRED.store(Pair { first: 3, second: true }, Release);
        assert_eq!(PAIRED.load(Acquire), Pair { first: 3, second: true }, "a pair");
        COUNTED.store(Some(Counted(2, false, PhantomData)), Release);
        assert_eq!(COUNTED.load(Acquire), Some(Counted(2, false, PhantomData)), "a count");
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

    #[test]
    fn an_instance_names_atomiks_by_the_path_stated() {
        repr_and_validity_are::<Duo<u8, bool>, u32, ZeroValid>();
        assert_eq!(Duo(7_u8, true).to_repr(), 7 | 1 << 8, "the flag above the byte");
        decodes_exactly_its_values::<Duo<u8, bool>, _>(0..=u32::from(u16::MAX), 256 * 2);
    }
}
