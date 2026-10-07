//! A pointer word, written out as the derive writes one, keeps its tags in its pointer's low bits
//! and the pointer's provenance through every encode, decode, exchange and tag operation, as Miri
//! checks under `-Zmiri-strict-provenance` by reading each node through the pointer decoded; its
//! range and validity are what its fields promise, its `Option` costs nothing, each tag changes
//! alone through its place, at bit 0, 1 and 2, a signed tag at the top of the tags reaches no bit
//! of the pointer, a word whose pointer is a word keeps its tags above the inner word's, and a
//! list's node holds an atomic word of a pointer to its own type. A pointer enum of a unit, a value
//! and a node, written out alike over `PointerEnumLayout`, holds each in one word.

#![feature(const_trait_impl)]
// Loom's cells exist only inside a model, and `as_ptr` and a `const` `new` not at all.
#![cfg(not(loom))]

#[cfg(test)]
mod testing;

#[cfg(test)]
#[expect(
    unsafe_code,
    reason = "frees the nodes the tests box, and reads them through each pointer"
)]
mod tests {
    use core::mem::size_of;
    use core::ptr::{self, NonNull};
    use std::thread;

    use atomiks_core::__private::{
        PackedField, PackedLayout, PackedValidity, PointeeAlignment, PointerEnumLayout,
        PointerEnumValidity, PointerEnumVariant, SelectValidity, Tags, ValidityCode,
        assert_aligned, from_bits, from_pointer, to_bits, to_tagged_pointer, to_tagged_repr,
    };
    use atomiks_core::ordering::{AcqRel, Acquire, Relaxed, Release};
    use atomiks_core::validity::{Partial, Total, ZeroNiche, ZeroValid};
    use atomiks_core::{
        Atom, Atomic, AtomicField, Field, FieldPath, PtrAtom, RangedU8, ReprRange, Then,
    };

    use crate::testing::atom::{low_bit_widths, repr_and_validity_are};
    use crate::testing::field::{bits, canonical, nibble};
    use crate::testing::pointer_word::pointer_word;

    /// A node, aligned to 8, so a pointer to one leaves three low bits clear.
    #[derive(Debug)]
    #[repr(align(8))]
    struct Node {
        /// What it holds.
        value: u64,
    }

    /// A block, aligned to 32, so a pointer to one leaves five low bits clear.
    #[derive(Debug)]
    #[repr(align(32))]
    struct Block {
        /// What it holds.
        value: u64,
    }

    pointer_word! {
        /// A Treiber stack's head: the top node or none, a version counter and a mark, in 3 bits.
        struct Head, projected as HeadFields {
            0 => top: Option<NonNull<Node>>,
            1 => version: RangedU8<0, 3>,
            2 => marked: bool,
        }
    }

    pointer_word! {
        /// A Harris list's link, which may be null, and whether its node is deleted.
        struct NullableLink, projected as NullableLinkFields {
            0 => next: *mut Node,
            1 => deleted: bool,
        }
    }

    pointer_word! {
        /// A link that is never null, so `Option<Link>`'s `None` takes the null repr.
        struct Link, projected as LinkFields {
            0 => next: NonNull<Node>,
            1 => deleted: bool,
        }
    }

    pointer_word! {
        /// A tag in each of a node's three low bits.
        struct Bits, projected as BitsFields {
            0 => node: NonNull<Node>,
            1 => low: bool,
            2 => middle: bool,
            3 => high: bool,
        }
    }

    nibble!();

    pointer_word! {
        /// A flag, then a signed nibble at the top of the tags, below the pointer.
        struct Signed, projected as SignedFields {
            0 => block: NonNull<Block>,
            1 => flag: bool,
            2 => nibble: Nibble,
        }
    }

    pointer_word! {
        /// A node and its mark, at bit 0.
        struct Inner, projected as InnerFields {
            0 => node: NonNull<Node>,
            1 => marked: bool,
        }
    }

    pointer_word! {
        /// A marked node and a lock, at bit 1, above the inner word's mark.
        struct Outer, projected as OuterFields {
            0 => inner: Inner,
            1 => locked: bool,
        }
    }

