//! Each field operation changes its field alone, and leaves the container's repr one that decodes:
//! on a `bool` at either end and the middle of 8 to 64 bits, on a bitwise field beside one whose
//! zero does not decode, on the add of the field at the repr's top bit, on a field of a field, on a
//! signed last field, whose sign fills the bits above it, and through generic code.

#![feature(const_trait_impl)]
// Loom's cells exist only inside a model, and `as_ptr` and a `const` `new` not at all.
#![cfg(not(loom))]

#[cfg(test)]
mod testing;

#[cfg(test)]
mod tests {
    use core::num::NonZero;
    use std::thread;

    use atomix_core::ordering::{AcqRel, Acquire, Relaxed, Release};
    use atomix_core::{
        Atom, Atomic, AtomicField, BitTest, FetchAdd, Field, FieldPath, MaskBitwise, RangedU8,
        RangedU16, RangedU32, Then, TopField, Whole,
    };

    use crate::testing::field::{bits, canonical, nibble};
    use crate::testing::packed::packed;

    packed! {
        /// A resting quote: 32 bits of quantity, an owner's 8, whose zero does not decode, then a
        /// bit of whether it is live and 8 bits of flags.
        struct Quote in u64, projected as QuoteFields {
            0 => quantity: u32,
            1 => owner: NonZero<u8>,
            2 => live: bool,
            3 => flags: u8,
        }
    }

    packed! {
        /// Flags and a bit, then a count at the top of the word.
        struct Counted in u64, projected as CountedFields {
            0 => flags: u8,
            1 => ready: bool,
            2 => spare: RangedU32<0, 0x7F_FFFF>,
            3 => count: u32,
        }
    }

    packed! {
        /// A packed struct a field of another holds: 9 bits.
        struct Inner in u16, projected as InnerFields {
            0 => ready: bool,
            1 => flags: u8,
        }
    }

    packed! {
        /// A byte, the inner struct at bit 8, then a bit.
        struct Outer in u32, projected as OuterFields {
            0 => low: u8,
            1 => inner: Inner,
            2 => done: bool,
        }
    }

    packed! {
        /// A bit at each end of a `u8`.
        struct Ends8 in u8, projected as Ends8Fields {
            0 => low: bool,
            1 => between: RangedU8<0, 0x3F>,
            2 => top: bool,
        }
    }

    packed! {
        /// A bit at each end of a `u16`, and one at its middle.
        struct Ends16 in u16, projected as Ends16Fields {
            0 => low: bool,
            1 => below: RangedU8<0, 0x7F>,
            2 => middle: bool,
            3 => above: RangedU8<0, 0x3F>,
            4 => top: bool,
        }
    }

    packed! {
        /// A bit at each end of a `u32`, and one at its middle.
        struct Ends32 in u32, projected as Ends32Fields {
            0 => low: bool,
            1 => below: RangedU16<0, 0x7FFF>,
            2 => middle: bool,
            3 => above: RangedU16<0, 0x3FFF>,
            4 => top: bool,
        }
    }

    packed! {
        /// A bit at each end of a `u64`, and one at its middle.
        struct Ends in u64, projected as EndsFields {
            0 => low: bool,
            1 => below: RangedU32<0, 0x7FFF_FFFF>,
            2 => middle: bool,
            3 => above: RangedU32<0, 0x3FFF_FFFF>,
            4 => top: bool,
        }
    }

    nibble!();

    packed! {
        /// A byte, then a signed nibble, in a repr wider than their 12 bits: bits 12 to 31 copy
        /// the nibble's sign.
        struct Signed in u32, projected as SignedFields {
            0 => low: u8,
            1 => nibble: Nibble,
        }
    }

    packed! {
        /// A byte, then a signed nibble at the top: 12 bits, two's complement.
        struct SignedInner in u16, projected as SignedInnerFields {
            0 => low: u8,
            1 => nibble: Nibble,
        }
    }

    packed! {
        /// A byte, then the signed struct at the top: bits 20 to 31 copy the nibble's sign.
        struct SignedOuter in u32, projected as SignedOuterFields {
            0 => low: u8,
            1 => inner: SignedInner,
        }
    }

    /// The owner every quote here has: 7.
    const OWNER: NonZero<u8> = NonZero::new(7).expect("7 is not zero");

    /// A quote, live with flags, and the field its projection names, both built at compile time.
    static QUOTE: Atomic<Quote> =
        Atomic::new(Quote { quantity: 100, owner: OWNER, live: true, flags: 0b10 });
    static LIVE: &AtomicField<Field<Quote, 2, bool>> = QUOTE.fields().live;

