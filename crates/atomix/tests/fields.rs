//! What the derive adds to the field operations atomix-core's tests cover: each field's path, at
//! the placement the derive packs it at, on a named, a tuple, a generic and a nested struct, and a
//! signed last field, whose sign fills the bits above it; the projection, built at compile time;
//! the top field it marks; and the projection's `Debug`.

#![feature(const_trait_impl)]
// Loom's cells exist only inside a model, and `as_ptr` and a `const` `new` not at all; `model.rs`
// holds the loom tests.
#![cfg(not(loom))]

// atomix-core's, by its path; its `//!` says why.
#[cfg(test)]
#[path = "../../atomix-core/tests/testing/mod.rs"]
mod testing;

#[cfg(feature = "derive")]
#[cfg(test)]
mod tests {
    use core::marker::PhantomData;
    use core::num::NonZero;

    use atomix::ordering::{AcqRel, Acquire, Relaxed, Release};
    use atomix::{Atom, AtomBitwise, Atomic, AtomicField, Field, FieldPath, RangedU32, Then};

    use crate::testing::field::{bits, canonical, nibble};

    /// The side of the book an order rests on: one bit, and no bitwise operations.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Side {
        Bid,
        Ask,
    }

    /// Flags of an order, each pattern of a byte a value.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom, AtomBitwise)]
    struct Flags(u8);

    /// An owner's id, never zero.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct OwnerId(NonZero<u8>);

    /// A resting quote: 32 bits of quantity, a bit of side, a bit of whether it is live, then 8
    /// bits of flags and the owner's 8.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Quote {
        /// How many.
        quantity: u32,
        side: Side,
        live: bool,
        flags: Flags,
        owner: OwnerId,
    }

    /// Flags and a bit, then a count in the top half of the word.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Counted {
        flags: Flags,
        ready: bool,
        spare: RangedU32<0, 0x7F_FFFF>,
        count: u32,
    }

    /// A packed struct a field of another holds.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Inner {
        ready: bool,
        flags: Flags,
    }

    /// A byte, the inner struct at bit 8, then a bit.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Outer {
        low: u8,
        inner: Inner,
        done: bool,
    }

    /// A word of any value and a mark: each instance lays out its own fields.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u64)]
    struct Tagged<T> {
        marked: bool,
        value: T,
    }

    /// A word of any value, by default a `u16`, and a mark, whose parameters take the names the
    /// projection's own would.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u64)]
    struct Named<'a, P = u16> {
        marked: bool,
        value: P,
        origin: PhantomData<&'a ()>,
    }

    /// A tuple struct: a count, and whether it is final.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Pair(u16, bool);

    nibble!();

    /// A byte, then a signed nibble, in a repr wider than their 12 bits: bits 12 to 31 copy the
    /// nibble's sign.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u32)]
    struct Signed {
        low: u8,
        nibble: Nibble,
    }

    /// The owner every quote here has: 7.
    const OWNER: OwnerId = OwnerId(NonZero::new(7).expect("7 is not zero"));

    /// A live quote, and its field `live`, both built at compile time.
    static QUOTE: Atomic<Quote> = Atomic::new(Quote {
        quantity: 100,
        side: Side::Bid,
        live: true,
        flags: Flags(0b10),
        owner: OWNER,
    });
    static LIVE: &AtomicField<Field<Quote, 2, bool>> = QUOTE.fields().live;

    /// The path to the bit `ready` of the inner struct of an `Outer`.
    type InnerReady = Then<Field<Outer, 1, Inner>, Field<Inner, 0, bool>>;

    /// The path to the flags of the inner struct of an `Outer`.
    type InnerFlags = Then<Field<Outer, 1, Inner>, Field<Inner, 1, Flags>>;

    #[test]
    fn each_path_names_the_bits_the_derive_packs_its_field_in() {
        assert_eq!(bits::<Field<Quote, 0, u32>>(), (0, 32), "the quantity first");
        assert_eq!(bits::<Field<Quote, 1, Side>>(), (32, 1), "then the side");
        assert_eq!(bits::<Field<Quote, 2, bool>>(), (33, 1), "then whether it is live");
        assert_eq!(bits::<Field<Quote, 3, Flags>>(), (34, 8), "then the flags");
        assert_eq!(bits::<Field<Tagged<u8>, 1, u8>>(), (1, 8), "an instance's own layout");
        assert_eq!(bits::<Field<Tagged<u32>, 1, u32>>(), (1, 32), "and another's");
        assert_eq!(bits::<InnerFlags>(), (9, 8), "a field of a field, at the sum of their offsets");
    }

    #[test]
    fn the_projection_lends_each_field_at_its_own_path() {
        let quote = Atomic::new(QUOTE.load(Acquire));
        let QuoteFields { quantity, side, live, flags, owner } = quote.fields();
        let _: &AtomicField<Field<Quote, 0, u32>> = quantity;
        let _: &AtomicField<Field<Quote, 1, Side>> = side;
        let _: &AtomicField<Field<Quote, 2, bool>> = live;
        let _: &AtomicField<Field<Quote, 3, Flags>> = flags;
        assert_eq!(owner.load(Acquire), OWNER, "each the field its path names");
        let first = Outer { low: 3, inner: Inner { ready: true, flags: Flags(1) }, done: true };
        let outer = Atomic::new(first);
        let InnerFields { ready, flags } = outer.fields().inner.fields();
        let _: (&AtomicField<InnerReady>, &AtomicField<InnerFlags>) = (ready, flags);
        ready.clear(Release);
        flags.or(Flags(0x80), Release);
        let changed = Inner { ready: false, flags: Flags(0x81) };
        assert_eq!(canonical(&outer), Outer { inner: changed, ..first }, "a field of a field");
    }

    #[test]
    fn a_generic_field_lies_where_its_instance_puts_it() {
        let tagged = Atomic::new(Tagged { marked: false, value: 0xABCD_u16 });
        let fields = tagged.fields();
        assert!(!fields.marked.test_and_set(AcqRel), "unmarked before");
        assert_eq!(fields.value.update(AcqRel, Acquire, |value| value + 1).value, 0xABCD, "before");
        assert_eq!(canonical(&tagged), Tagged { marked: true, value: 0xABCE }, "both changed");
    }

    #[test]
    fn a_structs_own_parameters_named_as_the_projections_would_be_are_its_own() {
        let named: Atomic<Named<'_>> =
            Atomic::new(Named { marked: false, value: 7, origin: PhantomData });
        named.fields().marked.set(Release);
        named.fields().value.update(AcqRel, Acquire, |value| value * 3);
        let last = Named { marked: true, value: 21, origin: PhantomData };
        assert_eq!(canonical(&named), last, "each field, in a `u16` by default");
    }

    #[test]
    fn a_tuple_structs_projection_is_a_tuple_struct() {
        let pair = Atomic::new(Pair(5, false));
        pair.fields().1.toggle(Release);
        pair.fields().0.update(AcqRel, Acquire, |count| count * 2);
        assert_eq!(canonical(&pair), Pair(10, true), "each field by its index");
    }

    #[test]
    fn the_last_field_ending_at_the_top_bit_adds_in_place() {
        let first = Counted { flags: Flags(0xFF), ready: true, spare: RangedU32::MAX, count: 1 };
        let counted = Atomic::new(first);
        assert_eq!(counted.fields().count.fetch_add(u32::MAX, AcqRel), first, "before");
        assert_eq!(canonical(&counted), Counted { count: 0, ..first }, "wrapped, nothing else");
    }

    #[test]
    fn a_signed_last_field_extends_its_sign_through_the_bits_above() {
        let end = <Field<Signed, 1, Nibble> as FieldPath>::END;
        assert_eq!(end, 32, "it governs the bits above it");
        let signed = Atomic::new(Signed { low: 0xFF, nibble: Nibble(3) });
        let nibble = signed.fields().nibble;
        nibble.xor(Nibble(-8), Release);
        assert_eq!(canonical(&signed), Signed { low: 0xFF, nibble: Nibble(-5) }, "sign set");
        nibble.and(Nibble(7), Release);
        assert_eq!(canonical(&signed), Signed { low: 0xFF, nibble: Nibble(3) }, "and cleared");
    }

    #[test]
    fn a_field_lent_by_a_static_changes_that_static_alone() {
        LIVE.clear(Relaxed);
        assert!(!QUOTE.load(Acquire).live, "the field `LIVE` names");
        assert_eq!(QUOTE.load(Acquire).flags, Flags(0b10), "and nothing else");
    }

    #[test]
    fn the_projection_prints_each_fields_value() {
        let quote =
            Quote { quantity: 5, side: Side::Ask, live: false, flags: Flags(1), owner: OWNER };
        let shown = format!("{:?}", Atomic::new(quote).fields());
        let expected = "QuoteFields { quantity: 5, side: Ask, live: false, flags: Flags(1), owner: OwnerId(7) }";
        assert_eq!(shown, expected, "each field's value");
        assert_eq!(
            format!("{:?}", Atomic::new(Pair(2, true)).fields()),
            "PairFields(2, true)",
            "by position"
        );
    }
}
