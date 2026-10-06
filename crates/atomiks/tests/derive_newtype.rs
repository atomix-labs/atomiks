//! `#[derive(Atom)]` on a newtype: its repr, range, validity and conversions are its field's, a
//! static of it needs no feature gate, and the capability derives bring their read-modify-writes.

#![cfg(feature = "derive")]
// Loom's `Atomic::new` is not `const`, so no static of one builds under it; `derive_laws.rs` builds
// the derive's code there.
#![cfg(not(loom))]

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
    use atomiks::validity::{Partial, Total, TotalZeroNiche};
    use atomiks::{Atom, AtomAdd, AtomBitwise, AtomOrd, Atomic, RangedU64, ReprRange};

    use crate::testing::atom::repr_and_validity_are;

    /// A sequence number.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Seq(u64);

    /// An order's id, never zero.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct OrderId {
        /// The id.
        id: NonZero<u64>,
    }

    /// A lock's owner, from 3: a lock keeps 1 and 2 for its own states. It orders as its field.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Atom, AtomOrd)]
    struct OwnerId(RangedU64<3>);

    /// A count, which adds, orders and combines as its field does.
    #[derive(
        Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Atom, AtomAdd, AtomOrd, AtomBitwise,
    )]
    struct Count(u32);

    /// What an id names.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Venue {}

    /// An id of something of kind `K`, between two markers of it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Tagged<K>(PhantomData<K>, u16, PhantomData<fn() -> K>);

    /// A timestamp, in the repr it states.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u64)]
    struct Stamp(u64);

    /// A flag that names atomiks by the crate's other name.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(crate = renamed)]
    struct Live(bool);

    // Named as the derive's locals are: were those named at the call site, each would resolve to
    // its constant here, and match it as a pattern rather than bind.
    #[expect(
        non_upper_case_globals,
        dead_code,
        reason = "a name the derive's locals must not take"
    )]
    const repr: u64 = 9;
    #[expect(
        non_upper_case_globals,
        dead_code,
        reason = "a name the derive's locals must not take"
    )]
    const field: u64 = 9;

    static SEQ: Atomic<Seq> = Atomic::new(Seq(1));
    static OWNER: Atomic<Option<OrderId>> = Atomic::new(None);
    static LOCK_OWNER: Atomic<Option<OwnerId>> = Atomic::new(None);
    static COUNT: Atomic<Count> = Atomic::new(Count(1));
    static VENUE: Atomic<Tagged<Venue>> = Atomic::new(Tagged(PhantomData, 7, PhantomData));
    static LIVE: Atomic<Live> = Atomic::new(Live(true));

    /// Compiles only where `T` has every capability.
    const fn has_every_capability<T: AtomAdd + AtomOrd + AtomBitwise>() {}

    #[test]
    fn a_newtype_round_trips_as_its_field() {
        assert_eq!(Seq(u64::MAX).to_repr(), u64::MAX, "the field's repr");
        assert_eq!(Seq::from_repr(7), Some(Seq(7)), "decoded as the field");
        let id = OrderId { id: NonZero::<u64>::MIN };
        assert_eq!(OrderId::from_repr(id.to_repr()), Some(id), "and back");
        assert_eq!(OrderId::from_repr(0), None, "refusing what the field refuses");
    }

    #[test]
    fn a_newtype_takes_its_fields_repr_range_and_validity() {
        repr_and_validity_are::<Seq, u64, Total>();
        repr_and_validity_are::<OrderId, u64, TotalZeroNiche>();
        repr_and_validity_are::<Stamp, u64, Total>();
        assert_eq!(Seq::REPRS, ReprRange::FULL, "every repr of a `u64`");
        assert_eq!(OrderId::REPRS, ReprRange::NONZERO, "every one but zero");
    }

    #[test]
    fn none_takes_zero_where_the_field_never_does() {
        assert_eq!(None::<OrderId>.to_repr(), 0, "`None`");
        assert_eq!(OWNER.load(Acquire), None, "stored as zero");
        let id = OrderId { id: NonZero::<u64>::MAX };
        OWNER.store(Some(id), Release);
        assert_eq!(OWNER.load(Acquire), Some(id), "beside every id");
    }

    #[test]
    fn a_newtype_over_a_ranged_integer_takes_its_range() {
        repr_and_validity_are::<OwnerId, u64, Partial>();
        assert_eq!(OwnerId::REPRS, ReprRange::new(3, u128::from(u64::MAX)), "from 3 up");
        assert_eq!(OwnerId::from_repr(2), None, "refusing 2, below it");
    }

    #[test]
    fn none_takes_zero_past_a_ranged_fields_largest() {
        assert_eq!(None::<OwnerId>.to_repr(), 0, "`None`, past `u64::MAX`");
        assert_eq!(LOCK_OWNER.load(Acquire), None, "stored as zero");
        LOCK_OWNER.store(Some(OwnerId(RangedU64::MIN)), Release);
        assert_eq!(LOCK_OWNER.load(Acquire), Some(OwnerId(RangedU64::MIN)), "beside the first id");
    }

    #[test]
    fn markers_take_no_repr() {
        repr_and_validity_are::<Tagged<Venue>, u16, Total>();
        assert_eq!(VENUE.load(Acquire), Tagged(PhantomData, 7, PhantomData), "the id alone");
    }

    #[test]
    fn statics_hold_derived_values() {
        SEQ.store(Seq(2), Release);
        assert_eq!(SEQ.load(Acquire), Seq(2), "the sequence number stored");
        assert_eq!(LIVE.load(Acquire), Live(true), "and the flag named through `renamed`");
    }

    #[test]
    fn the_capability_derives_bring_their_read_modify_writes() {
        has_every_capability::<Count>();
        COUNT.fetch_add(2, Relaxed);
        assert_eq!(COUNT.load(Relaxed), Count(3), "add");
        COUNT.or(Count(0b100), Relaxed);
        assert_eq!(COUNT.load(Relaxed), Count(0b111), "then or");
    }

    // x86_64 has no atomic maximum, so `fetch_max` exists on aarch64 alone.
    #[cfg(target_arch = "aarch64")]
    #[test]
    fn atom_ord_brings_fetch_max_where_the_target_has_one() {
        let count = Atomic::new(Count(1));
        count.fetch_max(Count(9), Relaxed);
        assert_eq!(count.load(Relaxed), Count(9), "the larger");
        let owner = Atomic::new(OwnerId(RangedU64::MIN));
        owner.fetch_max(OwnerId(RangedU64::MAX), Relaxed);
        assert_eq!(owner.load(Relaxed), OwnerId(RangedU64::MAX), "and the larger id");
    }
}