    /// The path to the bit `ready` of the inner struct of an `Outer`.
    type InnerReady = Then<Field<Outer, 1, Inner>, Field<Inner, 0, bool>>;

    /// The path to the flags of the inner struct of an `Outer`.
    type InnerFlags = Then<Field<Outer, 1, Inner>, Field<Inner, 1, u8>>;

    #[test]
    fn each_path_names_its_bits() {
        assert_eq!(bits::<Field<Quote, 0, u32>>(), (0, 32), "the quantity first");
        assert_eq!(bits::<Field<Quote, 1, NonZero<u8>>>(), (32, 8), "then the owner");
        assert_eq!(bits::<Field<Quote, 2, bool>>(), (40, 1), "then whether it is live");
        assert_eq!(bits::<Field<Quote, 3, u8>>(), (41, 8), "then the flags");
        assert_eq!(bits::<InnerFlags>(), (9, 8), "a field of a field, at the sum of their offsets");
        assert_eq!(bits::<Whole<Quote>>(), (0, 64), "and the whole value, every bit");
    }

    #[test]
    fn the_projection_lends_each_field_at_its_own_path() {
        let quote = Atomic::new(Quote { quantity: 1, owner: OWNER, live: true, flags: 2 });
        let QuoteFields { quantity, owner, live, flags } = quote.fields();
        let _: &AtomicField<Field<Quote, 0, u32>> = quantity;
        let _: &AtomicField<Field<Quote, 1, NonZero<u8>>> = owner;
        let _: &AtomicField<Field<Quote, 2, bool>> = live;
        let _: &AtomicField<Field<Quote, 3, u8>> = flags;
        assert_eq!(owner.load(Acquire), OWNER, "each the field its path names");
        let counted =
            Atomic::new(Counted { flags: 0, ready: false, spare: RangedU32::MIN, count: 0 });
        let CountedFields { flags, ready, spare, count } = counted.fields();
        assert_eq!((flags.load(Acquire), ready.load(Acquire)), (0, false), "below the spare bits");
        assert_eq!((spare.load(Acquire), count.load(Acquire)), (RangedU32::MIN, 0), "and above");
        let first = Outer { low: 1, inner: Inner { ready: true, flags: 2 }, done: true };
        let outer = Atomic::new(first);
        let OuterFields { low, inner, done } = outer.fields();
        let InnerFields { ready, flags } = inner.fields();
        let _: &AtomicField<InnerReady> = ready;
        let _: &AtomicField<InnerFlags> = flags;
        assert_eq!((low.load(Acquire), done.load(Acquire)), (1, true), "the outer fields");
        assert_eq!((ready.load(Acquire), flags.load(Acquire)), (true, 2), "and the inner ones");
        let signed = Atomic::new(Signed { low: 4, nibble: Nibble(-1) });
        let SignedFields { low, nibble } = signed.fields();
        assert_eq!((low.load(Acquire), nibble.load(Acquire)), (4, Nibble(-1)), "and a signed one");
    }

    #[test]
    fn a_bool_field_sets_clears_toggles_and_stores_alone() {
        let first = Quote { quantity: 7, owner: OWNER, live: false, flags: 0xA5 };
        let quote = Atomic::new(first);
        let live = quote.fields().live;
        live.set(Release);
        assert!(live.load(Acquire), "set");
        assert_eq!(canonical(&quote), Quote { live: true, ..first }, "the others as they were");
        live.clear(Release);
        assert_eq!(canonical(&quote), first, "cleared");
        live.toggle(Release);
        assert_eq!(canonical(&quote), Quote { live: true, ..first }, "toggled on");
        live.store(false, Release);
        assert_eq!(canonical(&quote), first, "stored off");
        live.store(true, Release);
        assert_eq!(canonical(&quote), Quote { live: true, ..first }, "and stored on");
    }

    #[test]
    fn a_bool_field_returns_its_bit_before() {
        let first = Quote { quantity: u32::MAX, owner: OWNER, live: false, flags: 0xFF };
        let quote = Atomic::new(first);
        let live = quote.fields().live;
        assert!(!live.test_and_set(AcqRel), "clear before the set");
        assert!(live.test_and_toggle(AcqRel), "set before the toggle");
        assert!(!live.test_and_clear(AcqRel), "toggled clear before the clear");
        assert_eq!(canonical(&quote), first, "the others as they were");
    }

