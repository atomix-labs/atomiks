//! `#[derive(Atom)]` on a generic type, which needs `const_trait_impl` in the crate that derives:
//! its impl bounds each field that names a parameter, so each instance takes its field's
//! layout, or lays its fields out by their reprs in the repr it states, where zero decodes only
//! if each field's does, or for an enum, where its discriminants put a unit variant there.

// Above the `cfg`, which without `derive` drops every attribute after it: the parser refuses a
// `const` impl without the feature before the `cfg` drops the impl.
#![feature(const_trait_impl)]
#![cfg(feature = "derive")]
// Loom's `Atomic::new` is not `const`, so no static of one builds under it; `derive_laws.rs` builds
// the derive's code there.
#![cfg(not(loom))]

// The crate under another name, which `#[atom(crate = …)]` names.
extern crate atomix as renamed;

// atomix-core's, by its path; its `//!` says why.
#[cfg(test)]
#[path = "../../atomix-core/tests/testing/mod.rs"]
mod testing;

#[cfg(test)]
mod tests {
    use core::marker::PhantomData;
    use core::num::NonZero;

    use atomix::ordering::{Acquire, Relaxed, Release};
    use atomix::validity::{Partial, Total, TotalZeroNiche, ZeroValid};
    use atomix::{Atom, AtomAdd, AtomBitwise, AtomOrd, Atomic, RangedU64, ReprRange};

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

    /// Stores nothing.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Marker;

    /// An owner's id, from 3 up, so the reprs below are free.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct OwnerId(RangedU64<3>);

    /// A lock, uninitialised, free or owned: its units fill a niche beside an owner's reprs where
    /// that is no wider than a tag.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u64)]
    enum Lock<O> {
        /// Not yet built.
        Uninit,
        /// Free to take.
        Free,
        /// Held by its owner.
        Owned(O),
    }

    /// A lock word of stated discriminants, tagged by them, so zero is `Unbuilt` whatever owns
    /// it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u64)]
    #[repr(u8)]
    enum LockWord<O> {
        /// Not yet built.
        Unbuilt  = 0,
        /// Free to take.
        Free     = 1,
        /// Held by its owner.
        Owned(O) = 2,
        /// Poisoned by a panic.
        Poisoned = 3,
    }

    /// Variants named as the items the derive names, one with a field of any atom: none of their
    /// names clashes with one of the derive's, nor with the function that lays an instance out.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u16)]
    #[expect(non_camel_case_types, reason = "named as the derive's items, lowercase ones too")]
    enum Named<T> {
        /// As the repr.
        Repr,
        /// As a conversion's parameter.
        repr,
        /// As the bits a conversion reads.
        bits,
        /// As the layout.
        layout(T),
        /// As a field's value.
        value_0,
        /// As the function that lays an instance out.
        lay_out,
        /// As a variant's layout.
        variant_0,
        /// As a field's placement.
        placement_0,
        /// As the discriminants' integer.
        Discriminant,
        /// As a discriminant.
        discriminant_0,
    }

    /// Two values of any atoms, naming atomix by the crate's other name.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(crate = renamed, repr = u32)]
    struct Duo<A, B>(A, B);

    /// A value of any atom or none, of stated discriminants, naming atomix by the crate's other
    /// name.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(crate = renamed, repr = u32)]
    #[repr(u8)]
    enum Slot<T> {
        /// Nothing held.
        Empty = 0,
        /// A value held.
        Full(T) = 1,
    }

