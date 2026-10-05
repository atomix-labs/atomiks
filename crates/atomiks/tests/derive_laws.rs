//! The `Atom` laws over derived values, one of each shape and layout, generic instances and
//! `Option`s of them: a value's repr lies in its range, which may wrap, and decodes back to it; a
//! repr that decodes re-encodes to itself, unchecked too; each validity's promise holds; and `None`
//! takes a spare repr: zero where zero is the niche, else one outside its value's range.
//!
//! The repr laws run on every repr of 16 bits or fewer; on a wider one, on the edges of each width,
//! on random bits, and beside each value's repr.

// The derives on the generic `Wrap`, `Pair`, `Lock` and `LockWord` need it.
#![feature(const_trait_impl)]
#![cfg(feature = "derive")]

// atomiks-core's, by its path; its `//!` says why.
#[cfg(test)]
#[path = "../../atomiks-core/tests/testing/mod.rs"]
mod testing;

#[cfg(test)]
mod tests {
    use core::any::type_name;
    use core::fmt::Debug;
    use core::num::NonZero;

    use atomiks::validity::{Total, TotalZeroNiche, ZeroNiche, ZeroValid};
    use atomiks::{Atom, ExactBits, Primitive, RangedI8, RangedU64, ReprRange};
    use proptest::prelude::{Just, Strategy, any, prop_oneof};
    use proptest::sample::select;
    use proptest::test_runner::TestCaseError;
    use proptest::{option, proptest};

    use crate::testing::atom::{repr_and_validity_are, with_every_byte};
    use crate::testing::law::{
        Promise, assert_holds, decodes_as_promised, edge_or_random_bits, none_laws, round_trips,
    };

