//! `#[derive(Atom)]` on an enum with fields: each variant's fields pack from bit 0, as a struct's,
//! below a tag of its discriminant above the widest variant's; or, where one variant alone has
//! fields and none states a discriminant, the unit variants take the reprs beside its top field's
//! range, as `Option`'s `None` takes one, wherever that is no wider than a tag. Every repr no value
//! encodes to is refused, exhaustively for 8- and 16-bit enums, and a static of each needs no
//! feature gate.

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
    use core::num::NonZero;

    use atomiks::ordering::{Acquire, Release};
    use atomiks::validity::{Total, ZeroNiche, ZeroValid};
    use atomiks::{Atom, Atomic, ReprRange};

    use crate::testing::atom::{decodes_exactly_its_values, field_width, repr_and_validity_are};

    /// The side of the book an order rests on: one bit.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Side {
        /// A buy.
        Bid,
        /// A sell.
        Ask,
    }

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

    /// An owner's id, never zero.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct OwnerId(NonZero<u64>);

    /// A ring buffer's slot: two variants with fields, so a tag of two bits above the 32 of a lap.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Slot {
        /// Nothing written.
        Empty,
        /// Being written, in a lap.
        Writing {
            /// The lap.
            lap: u32,
        },
        /// Written, in a lap.
        Ready {
            /// The lap.
            lap: u32,
        },
    }

    /// A gate held by an owner: `Closed` takes zero beside the ids, which a tag would widen.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Gate {
        /// No owner.
        Closed,
        /// Held by its owner.
        Open(OwnerId),
    }

    /// A reading of a sign or none: `Missing` takes -2, below the sign's range, in two bits.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Reading {
        /// No reading yet.
        Missing,
        /// The sign read.
        Present(Sign),
    }

    /// A quote on a side, or closed: `Closed` takes 2, beside the side's bit, where a tag would
    /// take two bits alike.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Quote {
        /// Not quoting.
        Closed,
        /// Quoting on a side.
        Open(Side),
    }

    /// A mark set or not, its payload of no bits: the units above it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Mark {
        /// Never set.
        Unset,
        /// Set, then cleared.
        Cleared,
        /// Set.
        Set(Marker),
    }

    /// An order: a quantity below a sign, its top field, beside which `Pending` takes -2.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Order {
        /// Not yet filled.
        Pending,
        /// Filled.
        Filled {
            /// How many.
            quantity: u8,
            /// Which way the price moved.
            sign: Sign,
        },
    }

    /// Units written with parentheses or braces, beside a byte: tagged.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Mode {
        /// Off.
        Off(),
        /// On.
        On {},
        /// Busy, for a while.
        Busy(u8),
        /// Idle.
        Idle,
    }

    /// How far a discriminant moves, named as a variant the derive names a constant after.
    const OFFSET: i8 = 2;

    /// A shift back or ahead by a byte: negative and computed discriminants, so a tag of four bits,
    /// signed, above the byte.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[repr(i8)]
    #[expect(
        clippy::upper_case_acronyms,
        reason = "named as the constant `OFFSET`, which its discriminants read as the user's"
    )]
    enum Shift {
        /// Back by a byte.
        OFFSET(u8) = -1,
        /// Still.
        Still,
        /// Ahead by a byte.
        Ahead(u8) = OFFSET + 3,
    }

    /// A level above a base the enum states, which its discriminants name through `Self`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[repr(u8)]
    enum Level {
        /// The base.
        Base = Self::BASE,
        /// Above it, by a byte.
        Above(u8),
    }

    impl Level {
        /// The base's discriminant.
        const BASE: u8 = 4;
    }

    /// A flag whose variant at tag zero holds an id never zero, so zero never decodes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[repr(u8)]
    enum Flag {
        /// Raised by an id.
        Raised(NonZero<u8>) = 0,
        /// Lowered.
        Lowered = 1,
    }

    /// Seven flags on either side: every pattern of a byte decodes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Septet {
        /// On the left.
        Left(bool, bool, bool, bool, bool, bool, bool),
        /// On the right.
        Right(bool, bool, bool, bool, bool, bool, bool),
    }

    /// The two ends of a byte of tags, stated: the tags between them are no variant's.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[repr(u16)]
    enum Edge {
        /// The lowest.
        Low() = 0,
        /// The highest.
        High() = 255,
    }

    /// A slot beside a sequence number, nested whole.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Entry {
        /// What the slot holds.
        slot: Slot,
        /// When it was written.
        seq: u16,
    }

    /// Variants named as the items the derive names, three with fields, so tagged: none of their
    /// names clashes with one of the derive's.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[expect(non_camel_case_types, reason = "named as the derive's items, lowercase ones too")]
    enum Named {
        /// As the repr.
        Repr,
        /// As a conversion's parameter.
        repr(u8),
        /// As the bits a conversion reads.
        bits,
        /// As the layout.
        layout(Side),
        /// As a field's value.
        value_0,
        /// As the function that lays an instance out.
        lay_out,
        /// As a variant's layout.
        variant_0 {
            /// Whether it is set.
            set: bool,
        },
        /// As a field's placement.
        placement_0,
        /// As the discriminants' integer.
        Discriminant,
        /// As a discriminant.
        discriminant_0,
    }

    /// A signal raised or lowered on a side, naming atomiks by the crate's other name: tagged.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(crate = renamed)]
    enum Signal {
        /// Never raised.
        Off,
        /// Raised on a side.
        Raised(Side),
        /// Lowered on a side.
        Lowered(Side),
    }

    static SLOT: Atomic<Option<Slot>> = Atomic::new(None);
    static GATE: Atomic<Gate> = Atomic::new(Gate::Closed);
    static READING: Atomic<Option<Reading>> = Atomic::new(None);
    static ENTRY: Atomic<Entry> = Atomic::new(Entry { slot: Slot::Empty, seq: 0 });

    #[test]
    fn a_tag_lies_above_the_widest_variant_and_spares_a_repr_for_none() {
        repr_and_validity_are::<Slot, u64, ZeroValid>();
        assert_eq!(Slot::Writing { lap: 7 }.to_repr(), 1 << 32 | 7, "tag 1 at bit 32");
        assert_eq!(Slot::Ready { lap: 7 }.to_repr(), 2 << 32 | 7, "tag 2");
        assert_eq!(Slot::Empty.to_repr(), 0, "and `Empty`'s tag 0, its lap bits clear");
        assert_eq!(Slot::REPRS, ReprRange::new(0, 2 << 32 | 0xFFFF_FFFF), "the spare tag above");
        assert_eq!(None::<Slot>.to_repr(), 3 << 32, "which `None` takes");
        assert_eq!(field_width(Option::<Slot>::REPRS), 34, "so `Option` adds no bit");
        assert_eq!(Slot::from_repr(3 << 32), None, "no fourth variant");
        assert_eq!(Slot::from_repr(1), None, "`Empty` holds no lap");
        assert_eq!(Slot::from_repr(1 << 34), None, "nor a bit above the tag");
    }

    #[test]
    fn units_beside_the_payloads_range_beat_a_tag() {
        repr_and_validity_are::<Gate, u64, ZeroValid>();
        assert_eq!(Gate::Closed.to_repr(), 0, "`Closed` takes zero, which no id is");
        let id = OwnerId(NonZero::<u64>::MAX);
        assert_eq!(Gate::Open(id).to_repr(), u64::MAX, "an id is stored as it is");
        assert_eq!(Gate::REPRS, ReprRange::FULL, "in 64 bits, where a tag would take 65");
        assert_eq!(
            Gate::from_repr(5),
            Some(Gate::Open(OwnerId(NonZero::<u64>::new(5).expect("5")))),
            "an id"
        );
    }

    #[test]
    fn a_unit_beside_a_signed_payload_takes_the_repr_below_it() {
        repr_and_validity_are::<Reading, u8, ZeroValid>();
        assert_eq!(Reading::Missing.to_repr(), 0xFE, "-2, below the sign's -1");
        assert_eq!(Reading::Present(Sign::Minus).to_repr(), 0xFF, "-1, sign-extended");
        assert_eq!(field_width(Reading::REPRS), 2, "two bits, signed");
        assert_eq!(Reading::REPRS, ReprRange::from_signed(-2, 1), "so the range wraps");
        assert_eq!(None::<Reading>.to_repr(), 0xFD, "and `None` takes -3");
    }

    #[test]
    fn units_fill_a_niche_where_a_tag_is_no_narrower() {
        repr_and_validity_are::<Quote, u8, ZeroValid>();
        assert_eq!(Quote::Closed.to_repr(), 2, "after the side's 0 and 1, in two bits as a tag");
        assert_eq!(Quote::Open(Side::Ask).to_repr(), 1, "the side as it is");
        assert_eq!(Mark::Set(Marker).to_repr(), 0, "a payload of no bits keeps zero");
        assert_eq!((Mark::Unset.to_repr(), Mark::Cleared.to_repr()), (1, 2), "the units above");
    }

    #[test]
    fn units_beside_a_top_field_above_others_clear_the_bits_below_it() {
        repr_and_validity_are::<Order, u16, ZeroValid>();
        assert_eq!(Order::Pending.to_repr(), 0xFE00, "-2 above a clear byte, sign-extended");
        let filled = Order::Filled { quantity: 3, sign: Sign::Minus };
        assert_eq!(filled.to_repr(), 0xFF03, "the quantity below the sign");
        assert_eq!(Order::from_repr(0xFE01), None, "a unit holds no quantity");
    }

    #[test]
    fn stated_discriminants_are_the_tags_as_rustc_evaluates_them() {
        repr_and_validity_are::<Shift, u16, ZeroValid>();
        assert_eq!(Shift::Still.to_repr(), 0, "-1 plus one");
        assert_eq!(Shift::Ahead(7).to_repr(), 5 << 8 | 7, "the user's `OFFSET` plus 3, not -1's");
        assert_eq!(Shift::OFFSET(7).to_repr(), 0xFF07, "-1, sign-extended from a tag of four bits");
        assert_eq!(field_width(Shift::REPRS), 12, "above the byte");
        assert_eq!(Shift::from_repr(1 << 8), None, "no variant takes tag 1");
        assert_eq!(Level::Above(1).to_repr(), 5 << 8 | 1, "`Self::BASE` plus one, above a byte");
        repr_and_validity_are::<Flag, u16, ZeroNiche>();
        assert_eq!(None::<Flag>.to_repr(), 0, "zero never decodes, so `None` takes it");
    }

    #[test]
    fn units_written_with_parentheses_or_braces_are_units() {
        repr_and_validity_are::<Mode, u16, ZeroValid>();
        assert_eq!(Mode::On {}.to_repr(), 1 << 8, "tag 1, above the byte");
        assert_eq!(Mode::Busy(9).to_repr(), 2 << 8 | 9, "tag 2");
        assert_eq!(Mode::from_repr(0), Some(Mode::Off()), "and `Off` zero");
    }

    #[test]
    fn an_enum_is_total_where_its_variants_take_every_tag_and_every_bit_below() {
        repr_and_validity_are::<Septet, u8, Total>();
        let right = Septet::Right(true, false, false, false, false, false, false);
        assert_eq!(right.to_repr(), 0x81, "the tag above seven flags");
        repr_and_validity_are::<Edge, u8, ZeroValid>();
        assert_eq!(Edge::High().to_repr(), 0xFF, "but two tags whose range fills a byte are not");
    }

    #[test]
    fn every_repr_of_an_8_or_16_bit_enum_decodes_exactly_where_a_value_encodes_to_it() {
        decodes_exactly_its_values::<Reading, _>(u8::MIN..=u8::MAX, 1 + 3);
        decodes_exactly_its_values::<Quote, _>(u8::MIN..=u8::MAX, 1 + 2);
        decodes_exactly_its_values::<Mark, _>(u8::MIN..=u8::MAX, 2 + 1);
        decodes_exactly_its_values::<Septet, _>(u8::MIN..=u8::MAX, 2 * 128);
        decodes_exactly_its_values::<Edge, _>(u8::MIN..=u8::MAX, 2);
        decodes_exactly_its_values::<Order, _>(u16::MIN..=u16::MAX, 1 + 256 * 3);
        decodes_exactly_its_values::<Shift, _>(u16::MIN..=u16::MAX, 256 + 1 + 256);
        decodes_exactly_its_values::<Level, _>(u16::MIN..=u16::MAX, 1 + 256);
        decodes_exactly_its_values::<Flag, _>(u16::MIN..=u16::MAX, 255 + 1);
        decodes_exactly_its_values::<Mode, _>(u16::MIN..=u16::MAX, 3 + 256);
    }

    #[test]
    fn variants_named_as_the_derives_items_are_stored_as_any_others() {
        assert_eq!(Named::repr(7).to_repr(), 1 << 8 | 7, "tag 1 above a byte");
        assert_eq!(Named::variant_0 { set: true }.to_repr(), 6 << 8 | 1, "tag 6");
        decodes_exactly_its_values::<Named, _>(u16::MIN..=u16::MAX, 7 + 256 + 2 + 2);
    }

    #[test]
    fn an_enum_with_fields_names_atomiks_by_the_path_stated() {
        assert_eq!(Signal::Lowered(Side::Ask).to_repr(), 2 << 1 | 1, "tag 2 above a side");
        decodes_exactly_its_values::<Signal, _>(u8::MIN..=u8::MAX, 1 + 2 + 2);
    }

    #[test]
    fn statics_hold_enums_with_fields() {
        SLOT.store(Some(Slot::Ready { lap: 4 }), Release);
        assert_eq!(SLOT.load(Acquire), Some(Slot::Ready { lap: 4 }), "a slot");
        let id = OwnerId(NonZero::<u64>::MIN);
        GATE.store(Gate::Open(id), Release);
        assert_eq!(GATE.load(Acquire), Gate::Open(id), "a gate");
        assert_eq!(READING.load(Acquire), None, "no reading");
        READING.store(Some(Reading::Missing), Release);
        assert_eq!(READING.load(Acquire), Some(Reading::Missing), "then a missing one");
        let entry = Entry { slot: Slot::Writing { lap: 1 }, seq: 2 };
        ENTRY.store(entry, Release);
        assert_eq!(ENTRY.load(Acquire), entry, "and an entry nesting a slot");
    }
}
