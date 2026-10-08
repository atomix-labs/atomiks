//! `#[derive(Atom)]` on a fieldless enum and on a zero-width struct: each variant is stored as its
//! discriminant, read exactly as rustc evaluates it, in the repr its `#[repr]` names or the
//! narrowest that holds every one; the range encloses them, the validity says what they take of the
//! repr, every other repr is refused, and a static of each needs no feature gate.

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
    use core::ffi::c_int;
    use core::marker::PhantomData;

    use atomix::ordering::{Acquire, Release};
    use atomix::validity::{Total, TotalZeroNiche, ZeroNiche, ZeroValid};
    use atomix::{Atom, Atomic, ReprRange};

    use crate::testing::atom::{
        decodes_exactly, field_width, repr_and_validity_are, with_every_byte,
    };

    /// The side of the book an order rests on.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Side {
        /// A buy.
        Bid,
        /// A sell.
        Ask,
    }

    /// Which way a price moved, stored as its discriminant: `Minus` as `0xFF`.
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

    /// The two ends of a byte, whose range wraps through zero.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[repr(u8)]
    enum Edge {
        /// The lowest byte.
        Low  = 0,
        /// The highest.
        High = 255,
    }

    /// Which way a book leans, with no `#[repr]`: its negative discriminant is stored in
    /// the narrowest repr, sign-extended.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Tilt {
        /// To the bids.
        Left = -1,
        /// Neither.
        Level,
        /// To the asks.
        Right,
    }

    /// Two discriminants that need nine bits, signed, with no `#[repr]`: stored in a `u16`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Swing {
        /// Far down.
        Low  = -200,
        /// Far up.
        High = 200,
    }

    /// A discriminant past 32 bits, with no `#[repr]`: stored in a `u64`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[expect(
        clippy::enum_clike_unportable_variant,
        reason = "atomix builds for 64-bit targets alone"
    )]
    enum Scale {
        /// One.
        Unit = 1,
        /// A tebi.
        Tebi = 1 << 40,
    }

    /// A constant a discriminant names.
    const OFFSET: u16 = 10;

    /// Holds `LOW`, an associated constant a discriminant names.
    struct Limits;

    impl Limits {
        /// A discriminant.
        const LOW: u16 = 1 << 3;
    }

    /// Levels whose discriminants are constant expressions naming the user's own items.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[repr(u16)]
    enum Level {
        /// The user's `Limits::LOW`, 8.
        Low  = Limits::LOW,
        /// The user's `OFFSET` and two, 12.
        High = OFFSET + 2,
        /// One past `High`, by rustc's rule.
        Next,
    }

    /// A mode shared with C, whose discriminants are C's `int`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[repr(C)]
    enum Mode {
        /// Off.
        Off,
        /// On, apart from `Off`.
        On = 4,
    }

    /// A state whose repr is pinned wider than its discriminants need.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u32)]
    enum Phase {
        /// Before the open.
        Pre = 1,
        /// Trading.
        Open,
    }

    /// Declares `Byte`, a variant for every byte; `NonZeroByte`, one for every byte but zero; and
    /// `NonMaxByte`, one for every byte but 255, each variant documented with its name.
    macro_rules! byte_enums {
        ($zero:ident $one:ident $($rest:ident)*) => {
            /// A variant for every byte, each its own discriminant.
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
            #[repr(u8)]
            enum Byte {
                #[doc = stringify!($zero)]
                $zero,
                #[doc = stringify!($one)]
                $one,
                $(#[doc = stringify!($rest)] $rest,)*
            }

            /// A variant for every byte but zero, which `Option`'s `None` takes.
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
            #[repr(u8)]
            enum NonZeroByte {
                #[doc = stringify!($one)]
                $one = 1,
                $(#[doc = stringify!($rest)] $rest,)*
            }

            /// A variant for every byte but 255, which `Option`'s `None` takes: `B001` is 0.
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
            #[repr(u8)]
            enum NonMaxByte {
                #[doc = stringify!($one)]
                $one = 0,
                $(#[doc = stringify!($rest)] $rest,)*
            }
        };
    }

    with_every_byte!(byte_enums);

    /// Stores nothing: its one value is zero.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Marker;

    /// Marks a kind `K`, storing nothing.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Tag<K>(PhantomData<K>);

    /// Stores nothing, written with braces.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[expect(
        clippy::empty_structs_with_brackets,
        reason = "the derive builds a braced struct's value apart from a unit struct's"
    )]
    struct Braced {}

    /// Stores nothing, written as a tuple of no fields.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[expect(
        clippy::empty_structs_with_brackets,
        reason = "the derive builds a tuple struct's value apart from a unit struct's"
    )]
    struct Empty();

    /// Marks a side, storing nothing, in a repr pinned wider than it needs.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u64)]
    struct Pinned {
        /// What it marks.
        kind: PhantomData<Side>,
    }

    /// Variants named as the items the derives name, none of which a variant's name clashes with.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[expect(non_camel_case_types, reason = "named as the derives' items, lowercase ones too")]
    enum Named {
        /// As the repr.
        Repr,
        /// As a conversion's parameter.
        repr,
        /// As the bits a conversion reads.
        bits,
        /// As the layout.
        layout,
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

    /// Whether a book trades, naming atomix by the crate's other name.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(crate = renamed)]
    enum Status {
        /// Trading.
        Open,
        /// Not trading.
        Halted,
    }

    /// Marks a halt, naming atomix by the crate's other name.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(crate = renamed)]
    struct Halt;

    static SIDE: Atomic<Side> = Atomic::new(Side::Bid);
    static SIGN: Atomic<Option<Sign>> = Atomic::new(None);
    static LEVEL: Atomic<Level> = Atomic::new(Level::Low);
    static MARKER: Atomic<Option<Marker>> = Atomic::new(None);
    static TAG: Atomic<Tag<Side>> = Atomic::new(Tag(PhantomData));

    /// Checks that `T` decodes every byte but `holes` as the variant whose repr it is,
    /// and refuses each of `holes`.
    fn decodes_every_byte_but<T: Atom<Repr = u8>>(holes: &[u8]) {
        for repr in u8::MIN..=u8::MAX {
            let decoded = T::from_repr(repr).map(Atom::to_repr);
            let expected = (!holes.contains(&repr)).then_some(repr);
            assert_eq!(decoded, expected, "{repr} decodes as the variant it is, or as none");
        }
    }

    #[test]
    fn a_fieldless_enum_without_a_repr_is_stored_in_a_byte_from_zero() {
        repr_and_validity_are::<Side, u8, ZeroValid>();
        assert_eq!([Side::Bid.to_repr(), Side::Ask.to_repr()], [0, 1], "its discriminants");
        assert_eq!(Side::REPRS, ReprRange::new(0, 1), "enclosed");
        assert_eq!(None::<Side>.to_repr(), 2, "and `None` past them");
        decodes_exactly([Side::Bid, Side::Ask], u8::MIN..=u8::MAX);
    }

    #[test]
    fn a_signed_repr_stores_each_discriminants_own_bits() {
        repr_and_validity_are::<Sign, i8, ZeroValid>();
        assert_eq!(Sign::Minus.to_repr(), -1, "`Minus`, -1, as `0xFF`");
        assert_eq!(Sign::REPRS, ReprRange::from_signed(-1, 1), "through zero");
        assert_eq!(field_width(Sign::REPRS), 2, "two bits as a field, signed");
        assert_eq!(None::<Sign>.to_repr(), -2, "`None` below");
        assert_eq!(None::<Option<Sign>>.to_repr(), -3, "and its `None` below that");
        decodes_exactly([Sign::Minus, Sign::Flat, Sign::Plus], i8::MIN..=i8::MAX);
    }

    #[test]
    fn discriminants_at_both_ends_of_a_byte_wrap_through_zero() {
        repr_and_validity_are::<Edge, u8, ZeroValid>();
        assert_eq!(Edge::REPRS, ReprRange::from_signed(-1, 0), "255 through zero");
        assert_eq!(None::<Edge>.to_repr(), 254, "and `None` below 255");
        decodes_exactly([Edge::Low, Edge::High], u8::MIN..=u8::MAX);
    }

    #[test]
    fn negative_discriminants_without_a_repr_are_sign_extended_in_the_narrowest() {
        repr_and_validity_are::<Tilt, u8, ZeroValid>();
        assert_eq!(Tilt::Left.to_repr(), 0xFF, "-1, in a byte");
        assert_eq!(Tilt::REPRS, ReprRange::from_signed(-1, 1), "through zero");
        assert_eq!(field_width(Tilt::REPRS), 2, "two bits as a field");
        decodes_exactly([Tilt::Left, Tilt::Level, Tilt::Right], u8::MIN..=u8::MAX);
        repr_and_validity_are::<Swing, u16, ZeroNiche>();
        assert_eq!([Swing::Low.to_repr(), Swing::High.to_repr()], [0xFF38, 200], "nine bits");
        assert_eq!(Swing::REPRS, ReprRange::from_signed(-200, 200), "through zero");
        decodes_exactly([Swing::Low, Swing::High], u16::MIN..=u16::MAX);
    }

    #[test]
    fn a_discriminant_past_32_bits_without_a_repr_is_stored_in_a_u64() {
        repr_and_validity_are::<Scale, u64, ZeroNiche>();
        assert_eq!([Scale::Unit.to_repr(), Scale::Tebi.to_repr()], [1, 1 << 40], "exact");
    }

    #[test]
    fn a_discriminant_is_any_constant_expression_naming_the_users_items() {
        repr_and_validity_are::<Level, u16, ZeroNiche>();
        let reprs = [Level::Low.to_repr(), Level::High.to_repr(), Level::Next.to_repr()];
        assert_eq!(reprs, [8, 12, 13], "as rustc evaluates them");
        assert_eq!(Level::REPRS, ReprRange::new(8, 13), "enclosed");
        assert_eq!(None::<Level>.to_repr(), 0, "and `None` takes zero, which none is");
        decodes_exactly([Level::Low, Level::High, Level::Next], u16::MIN..=u16::MAX);
    }

    #[test]
    fn a_c_enum_is_stored_as_cs_int() {
        repr_and_validity_are::<Mode, c_int, ZeroValid>();
        assert_eq!([Mode::Off.to_repr(), Mode::On.to_repr()], [0, 4], "its discriminants");
        assert_eq!(Mode::REPRS, ReprRange::new(0, 4), "enclosed");
    }

    #[test]
    fn a_stated_repr_pins_the_size() {
        repr_and_validity_are::<Phase, u32, ZeroNiche>();
        assert_eq!(Phase::Open.to_repr(), 2, "the discriminant, in a `u32`");
        repr_and_validity_are::<Pinned, u64, ZeroValid>();
    }

    #[test]
    fn an_enum_of_every_byte_is_total() {
        repr_and_validity_are::<Byte, u8, Total>();
        assert_eq!(Byte::REPRS, ReprRange::FULL, "every byte");
        decodes_every_byte_but::<Byte>(&[]);
    }

    #[test]
    fn option_takes_the_one_byte_no_variant_is() {
        repr_and_validity_are::<NonZeroByte, u8, TotalZeroNiche>();
        repr_and_validity_are::<Option<NonZeroByte>, u8, Total>();
        assert_eq!(NonZeroByte::REPRS, ReprRange::NONZERO, "every byte but zero");
        assert_eq!(None::<NonZeroByte>.to_repr(), 0, "`None` zero");
        decodes_every_byte_but::<NonZeroByte>(&[0]);
        repr_and_validity_are::<NonMaxByte, u8, ZeroValid>();
        assert_eq!(NonMaxByte::REPRS, ReprRange::new(0, 254), "every byte but 255");
        assert_eq!(None::<NonMaxByte>.to_repr(), 255, "`None` 255");
        decodes_every_byte_but::<NonMaxByte>(&[255]);
    }

    #[test]
    fn a_zero_width_struct_is_zero_alone() {
        repr_and_validity_are::<Marker, u8, ZeroValid>();
        repr_and_validity_are::<Tag<Side>, u8, ZeroValid>();
        assert_eq!(Marker::REPRS, ReprRange::new(0, 0), "zero alone");
        assert_eq!(Marker.to_repr(), 0, "is its repr");
        assert_eq!(None::<Marker>.to_repr(), 1, "and `None` 1");
        decodes_exactly([Marker], u8::MIN..=u8::MAX);
        decodes_exactly([Tag::<Side>(PhantomData)], u8::MIN..=u8::MAX);
        decodes_exactly([Braced {}], u8::MIN..=u8::MAX);
        decodes_exactly([Empty()], u8::MIN..=u8::MAX);
        assert_eq!(
            Pinned::from_repr(0),
            Some(Pinned { kind: PhantomData }),
            "and in a pinned repr"
        );
    }

    #[test]
    fn variants_named_as_the_derives_items_are_stored_as_any_others() {
        let named = [
            Named::Repr,
            Named::repr,
            Named::bits,
            Named::layout,
            Named::value_0,
            Named::lay_out,
            Named::variant_0,
            Named::placement_0,
            Named::Discriminant,
            Named::discriminant_0,
        ];
        assert_eq!(named.map(Atom::to_repr), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9], "their indices");
        decodes_exactly(named, u8::MIN..=u8::MAX);
    }

    #[test]
    fn a_fieldless_enum_and_a_marker_name_atomix_by_the_path_stated() {
        decodes_exactly([Status::Open, Status::Halted], u8::MIN..=u8::MAX);
        decodes_exactly([Halt], u8::MIN..=u8::MAX);
    }

    /// What a 128-bit repr holds, on aarch64, which every build gives 128-bit atomics.
    #[cfg(target_arch = "aarch64")]
    mod wide {
        use atomix::validity::ZeroNiche;
        use atomix::{Atom, ReprRange};

        use crate::testing::atom::repr_and_validity_are;

        /// Discriminants past 64 bits, which an `i128` holds.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
        #[repr(i128)]
        enum Vast {
            /// Far below zero.
            Low  = -(1 << 100),
            /// Far above.
            High = 1 << 100,
        }

        #[test]
        fn a_128_bit_repr_stores_each_discriminants_own_bits() {
            repr_and_validity_are::<Vast, i128, ZeroNiche>();
            assert_eq!(
                [Vast::Low.to_repr(), Vast::High.to_repr()],
                [-(1 << 100), 1 << 100],
                "exact"
            );
            assert_eq!(Vast::REPRS, ReprRange::from_signed(-(1 << 100), 1 << 100), "through zero");
            assert_eq!(Vast::from_repr(0), None, "and refusing the rest");
        }
    }

    #[test]
    fn statics_hold_derived_values() {
        SIDE.store(Side::Ask, Release);
        assert_eq!(SIDE.load(Acquire), Side::Ask, "a side");
        SIGN.store(Some(Sign::Minus), Release);
        assert_eq!(SIGN.load(Acquire), Some(Sign::Minus), "a sign beside `None`");
        LEVEL.store(Level::Next, Release);
        assert_eq!(LEVEL.load(Acquire), Level::Next, "a level");
        MARKER.store(Some(Marker), Release);
        assert_eq!(MARKER.load(Acquire), Some(Marker), "a marker");
        assert_eq!(TAG.load(Acquire), Tag(PhantomData), "and a tag");
    }
}