    static COUNT: Atomic<Wrap<u32>> = Atomic::new(Wrap(1));
    static LOCK: Atomic<Option<Lock<OwnerId>>> = Atomic::new(None);
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
    fn an_enums_units_fill_the_niche_below_an_owners_reprs() {
        repr_and_validity_are::<Lock<OwnerId>, u64, Partial>();
        let units = (Lock::<OwnerId>::Uninit.to_repr(), Lock::<OwnerId>::Free.to_repr());
        assert_eq!(units, (1, 2), "`Uninit` and `Free` take 1 and 2");
        assert_eq!(Lock::Owned(OwnerId(RangedU64::MIN)).to_repr(), 3, "an owner its id");
        assert_eq!(None::<Lock<OwnerId>>.to_repr(), 0, "and `None` zero");
        assert_eq!(Lock::<OwnerId>::from_repr(2), Some(Lock::Free), "each read back");
        let owned = Lock::Owned(OwnerId(RangedU64::new(9).expect("9 is from 3 up")));
        LOCK.store(Some(owned), Release);
        assert_eq!(LOCK.load(Acquire), Some(owned), "through a static");
    }

    #[test]
    fn an_enum_whose_units_need_a_tag_takes_one() {
        assert_eq!(Lock::<u8>::Free.to_repr(), 1 << 8, "a byte leaves no repr spare");
        assert_eq!(Lock::Owned(7_u8).to_repr(), 2 << 8 | 7, "so each variant's index is its tag");
        assert_eq!(Lock::<u8>::from_repr(3 << 8), None, "and no variant takes tag 3");
        assert_eq!(Lock::Owned(Marker).to_repr(), 0, "but a payload of no bits keeps zero");
        assert_eq!(Lock::<Marker>::Free.to_repr(), 2, "where units in two bits tie a tag");
    }

    #[test]
    fn an_enum_promises_zero_decodes_where_a_unit_takes_it() {
        repr_and_validity_are::<LockWord<u32>, u64, ZeroValid>();
        repr_and_validity_are::<LockWord<OwnerId>, u64, ZeroValid>();
        assert_eq!(LockWord::<u32>::from_repr(0), Some(LockWord::Unbuilt), "zero is `Unbuilt`");
        assert_eq!(LockWord::Owned(5_u32).to_repr(), 2 << 32 | 5, "tag 2 above a `u32`'s bits");
        assert_eq!(LockWord::<u32>::from_repr(1 << 32 | 1), None, "`Free` holds nothing");
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
        COUNT.fetch_add(2, Relaxed);
        assert_eq!(COUNT.load(Relaxed), Wrap(3), "add");
        COUNT.or(Wrap(0b100), Relaxed);
        assert_eq!(COUNT.load(Relaxed), Wrap(0b111), "then or");
    }

    // x86_64 has no atomic maximum, so `fetch_max` exists on aarch64 alone.
    #[cfg(target_arch = "aarch64")]
    #[test]
    fn atom_ord_brings_fetch_max_where_the_target_has_one() {
        let count = Atomic::new(Wrap(1_u32));
        count.fetch_max(Wrap(9), Relaxed);
        assert_eq!(count.load(Relaxed), Wrap(9), "the larger");
    }

    #[test]
    fn variants_named_as_the_derives_items_are_stored_as_any_others() {
        assert_eq!(Named::layout(Sign::Plus).to_repr(), 1, "a sign as it is");
        assert_eq!(Named::<Sign>::Repr.to_repr(), 0xFFF6, "the units below it, from -10");
        decodes_exactly_its_values::<Named<Sign>, _>(u16::MIN..=u16::MAX, 9 + 3);
        assert_eq!(Named::layout(7_u8).to_repr(), 3 << 8 | 7, "but a byte's above a tag");
        decodes_exactly_its_values::<Named<u8>, _>(u16::MIN..=u16::MAX, 9 + 256);
    }

    #[test]
    fn an_instance_names_atomix_by_the_path_stated() {
        repr_and_validity_are::<Duo<u8, bool>, u32, ZeroValid>();
        assert_eq!(Duo(7_u8, true).to_repr(), 7 | 1 << 8, "the flag above the byte");
        repr_and_validity_are::<Slot<u8>, u32, ZeroValid>();
        assert_eq!(Slot::Full(7_u8).to_repr(), 1 << 8 | 7, "tag 1 above the byte");
        decodes_exactly_its_values::<Slot<u8>, _>(0..=u32::from(u16::MAX), 1 + 256);
    }
}
