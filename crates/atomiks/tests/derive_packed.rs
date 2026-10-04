//! `#[derive(Atom)]` on a struct of several fields: each field packs into bits of its own, from
//! bit 0 in declaration order, in the narrowest repr that holds them or the one stated; the bits
//! above extend the top field, so a signed one's sign fills them; every repr no value encodes to is
//! refused, exhaustively for 8- and 16-bit structs; and a static of each needs no feature gate.

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

    use atomiks::ordering::{Acquire, Release};
    use atomiks::validity::{Partial, Total, ZeroNiche, ZeroValid};
    use atomiks::{Atom, Atomic, RangedI8, ReprRange};

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

    /// What a field of no bits names.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Venue {}

    /// A resting quote: 32 bits of quantity, then a bit of side, then one of whether it is live.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Quote {
        /// How many.
        qty: u32,
        /// Which side.
        side: Side,
        /// Whether it may fill.
        live: bool,
    }

    /// A move: a byte of length below two bits of sign, its top field, so ten bits that extend
    /// the sign.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Step {
        /// How far.
        length: u8,
        /// Which way.
        sign: Sign,
    }

    /// A move on a side: the side's bit below four bits of ticks, -5 to 5, its top field, which
    /// extend their sign.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct PriceMove {
        /// Which side.
        side: Side,
        /// How far, and which way.
        ticks: RangedI8<-5, 5>,
    }

    /// A byte and a side, with fields of no bits between them, which take none.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Gapped {
        /// A byte.
        low: u8,
        /// Nothing.
        gap: (),
        /// Nothing, derived.
        mark: Marker,
        /// Nothing, of a kind.
        kind: PhantomData<Venue>,
        /// A bit.
        side: Side,
    }

    /// A flag, a side and a sign by position: four bits of a byte, which extend the sign.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Flags(bool, Side, Sign);

    /// A side and a flag: two bits of a byte, zeros above them.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Book {
        /// Which side.
        side: Side,
        /// Whether it is open.
        open: bool,
    }

    /// Two halves that fill their repr, every pattern of which decodes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Halves {
        /// The low half.
        low: u16,
        /// The high half.
        high: u16,
    }

    /// A quote nested whole, below an id never zero.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Order {
        /// What rests.
        quote: Quote,
        /// Its id.
        id: NonZero<u16>,
    }

    /// A quote or none, below a sequence number.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Slot {
        /// What the slot holds.
        quote: Option<Quote>,
        /// When it was written.
        seq: u16,
    }

    /// Nine bits, pinned to 32.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u32)]
    struct Pinned {
        /// How far.
        length: u8,
        /// Whether it is live.
        live: bool,
    }

    /// A step pinned to a signed repr, which holds its extended sign alike.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = i16)]
    struct Signed {
        /// How far.
        length: u8,
        /// Which way.
        sign: Sign,
    }

    /// Fields named as a conversion's locals, which the derive reads as members alone.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Named {
        /// As a conversion's parameter.
        repr: u8,
        /// As the bits a conversion reads.
        bits: bool,
    }

    /// A level of a book, naming atomiks by the crate's other name.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(crate = renamed)]
    struct Level {
        /// How deep.
        depth: u8,
        /// Which side.
        side: Side,
    }

    static QUOTE: Atomic<Quote> = Atomic::new(Quote { qty: 0, side: Side::Bid, live: false });
    static STEP: Atomic<Option<Step>> = Atomic::new(None);
    static ORDER: Atomic<Option<Order>> = Atomic::new(None);
    static SLOT: Atomic<Slot> = Atomic::new(Slot { quote: None, seq: 0 });

    #[test]
    fn each_field_packs_from_bit_zero_in_declaration_order() {
        repr_and_validity_are::<Quote, u64, ZeroValid>();
        let quote = Quote { qty: 5, side: Side::Ask, live: true };
        assert_eq!(quote.to_repr(), 5 | 1 << 32 | 1 << 33, "34 bits, the quantity lowest");
        assert_eq!(Quote::REPRS, ReprRange::new(0, (1 << 34) - 1), "zeros above them");
        assert_eq!(Quote::from_repr(quote.to_repr()), Some(quote), "and back");
        assert_eq!(Quote::from_repr(1 << 34), None, "refusing a bit above them");
    }

    #[test]
    fn a_signed_top_field_extends_its_sign_above_the_fields() {
        repr_and_validity_are::<Step, u16, ZeroValid>();
        let back = Step { length: 5, sign: Sign::Minus };
        assert_eq!(back.to_repr(), 0xFF05, "-1 extends from bit 9");
        assert_eq!(Step { length: 5, sign: Sign::Plus }.to_repr(), 0x0105, "1 extends zeros");
        assert_eq!(Step::REPRS, ReprRange::from_signed(-256, 511), "so the range wraps");
        assert_eq!(Step::from_repr(0x0305), None, "and bits that do not extend it are refused");
        repr_and_validity_are::<Signed, i16, ZeroValid>();
        let signed = Signed { length: 5, sign: Sign::Minus };
        assert_eq!(signed.to_repr(), -251, "alike in a signed repr");
    }

    #[test]
    fn a_ranged_field_takes_the_bits_its_range_needs() {
        assert_eq!(field_width(RangedI8::<-5, 5>::REPRS), 4, "-5 to 5 in four bits, signed");
        repr_and_validity_are::<PriceMove, u8, Partial>();
        let down = PriceMove { side: Side::Ask, ticks: RangedI8::MIN };
        assert_eq!(down.to_repr(), 0xF7, "-5 above the side, its sign extended from bit 4");
        let up = PriceMove { side: Side::Bid, ticks: RangedI8::MAX };
        assert_eq!(up.to_repr(), 0x0A, "and 5, zeros above it");
        assert_eq!(PriceMove::from_repr(0x0C), None, "refusing 6, past 5");
    }

    #[test]
    fn option_of_a_signed_top_field_takes_a_repr_without_another_bit() {
        assert_eq!(None::<Step>.to_repr(), 0xFEFF, "`None` below the range");
        assert_eq!(field_width(Option::<Step>::REPRS), field_width(Step::REPRS), "ten bits still");
    }

    #[test]
    fn fields_of_no_bits_take_none() {
        repr_and_validity_are::<Gapped, u16, ZeroValid>();
        let gapped = Gapped { low: 7, gap: (), mark: Marker, kind: PhantomData, side: Side::Ask };
        assert_eq!(gapped.to_repr(), 7 | 1 << 8, "the side just above the byte");
        assert_eq!(Gapped::REPRS, ReprRange::new(0, 0x1FF), "nine bits");
    }

    #[test]
    fn every_repr_of_an_8_or_16_bit_struct_decodes_exactly_where_a_value_encodes_to_it() {
        decodes_exactly_its_values::<Flags, _>(u8::MIN..=u8::MAX, 2 * 2 * 3);
        decodes_exactly_its_values::<Book, _>(u8::MIN..=u8::MAX, 2 * 2);
        decodes_exactly_its_values::<PriceMove, _>(u8::MIN..=u8::MAX, 2 * 11);
        decodes_exactly_its_values::<Step, _>(u16::MIN..=u16::MAX, 256 * 3);
        decodes_exactly_its_values::<Gapped, _>(u16::MIN..=u16::MAX, 256 * 2);
        decodes_exactly_its_values::<Signed, _>(i16::MIN..=i16::MAX, 256 * 3);
    }

    #[test]
    fn fields_filling_their_repr_with_every_pattern_are_total() {
        repr_and_validity_are::<Halves, u32, Total>();
        let halves = Halves { low: 0xBEEF, high: 0xDEAD };
        assert_eq!(halves.to_repr(), 0xDEAD_BEEF, "the high half above");
        assert_eq!(Halves::REPRS, ReprRange::FULL, "every repr");
    }

    #[test]
    fn a_stated_repr_pins_the_size() {
        repr_and_validity_are::<Pinned, u32, ZeroValid>();
        assert_eq!(Pinned { length: 3, live: true }.to_repr(), 3 | 1 << 8, "nine bits of 32");
    }

    #[test]
    fn a_packed_value_and_an_option_pack_as_fields() {
        repr_and_validity_are::<Order, u64, ZeroNiche>();
        let order =
            Order { quote: Quote { qty: 9, side: Side::Bid, live: true }, id: NonZero::<u16>::MIN };
        assert_eq!(order.to_repr(), 9 | 1 << 33 | 1 << 34, "the id above the quote's 34 bits");
        assert_eq!(None::<Order>.to_repr(), 0, "and `None` zero, which no id is");
        repr_and_validity_are::<Slot, u64, ZeroValid>();
        let empty = Slot { quote: None, seq: 1 };
        assert_eq!(empty.to_repr(), 1 << 34 | 1 << 35, "`None` past the quote's range, then 1");
        assert_eq!(Slot::from_repr(empty.to_repr()), Some(empty), "and back");
    }

    #[test]
    fn statics_hold_packed_values() {
        let quote = Quote { qty: 9, side: Side::Ask, live: true };
        QUOTE.store(quote, Release);
        assert_eq!(QUOTE.load(Acquire), quote, "a quote");
        STEP.store(Some(Step { length: 2, sign: Sign::Minus }), Release);
        assert_eq!(STEP.load(Acquire), Some(Step { length: 2, sign: Sign::Minus }), "a step");
        assert_eq!(ORDER.load(Acquire), None, "no order yet");
        SLOT.store(Slot { quote: Some(quote), seq: 4 }, Release);
        assert_eq!(SLOT.load(Acquire), Slot { quote: Some(quote), seq: 4 }, "and a slot");
    }

    #[test]
    fn fields_named_as_the_derives_locals_are_packed_as_any_others() {
        assert_eq!(Named { repr: 3, bits: true }.to_repr(), 3 | 1 << 8, "the flag above the byte");
        decodes_exactly_its_values::<Named, _>(u16::MIN..=u16::MAX, 256 * 2);
    }

    #[test]
    fn a_packed_struct_names_atomiks_by_the_path_stated() {
        decodes_exactly_its_values::<Level, _>(u16::MIN..=u16::MAX, 256 * 2);
    }

    /// What a 128-bit repr holds, where an atomic word does.
    #[cfg(any(
        target_arch = "aarch64",
        all(target_arch = "x86_64", target_feature = "cmpxchg16b")
    ))]
    mod wide {
        use atomiks::ordering::{Acquire, Release};
        use atomiks::validity::Total;
        use atomiks::{Atom, Atomic};

        use crate::testing::atom::repr_and_validity_are;

        /// A sequence number and the value it numbers: 128 bits.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
        struct Pair {
            /// The sequence number.
            seq: u64,
            /// The value.
            value: u64,
        }

        static PAIR: Atomic<Pair> = Atomic::new(Pair { seq: 0, value: 0 });

        #[test]
        fn two_u64s_fill_a_u128() {
            repr_and_validity_are::<Pair, u128, Total>();
            assert_eq!(Pair { seq: 1, value: 2 }.to_repr(), 1 | 2 << 64, "the value above");
            PAIR.store_rmw(Pair { seq: 3, value: u64::MAX }, Release);
            assert_eq!(PAIR.load_rmw(Acquire), Pair { seq: 3, value: u64::MAX }, "and stored");
        }
    }
}