    /// A Harris list's node, which holds the link to the next node: an atomic of a word whose
    /// pointer points to a node.
    struct ListNode {
        /// Its key.
        key: u64,
        /// The next node, and whether this one is deleted.
        next: Atomic<ListLink>,
    }

    pointer_word! {
        /// The link a list's node holds: the next node or none, and whether the node that holds the
        /// link is deleted.
        struct ListLink, projected as ListLinkFields {
            0 => next: Option<NonNull<ListNode>>,
            1 => deleted: bool,
        }
    }

    /// A slot of a table: empty, a value held inline, or a node, as the derive writes an enum of
    /// them: two tag bits, the value above the three a node's alignment leaves clear.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Slot {
        /// Tag 0, null.
        Empty,
        /// Tag 1.
        Inline(u32),
        /// Tag 2.
        Node(NonNull<Node>),
    }

    /// Where `Slot::Inline` keeps its value.
    const INLINE: PackedField = PackedField::new(<u32 as Atom>::REPRS, 0);
    /// `Slot::Inline`'s fields.
    const INLINE_FIELDS: PackedLayout = PackedLayout::new(&[INLINE]);
    /// `Slot::Node`'s tag fields: none.
    const NO_TAGS: PackedLayout = PackedLayout::new(&[]);
    /// How a slot lays out its variants.
    const SLOT: PointerEnumLayout = PointerEnumLayout::niche_or_tagged(&[
        PointerEnumVariant::unit(0_u8),
        PointerEnumVariant::data(1_u8),
        PointerEnumVariant::pointer::<_, NonNull<Node>>(2_u8, NO_TAGS),
    ]);
    /// What a slot promises of its zero repr: `Empty`'s.
    const SLOT_ZERO: PointerEnumValidity = PointerEnumValidity::new(SLOT)
        .with_variant(0_u8, PackedValidity::EMPTY)
        .with_variant(1_u8, PackedValidity::EMPTY.with_field::<Total>(INLINE.layout()))
        .with_variant(2_u8, PackedValidity::EMPTY.with_field::<ZeroNiche>(SLOT.pointer_layout()));

    /// What a slot's node leaves clear, which `Slot::Inline` keeps its value above: read by code
    /// alone, never by the slot's layout.
    const SLOT_ALIGNMENT: PointeeAlignment =
        PointeeAlignment::least(&[<NonNull<Node> as Atom>::POINTEE_ALIGNMENT]);

    const _: () = SLOT.assert_tags_fit::<Slot, NonNull<Node>>("`Slot::Node`");
    const _: () = SLOT.assert_data_fits::<Slot>(INLINE_FIELDS, SLOT_ALIGNMENT, "`Slot::Inline`");

    // SAFETY: each variant's repr is its tag, in the two low bits, beside the node's pointer,
    // which sets the tag, and those of any word that holds the slot, as `Tags::set_in` does, so it
    // keeps its provenance; or beside the value's bits above the three clear ones, an address
    // without provenance; or alone; a word's tags set in either of those as `set_in` sets them;
    // `to_repr` is the repr without tags beside, refusing a node's pointer that had a tag bit set;
    // `from_repr` reads the tag back and decodes the variant it names, refusing a unit's or a
    // value's repr with any other bit set, so each value has one repr, by the repr alone; the
    // default `from_repr_unchecked` unwraps it; the validity and range are `Empty`'s, which zero
    // is; and a node's pointer may cross threads, as `AtomicPtr`'s does.
    #[expect(unsafe_code, reason = "the impl the derive writes for a pointer enum, by hand")]
    const unsafe impl Atom for Slot {
        type Repr = *mut ();
        type Validity = <ValidityCode<{ SLOT_ZERO.code() }> as SelectValidity>::Validity;
        const REPRS: ReprRange<*mut ()> = SLOT_ZERO.range();
        const TAG_WIDTH: u32 = SLOT.tag_width();
        const POINTEE_ALIGNMENT: PointeeAlignment = SLOT_ALIGNMENT;
        fn to_repr(self) -> *mut () {
            let (repr, misaligned) = to_tagged_repr::<Self>(self, Tags::EMPTY);
            assert_aligned::<Self>(misaligned);
            repr
        }
        fn to_tagged_repr(self, tags: Tags) -> (*mut (), usize) {
            match self {
                Self::Empty => tags.set_in(SLOT.unit_repr(0_u8)),
                Self::Inline(value) => {
                    tags.set_in(SLOT.data_repr(1_u8, INLINE.pack(to_bits(value)), SLOT_ALIGNMENT))
                },
                Self::Node(node) => to_tagged_pointer(node, SLOT.pointer_tags(2_u8, 0, tags)),
            }
        }
        fn from_repr(repr: *mut ()) -> Option<Self> {
            match SLOT.discriminant::<u8>(repr) {
                0 if SLOT.is_unit(repr, 0_u8) => Some(Self::Empty),
                1 if SLOT.is_data(repr, 1_u8, INLINE_FIELDS, SLOT_ALIGNMENT) => {
                    match from_bits::<u32>(INLINE.unpack(SLOT.data_bits(repr, SLOT_ALIGNMENT))) {
                        Some(value) => Some(Self::Inline(value)),
                        None => None,
                    }
                },
                2 => match from_pointer::<NonNull<Node>>(SLOT.split(repr).0) {
                    Some(node) => Some(Self::Node(node)),
                    None => None,
                },
                _ => None,
            }
        }
    }

    /// Compiles only where `T` is stored as a pointer to `P`.
    const fn stored_as_a_pointer_to<T: PtrAtom<Pointee = P>, P>() {}

    /// A node on the heap holding `value`, and the pointer to it, whose provenance is the box's.
    fn node(value: u64) -> NonNull<Node> {
        NonNull::from(Box::leak(Box::new(Node { value })))
    }

    /// A block on the heap holding `value`, as `node` makes a node.
    fn block(value: u64) -> NonNull<Block> {
        NonNull::from(Box::leak(Box::new(Block { value })))
    }

    /// Frees a node or a block `node` or `block` made.
    fn free<T>(pointer: NonNull<T>) {
        // SAFETY: `node` or `block` made it with `Box::leak`, and no pointer to it is used after.
        drop(unsafe { Box::from_raw(pointer.as_ptr()) });
    }

    /// The value the node `pointer` points to holds, read through it: Miri refuses the read where
    /// the pointer lost its provenance.
    fn read_node(pointer: NonNull<Node>) -> u64 {
        // SAFETY: each test's node is live while its pointer is read.
        unsafe { pointer.as_ref().value }
    }

    /// The value the block `pointer` points to holds, as `read_node` reads a node.
    fn read_block(pointer: NonNull<Block>) -> u64 {
        // SAFETY: each test's block is live while its pointer is read.
        unsafe { pointer.as_ref().value }
    }

    /// A counter of 0 to 3.
    fn version(count: u8) -> RangedU8<0, 3> {
        RangedU8::new(count).expect("each count of a test lies in 0 to 3")
    }

    #[test]
    fn a_word_takes_its_pointers_repr_and_the_validity_its_fields_promise() {
        repr_and_validity_are::<Head, *mut Node, Partial>();
        repr_and_validity_are::<NullableLink, *mut Node, Total>();
        repr_and_validity_are::<Link, *mut Node, ZeroNiche>();
        repr_and_validity_are::<Option<Link>, *mut Node, ZeroValid>();
        repr_and_validity_are::<Signed, *mut Block, ZeroNiche>();
        repr_and_validity_are::<Outer, *mut Node, ZeroNiche>();
        repr_and_validity_are::<Slot, *mut (), ZeroValid>();
        stored_as_a_pointer_to::<Head, Node>();
        stored_as_a_pointer_to::<Option<Link>, Node>();
        stored_as_a_pointer_to::<NonNull<Node>, Node>();
        stored_as_a_pointer_to::<*const Node, Node>();
        stored_as_a_pointer_to::<Slot, ()>();
        assert_eq!(Head::REPRS, ReprRange::FULL, "a nullable pointer: every repr");
        assert_eq!(Link::REPRS, ReprRange::NONZERO, "a pointer never null: every repr but zero");
        assert_eq!(Slot::REPRS, ReprRange::FULL, "and an enum whose unit is null, every repr");
        assert_eq!(size_of::<Atomic<Option<Link>>>(), size_of::<usize>(), "`None` costs nothing");
    }

    #[test]
    fn a_value_says_which_low_bits_it_keeps_for_tags_and_which_it_leaves_clear() {
        assert_eq!(low_bit_widths::<Head>(), (3, 3), "a counter and a mark, below a node's three");
        assert_eq!(low_bit_widths::<Option<Link>>(), (1, 3), "an `Option` of a word, its word's");
        assert_eq!(low_bit_widths::<NonNull<Node>>(), (0, 3), "a pointer none");
        assert_eq!(low_bit_widths::<Outer>(), (2, 3), "a word over a word, both words' tags");
        assert_eq!(low_bit_widths::<Slot>(), (2, 3), "an enum, its tag");
        assert_eq!(low_bit_widths::<u64>(), (0, 0), "and an integer nothing");
    }

    #[test]
    fn a_word_round_trips_and_its_pointer_keeps_its_provenance() {
        let top = node(7);
        let head = Head { top: Some(top), version: version(3), marked: true };
        let repr = head.to_repr();
        assert_eq!(repr.addr(), top.as_ptr().addr() | 0b111, "the tags in the low bits");
        assert_eq!(Head::from_repr(repr), Some(head), "and back");
        let Some(Head { top: Some(back), .. }) = Head::from_repr(repr) else {
            panic!("a head with a top node, not {:?}", Head::from_repr(repr));
        };
        assert_eq!(read_node(back), 7, "the pointer decoded reads the node");
        free(top);
    }

    #[test]
    fn an_atomic_word_loads_stores_and_exchanges_with_its_pointer_intact() {
        let (first, second) = (node(1), node(2));
        let head = Atomic::new(Head { top: None, version: version(0), marked: false });
        head.store(Head { top: Some(first), version: version(1), marked: false }, Release);
        let seen = head.load(Acquire);
        let next = Head { top: Some(second), version: version(2), ..seen };
        assert_eq!(head.compare_exchange(seen, next, AcqRel, Acquire), Ok(seen), "exchanged");
        assert_eq!(head.swap(seen, AcqRel), next, "and swapped back");
        let Head { top: Some(top), version: count, .. } = head.load(Acquire) else {
            panic!("a head with a top node, not {:?}", head.load(Acquire));
        };
        assert_eq!((read_node(top), count.get()), (1, 1), "the first node, read through the load");
        free(first);
        free(second);
    }

    #[test]
    fn a_nullable_word_decodes_every_repr_and_null_with_its_tags() {
        let marked = NullableLink { next: ptr::null_mut(), deleted: true };
        assert_eq!(marked.to_repr().addr(), 1, "null, tagged");
        assert_eq!(NullableLink::from_repr(marked.to_repr()), Some(marked), "and back");
        for address in [0, 1, 8, 9, usize::MAX] {
            let repr = ptr::without_provenance_mut::<Node>(address);
            let decoded = NullableLink::from_repr(repr).map(Atom::to_repr);
            assert_eq!(decoded, Some(repr), "{address:#x} decodes as a word that encodes to it");
        }
    }

    #[test]
    fn option_of_a_word_whose_pointer_is_never_null_takes_null_alone() {
        static LAST: Atomic<Option<Link>> = Atomic::new(None);
        assert_eq!(Option::<Link>::None.to_repr(), ptr::null_mut(), "`None` is null");
        let tagged_null = ptr::without_provenance_mut::<Node>(1);
        assert_eq!(Option::<Link>::from_repr(tagged_null), None, "null with its tag: no value");
        let next = node(9);
        let some = Some(Link { next, deleted: true });
        assert_eq!(Option::<Link>::from_repr(some.to_repr()), Some(some), "`Some` round-trips");
        LAST.store(some, Release);
        assert_eq!(LAST.swap(None, AcqRel), some, "through an atomic");
        free(next);
    }

    #[test]
    #[should_panic(
        expected = "Link`: the pointer's address has a bit set where its tags go, so it is not \
                    aligned for them"
    )]
    fn a_pointer_with_a_tag_bit_set_is_refused() {
        let mut bytes = [0_u64; 2];
        let misaligned = bytes.as_mut_ptr().wrapping_byte_add(1).cast::<Node>();
        let link = Link { next: NonNull::new(misaligned).expect("not null"), deleted: false };
        let repr = link.to_repr();
        panic!("encoded as {repr:?}, not refused");
    }

    /// A node's pointer one byte past a word's address, so its bit 0, where a link's mark goes, is
    /// set: a pointer no tagged pointer may hold, and none reads through.
    fn misaligned_node() -> NonNull<Node> {
        NonNull::new(ptr::without_provenance_mut(0x1001)).expect("not null")
    }

    #[test]
    #[should_panic(expected = "Link`: the pointer's address has a bit set where its tags go")]
    fn an_exchange_refuses_a_new_word_whose_pointer_has_a_tag_bit_set() {
        let link = Atomic::new(Link { next: NonNull::dangling(), deleted: false });
        let current = link.load(Acquire);
        let new = Link { next: misaligned_node(), deleted: true };
        let exchanged = link.compare_exchange(current, new, AcqRel, Acquire);
        panic!("exchanged as {exchanged:?}, not refused");
    }

    #[test]
    #[should_panic(expected = "Link`: the pointer's address has a bit set where its tags go")]
    fn an_exchange_refuses_a_current_word_whose_pointer_has_a_tag_bit_set() {
        let link = Atomic::new(Link { next: NonNull::dangling(), deleted: false });
        let current = Link { next: misaligned_node(), deleted: false };
        let exchanged = link.compare_exchange_weak(current, link.load(Acquire), AcqRel, Acquire);
        panic!("exchanged as {exchanged:?}, not refused");
    }

    /// A word built in a constant, of a null pointer and its tags.
    static EMPTY: Atomic<Head> =
        Atomic::new(Head { top: None, version: RangedU8::MAX, marked: true });

    /// A word decoded in a constant from null, the one repr whose address a constant reads.
    const NULL: Option<Head> = Head::from_repr(ptr::null_mut());

    #[test]
    fn a_constant_builds_a_word_of_a_null_pointer_and_tags() {
        let empty = EMPTY.load(Acquire);
        assert_eq!((empty.top, empty.version.get(), empty.marked), (None, 3, true), "as built");
    }

    #[test]
    fn a_constant_decodes_a_word_of_a_null_pointer() {
        let null = NULL.map(|null| (null.top, null.version.get(), null.marked));
        assert_eq!(null, Some((None, 0, false)), "with no tag set");
    }

    #[test]
    fn the_projection_lends_each_tag_at_its_own_path_and_the_pointer_read_through_the_word() {
        let first = node(9);
        let head = Atomic::new(Head { top: Some(first), version: version(2), marked: true });
        let HeadFields { top, version: count, marked } = head.fields();
        let _: &AtomicField<Field<Head, 0, Option<NonNull<Node>>>> = top;
        assert_eq!(bits::<Field<Head, 0, Option<NonNull<Node>>>>(), (0, 0), "no tags of its own");
        let Some(loaded) = top.load(Acquire) else {
            panic!("the top node, loaded through the word")
        };
        assert_eq!(read_node(loaded), 9, "and it keeps its provenance");
        free(first);
        let _: &AtomicField<Field<Head, 1, RangedU8<0, 3>>> = count;
        let _: &AtomicField<Field<Head, 2, bool>> = marked;
        assert_eq!(bits::<Field<Head, 1, RangedU8<0, 3>>>(), (0, 2), "the counter from bit 0");
        assert_eq!(bits::<Field<Head, 2, bool>>(), (2, 1), "then the mark");
        assert_eq!((count.load(Acquire).get(), marked.load(Acquire)), (2, true), "each its own");
    }

    #[test]
    fn a_bool_tag_sets_clears_toggles_and_stores_alone() {
        let top = node(4);
        let first = Head { top: Some(top), version: version(2), marked: false };
        let head = Atomic::new(first);
        let marked = head.fields().marked;
        marked.set(Release);
        assert_eq!(canonical(&head), Head { marked: true, ..first }, "set");
        marked.clear(Release);
        assert_eq!(canonical(&head), first, "cleared");
        marked.toggle(Release);
        assert_eq!(canonical(&head), Head { marked: true, ..first }, "toggled on");
        marked.store(false, Release);
        assert_eq!(canonical(&head), first, "and stored off");
        let Head { top: Some(back), .. } = head.load(Acquire) else {
            panic!("a head with a top node, not {:?}", head.load(Acquire));
        };
        assert_eq!(read_node(back), 4, "the pointer reads the node after each");
        free(top);
    }

    /// Checks that each bit of `$atomic`, whose bits are all clear, returns itself before each
    /// test-and-set, -toggle and -clear, and is set by the last.
    macro_rules! each_bit_returns_itself_before {
        ($atomic:ident: $($bit:ident),+) => {{
            let fields = $atomic.fields();
            $(
                let bit = stringify!($bit);
                assert!(!fields.$bit.test_and_set(AcqRel), "{bit}: clear before the set");
                assert!(fields.$bit.test_and_toggle(AcqRel), "{bit}: set before the toggle");
                assert!(!fields.$bit.test_and_clear(AcqRel), "{bit}: clear before the clear");
                assert!(!fields.$bit.test_and_set(AcqRel), "{bit}: and before the last set");
            )+
        }};
    }

    #[test]
    fn each_tag_bit_returns_itself_before_wherever_it_lies() {
        let node = node(6);
        let bits = Atomic::new(Bits { node, low: false, middle: false, high: false });
        each_bit_returns_itself_before!(bits: low, middle, high);
        let all = Bits { node, low: true, middle: true, high: true };
        assert_eq!(canonical(&bits), all, "each bit set, the pointer as it was");
        assert_eq!(read_node(bits.load(Acquire).node), 6, "and reads the node");
        free(node);
    }

    #[test]
    fn a_signed_tag_below_the_pointer_combines_its_bits_alone() {
        let end = <Field<Signed, 2, Nibble> as FieldPath>::END;
        assert_eq!(end, 5, "the top tag governs its own bits, never the pointer's");
        let block = block(3);
        let first = Signed { block, flag: true, nibble: Nibble(3) };
        let signed = Atomic::new(first);
        let nibble = signed.fields().nibble;
        nibble.xor(Nibble(-8), Release);
        assert_eq!(canonical(&signed), Signed { nibble: Nibble(-5), ..first }, "sign set");
        nibble.and(Nibble(7), Release);
        assert_eq!(canonical(&signed), Signed { nibble: Nibble(3), ..first }, "and cleared");
        nibble.not(Release);
        assert_eq!(canonical(&signed), Signed { nibble: Nibble(-4), ..first }, "inverted");
        nibble.or(Nibble(1), Release);
        assert_eq!(canonical(&signed), Signed { nibble: Nibble(-3), ..first }, "ored");
        assert_eq!(read_block(signed.load(Acquire).block), 3, "and the pointer reads the block");
        free(block);
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn each_fetch_form_returns_the_word_before() {
        let block = block(8);
        let first = Signed { block, flag: false, nibble: Nibble(2) };
        let signed = Atomic::new(first);
        let fields = signed.fields();
        assert_eq!(fields.flag.fetch_or(true, AcqRel), first, "the word before the or");
        let flagged = Signed { flag: true, ..first };
        assert_eq!(fields.nibble.fetch_xor(Nibble(1), AcqRel), flagged, "before the xor");
        let xor = Signed { nibble: Nibble(3), ..flagged };
        assert_eq!(fields.nibble.fetch_and(Nibble(1), AcqRel), xor, "before the and");
        let and = Signed { nibble: Nibble(1), ..flagged };
        assert_eq!(fields.nibble.fetch_not(AcqRel), and, "before the not");
        assert_eq!(canonical(&signed), Signed { nibble: Nibble(-2), ..flagged }, "each alone");
        assert_eq!(read_block(signed.load(Acquire).block), 8, "and the pointer reads the block");
        free(block);
    }

    #[test]
    fn a_tag_loads_and_updates_keeping_the_pointer() {
        let top = node(5);
        let first = Head { top: Some(top), version: version(1), marked: true };
        let head = Atomic::new(first);
        let count = head.fields().version;
        let advance = |seen: RangedU8<0, 3>| version(seen.get().wrapping_add(1) & 3);
        assert_eq!(count.update(AcqRel, Acquire, advance), first, "the word before");
        assert_eq!(count.load(Acquire).get(), 2, "advanced");
        let advanced = Head { version: version(2), ..first };
        assert_eq!(count.try_update(AcqRel, Acquire, |_| None), Err(advanced), "declined");
        assert_eq!(canonical(&head), advanced, "with nothing written");
        let Head { top: Some(back), .. } = head.load(Acquire) else {
            panic!("a head with a top node, not {:?}", head.load(Acquire));
        };
        assert_eq!(read_node(back), 5, "the pointer reads the node");
        free(top);
    }

    #[test]
    fn threads_change_their_own_tags_at_once() {
        let top = node(8);
        let first = Head { top: Some(top), version: version(0), marked: false };
        let head = Atomic::new(first);
        let fields = head.fields();
        thread::scope(|scope| {
            scope.spawn(|| fields.marked.set(Release));
            scope.spawn(|| {
                // The word before holds a `NonNull`, which may not cross threads.
                let _ = fields.version.update(AcqRel, Relaxed, |_| version(3));
            });
        });
        assert_eq!(
            canonical(&head),
            Head { version: version(3), marked: true, ..first },
            "both changed"
        );
        free(top);
    }

    /// The path to the inner word's mark, through the outer word's pointer.
    type Marking = Then<Field<Outer, 0, Inner>, Field<Inner, 1, bool>>;

    #[test]
    fn a_word_stacks_its_tags_above_its_pointers_and_a_path_reaches_them() {
        let first = node(10);
        let outer = Outer { inner: Inner { node: first, marked: true }, locked: true };
        assert_eq!(outer.to_repr().addr(), first.as_ptr().addr() | 0b11, "mark at 0, lock at 1");
        assert_eq!(Outer::from_repr(outer.to_repr()), Some(outer), "and back");
        let unmarked = Outer { inner: Inner { node: first, marked: false }, locked: false };
        let atomic = Atomic::new(unmarked);
        let fields = atomic.fields();
        let marking: &AtomicField<Marking> = fields.inner.fields().marked;
        assert_eq!(bits::<Marking>(), (0, 1), "the inner word's mark, at bit 0 of the outer");
        assert_eq!(bits::<Field<Outer, 1, bool>>(), (1, 1), "and the lock above it");
        fields.locked.set(Release);
        assert!(!marking.test_and_set(AcqRel), "the mark, clear before");
        assert_eq!(canonical(&atomic), outer, "both set, each alone");
        let inner = fields.inner.load(Acquire);
        assert_eq!(
            (read_node(inner.node), inner.marked),
            (10, true),
            "the inner word, read through"
        );
        marking.clear(Release);
        assert_eq!(canonical(&atomic), Outer { inner: unmarked.inner, ..outer }, "the lock kept");
        free(first);
    }

    #[test]
    fn a_word_held_in_its_own_pointee_links_one_node_to_the_next() {
        let end = ListLink { next: None, deleted: false };
        let tail = NonNull::from(Box::leak(Box::new(ListNode { key: 2, next: Atomic::new(end) })));
        let head = ListNode { key: 1, next: Atomic::new(ListLink { next: Some(tail), ..end }) };
        assert!(!head.next.fields().deleted.test_and_set(AcqRel), "the head was live");
        let ListLink { next: Some(next), deleted } = head.next.load(Acquire) else {
            panic!("a link to the tail, not {:?}", head.next.load(Acquire));
        };
        // SAFETY: the tail is live until freed below.
        let tail_link = unsafe { next.as_ref() }.next.load(Acquire);
        assert_eq!((head.key, deleted), (1, true), "the head, deleted");
        // SAFETY: as above.
        assert_eq!(unsafe { next.as_ref() }.key, 2, "and the tail, read through its link");
        assert_eq!(tail_link, end, "which ends the list");
        free(tail);
    }

    #[test]
    fn a_tag_prints_its_value() {
        let link = Atomic::new(Link { next: NonNull::dangling(), deleted: true });
        assert_eq!(format!("{:?}", link.fields().deleted), "true", "whether it is deleted");
        let fields = format!("{:?}", link.fields());
        let pointer = format!("{:?}", NonNull::<Node>::dangling());
        assert_eq!(
            fields,
            format!("LinkFields {{ next: {pointer}, deleted: true }}"),
            "and the projection, the pointer and each tag"
        );
    }

    #[test]
    fn a_pointer_enum_holds_each_variant_and_its_nodes_provenance() {
        let first = node(11);
        let slot = Atomic::new(Slot::Empty);
        assert_eq!(Slot::Empty.to_repr(), ptr::null_mut(), "the unit at tag 0 is null");
        assert_eq!(Slot::Inline(5).to_repr().addr(), 5 << 3 | 1, "a value above the alignment");
        assert_eq!(Slot::Node(first).to_repr().addr(), first.as_ptr().addr() | 2, "a node at 2");
        let mut before = Slot::Empty;
        for value in [Slot::Empty, Slot::Inline(u32::MAX), Slot::Inline(0), Slot::Node(first)] {
            assert_eq!(Slot::from_repr(value.to_repr()), Some(value), "{value:?} round-trips");
            assert_eq!(slot.swap(value, AcqRel), before, "{value:?} exchanges the one before");
            assert_eq!(slot.load(Acquire), value, "{value:?} loads back");
            before = value;
        }
        let Slot::Node(back) = slot.load(Acquire) else {
            panic!("a node, not {:?}", slot.load(Acquire));
        };
        assert_eq!(read_node(back), 11, "read through the pointer decoded");
        free(first);
    }

    #[test]
    fn a_pointer_enum_refuses_each_repr_no_value_encodes_to() {
        for (address, what) in [
            (3, "a tag no variant takes"),
            (8, "a unit's clear bits set"),
            (0b101, "a value's clear bit set"),
            (1 << 35 | 1, "a value past a `u32`"),
        ] {
            let repr = ptr::without_provenance_mut(address);
            assert_eq!(Slot::from_repr(repr), None, "{what}: {address:#x}");
        }
    }

    #[test]
    #[should_panic(expected = "the pointer's address has a bit set where its tags go")]
    fn a_pointer_enums_pointer_misaligned_for_its_tag_is_refused() {
        let misaligned =
            NonNull::new(ptr::without_provenance_mut::<Node>(0x1002)).expect("not null");
        let repr = Slot::Node(misaligned).to_repr();
        panic!("encoded as {repr:?}, not refused");
    }

    /// Statics of a unit and a value, which a constant builds, reading no address.
    static UNIT_SLOT: Atomic<Slot> = Atomic::new(Slot::Empty);
    static INLINE_SLOT: Atomic<Slot> = Atomic::new(Slot::Inline(42));

    #[test]
    fn a_constant_builds_a_pointer_enums_unit_and_value() {
        assert_eq!(UNIT_SLOT.load(Acquire), Slot::Empty, "empty");
        assert_eq!(INLINE_SLOT.load(Acquire), Slot::Inline(42), "and a value");
    }
}