    /// Checks that each bit of `$atomic`, whose bits are all clear, stores on and off alone.
    macro_rules! each_bit_stores {
        ($atomic:ident: $($bit:ident),+) => {{
            let first = canonical(&$atomic);
            let fields = $atomic.fields();
            $(
                let bit = stringify!($bit);
                fields.$bit.store(true, Release);
                assert!(fields.$bit.load(Acquire), "{bit}: stored on");
                fields.$bit.store(false, Release);
                assert_eq!(canonical(&$atomic), first, "{bit}: and off, the others as they were");
            )+
        }};
    }

    /// Checks that each bit of `$atomic`, whose bits are all clear, returns itself before each
    /// test-and-set, -toggle and -clear, and leaves the others.
    macro_rules! each_bit_returns_itself_before {
        ($atomic:ident: $($bit:ident),+) => {{
            let first = canonical(&$atomic);
            let fields = $atomic.fields();
            $(
                let bit = stringify!($bit);
                assert!(!fields.$bit.test_and_set(AcqRel), "{bit}: clear before the set");
                assert!(fields.$bit.test_and_toggle(AcqRel), "{bit}: set before the toggle");
                assert!(!fields.$bit.test_and_clear(AcqRel), "{bit}: clear before the clear");
            )+
            assert_eq!(canonical(&$atomic), first, "the bits between them as they were");
        }};
    }

    #[test]
    fn each_bit_stores_wherever_it_lies() {
        let ends8 = Atomic::new(Ends8 { low: false, between: RangedU8::MAX, top: false });
        each_bit_stores!(ends8: low, top);
        let (below, above) = (RangedU8::MAX, RangedU8::MAX);
        let ends16 = Atomic::new(Ends16 { low: false, below, middle: false, above, top: false });
        each_bit_stores!(ends16: low, middle, top);
        let (below, above) = (RangedU16::MAX, RangedU16::MAX);
        let ends32 = Atomic::new(Ends32 { low: false, below, middle: false, above, top: false });
        each_bit_stores!(ends32: low, middle, top);
        let (below, above) = (RangedU32::MAX, RangedU32::MAX);
        let ends = Atomic::new(Ends { low: false, below, middle: false, above, top: false });
        each_bit_stores!(ends: low, middle, top);
    }

    // `x86_64`'s `lock bts` takes 16 bits or more, an `asm!` of each width with the field's
    // position its immediate, so each width runs its own instruction.
    #[test]
    fn each_bit_of_16_bits_or_more_returns_itself_before_wherever_it_lies() {
        let (below, above) = (RangedU8::MAX, RangedU8::MAX);
        let ends16 = Atomic::new(Ends16 { low: false, below, middle: false, above, top: false });
        each_bit_returns_itself_before!(ends16: low, middle, top);
        let (below, above) = (RangedU16::MAX, RangedU16::MAX);
        let ends32 = Atomic::new(Ends32 { low: false, below, middle: false, above, top: false });
        each_bit_returns_itself_before!(ends32: low, middle, top);
        let (below, above) = (RangedU32::MAX, RangedU32::MAX);
        let ends = Atomic::new(Ends { low: false, below, middle: false, above, top: false });
        each_bit_returns_itself_before!(ends: low, middle, top);
    }

    #[cfg(aarch64_code)]
    #[test]
    fn each_bit_of_8_bits_returns_itself_before() {
        let ends8 = Atomic::new(Ends8 { low: false, between: RangedU8::MAX, top: false });
        each_bit_returns_itself_before!(ends8: low, top);
    }

    #[test]
    fn a_bitwise_field_combines_its_bits_alone() {
        let first = Quote { quantity: u32::MAX, owner: OWNER, live: true, flags: 0b1010 };
        let quote = Atomic::new(first);
        let flags = quote.fields().flags;
        flags.or(0b0101, Release);
        assert_eq!(flags.load(Acquire), 0b1111, "or");
        flags.and(0b0110, Release);
        assert_eq!(flags.load(Acquire), 0b0110, "and");
        flags.xor(0xFF, Release);
        assert_eq!(flags.load(Acquire), 0b1111_1001, "xor");
        flags.not(Release);
        assert_eq!(canonical(&quote), Quote { flags: 0b0110, ..first }, "and not, alone");
    }

    #[test]
    fn and_keeps_a_neighbour_whose_zero_does_not_decode() {
        let first = Quote { quantity: 0, owner: OWNER, live: true, flags: 0xFF };
        let quote = Atomic::new(first);
        quote.fields().flags.and(0, Release);
        quote.fields().live.clear(Release);
        assert_eq!(canonical(&quote), Quote { live: false, flags: 0, ..first }, "the owner kept");
    }