    /// The side of the book an order rests on: one bit, from zero.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Side {
        /// A buy.
        Bid,
        /// A sell.
        Ask,
    }

    /// Which way a price moved: two bits, signed, wrapping through zero.
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

    /// Two discriminants far apart either side of zero, with no `#[repr]`: in a `u16`, zero none.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Swing {
        /// Far down.
        Low  = -200,
        /// Far up.
        High = 200,
    }

    /// Two discriminants past 32 bits apart, zero none.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[repr(u64)]
    enum Scale {
        /// One.
        Unit = 1,
        /// A tebi.
        Tebi = 1 << 40,
    }

    /// Declares `NonZeroByte`, a variant for every byte but zero, each documented with its name.
    macro_rules! non_zero_byte {
        ($zero:ident $one:ident $($rest:ident)*) => {
            /// A variant for every byte but zero, which `Option`'s `None` takes.
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
            enum NonZeroByte {
                #[doc = stringify!($one)]
                $one = 1,
                $(#[doc = stringify!($rest)] $rest,)*
            }
        };
    }

    with_every_byte!(non_zero_byte);

    /// A third of a turn: its range, from 0x60 round through zero, wraps both through zero and
    /// through the signed bytes' ends.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Third {
        /// None.
        Zero = 0,
        /// One third.
        One  = 0x60,
        /// Two thirds.
        Two  = 0xC0,
    }

    /// Stores nothing: its one value is zero.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Marker;

    /// An id never zero, as its field.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Id(NonZero<u16>);

    /// A sequence number, every repr of which decodes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Seq(u64);

    /// A lock's owner, from 3, as its field's range.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct OwnerId(RangedU64<3>);

    /// A flag, a mark of no bits, a side and a sign by position: four bits, which extend the sign.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Flags(bool, Marker, Side, Sign);

    /// A byte of length below two bits of sign, its top field: ten bits, which extend the sign.
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

    /// A step in the signed repr it states.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = i16)]
    struct SignedStep {
        /// How far.
        length: u8,
        /// Which way.
        sign: Sign,
    }

    /// A resting quote: 32 bits of quantity, then a bit of side, then one of whether it is live.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Quote {
        /// How many.
        quantity: u32,
        /// Which side.
        side: Side,
        /// Whether it may fill.
        live: bool,
    }

    /// A turn by thirds: its top field, a third, wraps both ways, so its range is every repr up to
    /// the third's end.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Turn {
        /// Which way.
        clockwise: bool,
        /// How far.
        angle: Third,
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
    struct Entry {
        /// What the entry holds.
        quote: Option<Quote>,
        /// When it was written.
        seq: u16,
    }

    /// A ring buffer's slot: tagged, two bits above the 32 of a lap.
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

    /// A reading of a sign, missing or stale: the units take the reprs beside the sign's range.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Reading {
        /// No reading yet.
        Missing,
        /// A reading too old to use.
        Stale,
        /// The sign read.
        Present(Sign),
    }

    /// An order pending or filled: `Pending` takes the repr beside the sign, above a clear byte.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Fill {
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

    /// A quote on a side, or closed: `Closed` takes the repr above the side's range.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Offer {
        /// Not quoting.
        Closed,
        /// Quoting on a side.
        Open(Side),
    }

    /// Seven flags on either side: tagged, and every pattern of a byte decodes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Septet {
        /// On the left.
        Left(bool, bool, bool, bool, bool, bool, bool),
        /// On the right.
        Right(bool, bool, bool, bool, bool, bool, bool),
    }

    /// A gate held by an owner: `Closed` takes zero, which no owner's id is, so every repr decodes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Gate {
        /// No owner.
        Closed,
        /// Held by its owner.
        Open(NonZero<u64>),
    }

    /// A shift back or ahead by a byte: tagged by its stated discriminants, negative too.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[repr(i8)]
    enum Shift {
        /// Back by a byte.
        Back(u8) = -1,
        /// Still.
        Still,
        /// Ahead by a byte.
        Ahead(u8) = 3,
    }

    /// A signal whose variant at tag zero holds an id never zero, so zero never decodes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[repr(u8)]
    enum Signal {
        /// Raised by an id.
        Raised(NonZero<u8>) = 0,
        /// Lowered.
        Lowered = 1,
    }

    /// A value of any atom.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Wrap<T>(T);

    /// Two values of any atoms, packed in a `u64`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u64)]
    struct Pair<A, B> {
        /// The low one.
        first: A,
        /// The one above it.
        second: B,
    }

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

    /// A lock word of stated discriminants, tagged by them, so zero is `Unbuilt` whatever owns it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u16)]
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

    /// Each side.
    fn sides() -> impl Strategy<Value = Side> {
        select(&[Side::Bid, Side::Ask])
    }

    /// Each sign.
    fn signs() -> impl Strategy<Value = Sign> {
        select(&[Sign::Minus, Sign::Flat, Sign::Plus])
    }

    /// Each swing.
    fn swings() -> impl Strategy<Value = Swing> {
        select(&[Swing::Low, Swing::High])
    }

    /// Steps of every length and sign.
    fn steps() -> impl Strategy<Value = Step> {
        (any::<u8>(), signs()).prop_map(|(length, sign)| Step { length, sign })
    }

    /// Quotes of every quantity, side and liveness.
    fn quotes() -> impl Strategy<Value = Quote> {
        (any::<u32>(), sides(), any::<bool>()).prop_map(|(quantity, side, live)| Quote {
            quantity,
            side,
            live,
        })
    }

    /// Orders of every quote and id.
    fn orders() -> impl Strategy<Value = Order> {
        (quotes(), any::<NonZero<u16>>()).prop_map(|(quote, id)| Order { quote, id })
    }

    /// Each kind of slot, in every lap.
    fn slots() -> impl Strategy<Value = Slot> {
        prop_oneof![
            Just(Slot::Empty),
            any::<u32>().prop_map(|lap| Slot::Writing { lap }),
            any::<u32>().prop_map(|lap| Slot::Ready { lap }),
        ]
    }

    /// Each reading.
    fn readings() -> impl Strategy<Value = Reading> {
        prop_oneof![
            Just(Reading::Missing),
            Just(Reading::Stale),
            signs().prop_map(Reading::Present)
        ]
    }

    /// Owners of every id.
    fn owner_ids() -> impl Strategy<Value = OwnerId> {
        (3..=u64::MAX).prop_map(|id| OwnerId(RangedU64::new(id).expect("the id is from 3 up")))
    }

    /// Pairs of every value of `first` and `second`.
    fn pairs<A: Strategy, B: Strategy>(
        first: A, second: B,
    ) -> impl Strategy<Value = Pair<A::Value, B::Value>> {
        (first, second).prop_map(|(first, second)| Pair { first, second })
    }

    /// Each kind of lock, with every owner of `owners`.
    fn locks<O: Strategy<Value: Clone>>(owners: O) -> impl Strategy<Value = Lock<O::Value>> {
        prop_oneof![Just(Lock::Uninit), Just(Lock::Free), owners.prop_map(Lock::Owned)]
    }

    /// Each kind of lock word, with every owner of `owners`.
    fn lock_words<O: Strategy<Value: Clone>>(
        owners: O,
    ) -> impl Strategy<Value = LockWord<O::Value>> {
        prop_oneof![
            Just(LockWord::Unbuilt),
            Just(LockWord::Free),
            owners.prop_map(LockWord::Owned),
            Just(LockWord::Poisoned),
        ]
    }

    /// The laws for `repr`: what `T`'s validity promises of it, the repr laws, and, where it
    /// decodes, the round trip of the value it decodes to.
    fn repr_obeys_the_laws<T>(repr: T::Repr) -> Result<(), TestCaseError>
    where
        T: Atom + PartialEq + Debug,
        T::Validity: Promise,
        T::Repr: ExactBits + PartialEq + Debug,
    {
        decodes_as_promised::<_, T>(repr)?;
        T::from_repr(repr).map_or(Ok(()), round_trips)
    }

    /// The laws for `value`, and for its repr's neighbours and the low bits of `bits` as reprs.
    fn value_obeys_the_laws<T>(value: T, bits: u128) -> Result<(), TestCaseError>
    where
        T: Atom + PartialEq + Debug,
        T::Validity: Promise,
        T::Repr: ExactBits + PartialEq + Debug,
    {
        round_trips(value)?;
        let repr = value.to_repr().to_bits();
        [repr.wrapping_sub(1), repr.wrapping_add(1), bits]
            .into_iter()
            .try_for_each(|bits| repr_obeys_the_laws::<T>(Primitive::from_bits(bits)))
    }

    /// Checks the laws on every repr of `T`, of 16 bits or fewer.
    fn every_repr_obeys_the_laws<T>()
    where
        T: Atom + PartialEq + Debug,
        T::Validity: Promise,
        T::Repr: ExactBits + PartialEq + Debug,
    {
        let width = <T::Repr as Primitive>::BITS;
        assert!(width <= 16, "`{}`'s {width} bits are too many to check each", type_name::<T>());
        for bits in 0..=u128::MAX.unbounded_shr(128_u32.wrapping_sub(width)) {
            assert_holds(repr_obeys_the_laws::<T>(Primitive::from_bits(bits)));
        }
    }

    #[test]
    fn each_validity_is_the_one_its_layout_promises() {
        repr_and_validity_are::<NonZeroByte, u8, TotalZeroNiche>();
        repr_and_validity_are::<Septet, u8, Total>();
        repr_and_validity_are::<LockWord<Sign>, u16, ZeroValid>();
        repr_and_validity_are::<LockWord<NonZero<u8>>, u16, ZeroValid>();
        repr_and_validity_are::<Wrap<Swing>, u16, ZeroNiche>();
        assert_eq!(Turn::REPRS, ReprRange::new(0, 0x1FF), "every repr up to the third's end");
        assert_eq!(Offer::Closed.to_repr(), 2, "and `Closed` above the side's 0 and 1");
    }

    #[test]
    fn every_repr_of_16_bits_or_fewer_obeys_the_laws() {
        every_repr_obeys_the_laws::<Side>();
        every_repr_obeys_the_laws::<Sign>();
        every_repr_obeys_the_laws::<Swing>();
        every_repr_obeys_the_laws::<NonZeroByte>();
        every_repr_obeys_the_laws::<Third>();
        every_repr_obeys_the_laws::<Turn>();
        every_repr_obeys_the_laws::<Offer>();
        every_repr_obeys_the_laws::<Septet>();
        every_repr_obeys_the_laws::<LockWord<Sign>>();
        every_repr_obeys_the_laws::<LockWord<u8>>();
        every_repr_obeys_the_laws::<LockWord<NonZero<u8>>>();
        every_repr_obeys_the_laws::<LockWord<Swing>>();
        every_repr_obeys_the_laws::<Option<NonZeroByte>>();
        every_repr_obeys_the_laws::<Marker>();
        every_repr_obeys_the_laws::<Id>();
        every_repr_obeys_the_laws::<Flags>();
        every_repr_obeys_the_laws::<Step>();
        every_repr_obeys_the_laws::<PriceMove>();
        every_repr_obeys_the_laws::<Option<PriceMove>>();
        every_repr_obeys_the_laws::<SignedStep>();
        every_repr_obeys_the_laws::<Reading>();
        every_repr_obeys_the_laws::<Fill>();
        every_repr_obeys_the_laws::<Shift>();
        every_repr_obeys_the_laws::<Signal>();
        every_repr_obeys_the_laws::<Wrap<Sign>>();
        every_repr_obeys_the_laws::<Wrap<Swing>>();
        every_repr_obeys_the_laws::<Option<Option<Sign>>>();
        every_repr_obeys_the_laws::<Option<Swing>>();
        every_repr_obeys_the_laws::<Option<Step>>();
        every_repr_obeys_the_laws::<Option<Option<Reading>>>();
    }

    #[test]
    fn none_takes_a_spare_repr_of_each_derived_value() {
        none_laws!(Side, Sign, Swing, Scale, Marker, Id, Flags, Step, SignedStep, Quote, Order);
        none_laws!(OwnerId, PriceMove, Lock<OwnerId>);
        none_laws!(NonZeroByte, Third, Turn, Entry, Slot, Reading, Fill, Offer, Shift, Signal);
        none_laws!(Wrap<Sign>, Pair<u32, bool>, Pair<NonZero<u8>, Sign>, Lock<u8>, Lock<Sign>);
        none_laws!(LockWord<Sign>, LockWord<NonZero<u8>>);
        none_laws!(Wrap<Swing>, Lock<Swing>, LockWord<Swing>);
        none_laws!(Option<Sign>, Option<Step>, Option<Reading>, Option<Order>, Option<Slot>);
    }

    proptest! {
        #[test]
        fn fieldless_enums_obey_the_laws(
            side in sides(),
            sign in signs(),
            swing in swings(),
            scale in select(&[Scale::Unit, Scale::Tebi]),
            bits in edge_or_random_bits(&[]),
        ) {
            value_obeys_the_laws(side, bits)?;
            value_obeys_the_laws(sign, bits)?;
            value_obeys_the_laws(swing, bits)?;
            value_obeys_the_laws(scale, bits)?;
        }

        #[test]
        fn newtypes_and_zero_width_structs_obey_the_laws(
            id in any::<NonZero<u16>>().prop_map(Id),
            seq in any::<u64>().prop_map(Seq),
            owner in owner_ids(),
            bits in edge_or_random_bits(&[]),
        ) {
            value_obeys_the_laws(id, bits)?;
            value_obeys_the_laws(seq, bits)?;
            value_obeys_the_laws(owner, bits)?;
            value_obeys_the_laws(Marker, bits)?;
        }

        #[test]
        fn packed_structs_obey_the_laws(
            flags in (any::<bool>(), sides(), signs())
                .prop_map(|(live, side, sign)| Flags(live, Marker, side, sign)),
            step in steps(),
            signed in steps().prop_map(|Step { length, sign }| SignedStep { length, sign }),
            quote in quotes(),
            halves in any::<(u16, u16)>().prop_map(|(low, high)| Halves { low, high }),
            order in orders(),
            entry in (option::of(quotes()), any::<u16>())
                .prop_map(|(quote, seq)| Entry { quote, seq }),
            bits in edge_or_random_bits(&[]),
        ) {
            value_obeys_the_laws(flags, bits)?;
            value_obeys_the_laws(step, bits)?;
            value_obeys_the_laws(signed, bits)?;
            value_obeys_the_laws(quote, bits)?;
            value_obeys_the_laws(halves, bits)?;
            value_obeys_the_laws(order, bits)?;
            value_obeys_the_laws(entry, bits)?;
        }

        #[test]
        fn enums_with_fields_obey_the_laws(
            slot in slots(),
            reading in readings(),
            fill in prop_oneof![
                Just(Fill::Pending),
                (any::<u8>(), signs()).prop_map(|(quantity, sign)| Fill::Filled { quantity, sign }),
            ],
            gate in prop_oneof![Just(Gate::Closed), any::<NonZero<u64>>().prop_map(Gate::Open)],
            shift in prop_oneof![
                any::<u8>().prop_map(Shift::Back),
                Just(Shift::Still),
                any::<u8>().prop_map(Shift::Ahead),
            ],
            signal in prop_oneof![
                any::<NonZero<u8>>().prop_map(Signal::Raised),
                Just(Signal::Lowered),
            ],
            bits in edge_or_random_bits(&[]),
        ) {
            value_obeys_the_laws(slot, bits)?;
            value_obeys_the_laws(reading, bits)?;
            value_obeys_the_laws(fill, bits)?;
            value_obeys_the_laws(gate, bits)?;
            value_obeys_the_laws(shift, bits)?;
            value_obeys_the_laws(signal, bits)?;
        }

        #[test]
        fn generic_instances_obey_the_laws(
            wrap in signs().prop_map(Wrap),
            pair in pairs(any::<u32>(), any::<bool>()),
            partial in pairs(any::<NonZero<u8>>(), signs()),
            tagged in locks(any::<u8>()),
            niche in locks(signs()),
            ranged in locks(owner_ids()),
            zero_niche in locks(swings()),
            stated in lock_words(any::<u8>()),
            bits in edge_or_random_bits(&[]),
        ) {
            value_obeys_the_laws(wrap, bits)?;
            value_obeys_the_laws(pair, bits)?;
            value_obeys_the_laws(partial, bits)?;
            value_obeys_the_laws(tagged, bits)?;
            value_obeys_the_laws(niche, bits)?;
            value_obeys_the_laws(ranged, bits)?;
            value_obeys_the_laws(zero_niche, bits)?;
            value_obeys_the_laws(stated, bits)?;
        }

        #[test]
        fn options_obey_the_laws(
            sign in option::of(option::of(signs())),
            step in option::of(steps()),
            reading in option::of(option::of(readings())),
            order in option::of(orders()),
            slot in option::of(slots()),
            lock in option::of(locks(any::<u8>())),
            bits in edge_or_random_bits(&[]),
        ) {
            value_obeys_the_laws(sign, bits)?;
            value_obeys_the_laws(step, bits)?;
            value_obeys_the_laws(reading, bits)?;
            value_obeys_the_laws(order, bits)?;
            value_obeys_the_laws(slot, bits)?;
            value_obeys_the_laws(lock, bits)?;
        }
    }
}