    #[cfg(aarch64_code)]
    #[test]
    fn each_fetch_form_returns_the_container_before() {
        let first = Quote { quantity: 3, owner: OWNER, live: false, flags: 0b1010 };
        let quote = Atomic::new(first);
        let fields = quote.fields();
        assert_eq!(fields.live.fetch_or(true, AcqRel), first, "the container before the or");
        let live = Quote { live: true, ..first };
        assert_eq!(fields.flags.fetch_and(0b0010, AcqRel), live, "before the and");
        let and = Quote { flags: 0b0010, ..live };
        assert_eq!(fields.flags.fetch_xor(0b0011, AcqRel), and, "before the xor");
        let xor = Quote { flags: 0b0001, ..live };
        assert_eq!(fields.flags.fetch_not(AcqRel), xor, "before the not");
        assert_eq!(canonical(&quote), Quote { flags: 0b1111_1110, ..live }, "each changed alone");
    }

    #[test]
    fn the_field_at_the_top_bit_adds_and_its_carry_leaves_the_word() {
        let spare = RangedU32::MAX;
        let first = Counted { flags: 0xFF, ready: true, spare, count: u32::MAX };
        let counted = Atomic::new(first);
        let count = counted.fields().count;
        assert_eq!(count.fetch_add(1, AcqRel), first, "the container before");
        assert_eq!(canonical(&counted), Counted { count: 0, ..first }, "wrapped, nothing else");
        assert_eq!(count.fetch_sub(1, Relaxed).count, 0, "the count before");
        assert_eq!(canonical(&counted), first, "and back, borrowing nothing from below");
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
        nibble.not(Release);
        assert_eq!(canonical(&signed), Signed { low: 0xFF, nibble: Nibble(-4) }, "inverted");
        nibble.update(AcqRel, Acquire, |_| Nibble(2));
        assert_eq!(canonical(&signed), Signed { low: 0xFF, nibble: Nibble(2) }, "and updated");
    }

    #[test]
    fn a_signed_field_ending_a_signed_last_field_governs_the_bits_above_both() {
        type Nested = Then<Field<SignedOuter, 1, SignedInner>, Field<SignedInner, 1, Nibble>>;
        assert_eq!(bits::<Nested>(), (16, 4), "the nibble of the inner struct, at bit 16");
        assert_eq!(<Nested as FieldPath>::END, 32, "and up to the top");
        let inner = SignedInner { low: 0xFF, nibble: Nibble(3) };
        let outer = Atomic::new(SignedOuter { low: 0xFF, inner });
        let SignedOuterFields { low, inner: nested } = outer.fields();
        let SignedInnerFields { low: inner_low, nibble } = nested.fields();
        nibble.not(Release);
        let inverted = SignedInner { nibble: Nibble(-4), ..inner };
        assert_eq!(
            canonical(&outer),
            SignedOuter { low: 0xFF, inner: inverted },
            "its sign through bit 31"
        );
        assert_eq!(
            (low.load(Acquire), inner_low.load(Acquire)),
            (0xFF, 0xFF),
            "the bytes as they were"
        );
    }

    #[test]
    fn a_field_of_a_field_changes_alone() {
        let first = Outer { low: 3, inner: Inner { ready: true, flags: 1 }, done: true };
        let outer = Atomic::new(first);
        let inner = outer.fields().inner.fields();
        inner.ready.clear(Release);
        inner.flags.or(0x80, Release);
        let changed = Inner { ready: false, flags: 0x81 };
        assert_eq!(canonical(&outer), Outer { inner: changed, ..first }, "both changed, alone");
        assert_eq!(outer.fields().inner.load(Acquire), changed, "and read back whole");
    }

    /// Whether the bit `ready`, turned on, is on before a test-and-set.
    fn set_then_test<P>(ready: &AtomicField<P>) -> bool
    where
        P: FieldPath<Value = bool, Container: Atom<Repr: BitTest>>,
    {
        ready.set(Release);
        ready.test_and_set(AcqRel)
    }

    /// Ors `flags` into `field`.
    fn or_flags<P>(field: &AtomicField<P>, flags: u8)
    where
        P: FieldPath<Value = u8, Container: Atom<Repr: MaskBitwise>>,
    {
        field.or(flags, Release);
    }

    /// Adds one to `count`, at the top of its word: the container before.
    fn add_one<P>(count: &AtomicField<P>) -> P::Container
    where
        P: TopField<Value = u32, Container: Atom<Repr: FetchAdd>>,
    {
        count.fetch_add(1, AcqRel)
    }

    /// Turns on the bit `ready` of an `Inner`, wherever it lies.
    fn set_ready<P>(inner: &AtomicField<P>)
    where
        P: FieldPath<Value = Inner, Container: Atom<Repr: MaskBitwise>>,
    {
        inner.fields().ready.set(Release);
    }

    #[test]
    fn generic_code_changes_a_field_through_the_capabilities_it_states() {
        let first = Counted { flags: 0b100, ready: false, spare: RangedU32::MIN, count: 7 };
        let counted = Atomic::new(first);
        let fields = counted.fields();
        assert!(set_then_test(fields.ready), "set before the test");
        or_flags(fields.flags, 0b11);
        let flagged = Counted { flags: 0b111, ready: true, ..first };
        assert_eq!(add_one(fields.count), flagged, "the container before the add");
        assert_eq!(canonical(&counted), Counted { count: 8, ..flagged }, "each field changed");
        let outer =
            Atomic::new(Outer { low: 3, inner: Inner { ready: false, flags: 1 }, done: true });
        set_ready(outer.fields().inner);
        assert!(outer.fields().inner.fields().ready.load(Acquire), "and a field of a field");
    }

    #[test]
    fn update_replaces_the_field_and_returns_the_container_before() {
        let first = Quote { quantity: 100, owner: OWNER, live: true, flags: 0 };
        let quote = Atomic::new(first);
        let quantity = quote.fields().quantity;
        let tripled = Quote { quantity: 300, ..first };
        assert_eq!(quantity.update(AcqRel, Acquire, |seen| seen * 3), first, "the one before");
        assert_eq!(canonical(&quote), tripled, "tripled, alone");
        let halve = |seen: u32| seen.is_multiple_of(2).then_some(seen / 2);
        assert_eq!(quantity.try_update(AcqRel, Acquire, halve), Ok(tripled), "halved");
        let halved = Quote { quantity: 150, ..first };
        assert_eq!(quantity.try_update(AcqRel, Acquire, |_| None), Err(halved), "declined");
        assert_eq!(canonical(&quote), halved, "with nothing written");
    }

    #[test]
    fn load_rmw_reads_the_field_and_leaves_the_word() {
        let first = Quote { quantity: 9, owner: OWNER, live: true, flags: 0x40 };
        let quote = Atomic::new(first);
        assert_eq!(quote.fields().quantity.load_rmw(Acquire), 9, "the quantity");
        assert_eq!(quote.fields().owner.load_rmw(Acquire), OWNER, "the owner");
        assert_eq!(canonical(&quote), first, "and the word as it was");
    }

    #[cfg(wide)]
    #[test]
    fn a_wide_containers_field_reads_and_updates() {
        packed! {
            /// A value, and the sequence number it was written at.
            struct Versioned in u128, projected as VersionedFields { 0 => value: u64, 1 => seq: u64 }
        }
        let versioned = Atomic::new(Versioned { value: 5, seq: 1 });
        let VersionedFields { value, seq } = versioned.fields();
        assert_eq!(
            seq.update(AcqRel, Acquire, |seq| seq.wrapping_add(1)).seq,
            1,
            "the sequence before"
        );
        assert_eq!(value.load_rmw(Acquire), 5, "the value, by a compare-exchange");
        assert_eq!(seq.load_rmw(Acquire), 2, "and the sequence after");
    }

    #[test]
    fn a_field_lent_by_a_static_changes_that_static_alone() {
        LIVE.clear(Release);
        assert!(!QUOTE.load(Acquire).live, "the field `LIVE` names");
        assert_eq!(QUOTE.load(Acquire).flags, 0b10, "and nothing else");
    }

    #[test]
    fn threads_change_their_own_fields_at_once() {
        let first = Quote { quantity: 0, owner: OWNER, live: false, flags: 0 };
        let quote = Atomic::new(first);
        let fields = quote.fields();
        thread::scope(|scope| {
            scope.spawn(|| fields.live.set(Release));
            scope.spawn(|| fields.flags.or(0x10, Relaxed));
            scope.spawn(|| fields.quantity.update(AcqRel, Relaxed, |seen| seen + 5));
        });
        let last = Quote { quantity: 5, live: true, flags: 0x10, ..first };
        assert_eq!(canonical(&quote), last, "each field changed");
    }

    #[test]
    fn a_field_prints_its_value() {
        let quote = Atomic::new(Quote { quantity: 5, owner: OWNER, live: true, flags: 0 });
        assert_eq!(format!("{:?}", quote.fields().quantity), "5", "the quantity");
        assert_eq!(format!("{:?}", quote.fields().live), "true", "and whether it is live");
    }
}
