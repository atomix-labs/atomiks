//! `#[derive(Atom)]` on a pointer word, a struct of one pointer field beside tag fields packed into
//! the pointer's low bits: its repr is the pointer's, whose provenance each encode, decode,
//! exchange and tag operation keeps, as Miri checks under `-Zmiri-strict-provenance` by reading
//! each node through the pointer decoded; its range and validity are what its fields promise, and
//! its `Option` is free where its pointer is never null; it takes a pointer its type shows, or one
//! marked `#[atom(ptr)]`, generic or not; and its projection lends each tag, and the pointer,
//! read through the word.

#![cfg(feature = "derive")]
// Loom's `Atomic::new` is not `const`, so no static of one builds under it.
#![cfg(not(loom))]
#![feature(const_trait_impl)]

// atomix-core's, by its path; its `//!` says why.
#[cfg(test)]
#[path = "../../atomix-core/tests/testing/mod.rs"]
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

    use atomix::ordering::{AcqRel, Acquire, Relaxed, Release};
    use atomix::validity::{Partial, Total, ZeroNiche, ZeroValid};
    use atomix::{Atom, AtomBitwise, Atomic, AtomicField, Field, PtrAtom, ReprRange};

    use crate::testing::atom::repr_and_validity_are;
    use crate::testing::field::canonical;

    /// A node, aligned to 8, so a pointer to one leaves three low bits clear.
    #[derive(Debug)]
    #[repr(align(8))]
    struct Node {
        /// What it holds.
        value: u64,
    }

    /// A page, aligned to 256, so a pointer to one leaves a byte of low bits clear.
    #[derive(Debug)]
    #[repr(align(256))]
    struct Page {
        /// What it holds.
        value: u64,
    }

    /// A version, which counts pops, every pattern of its two bits a value.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Version {
        /// The first.
        Zero,
        /// The second.
        One,
        /// The third.
        Two,
        /// The fourth.
        Three,
    }

    impl Version {
        /// The version after this one, wrapping.
        const fn next(self) -> Self {
            match self {
                Self::Zero => Self::One,
                Self::One => Self::Two,
                Self::Two => Self::Three,
                Self::Three => Self::Zero,
            }
        }
    }

    /// A Treiber stack's head: the top node or none, a version and a mark, in three bits.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Head {
        /// The top node.
        top: Option<NonNull<Node>>,
        /// The version, which tells a top popped and pushed again from the one read.
        version: Version,
        /// Whether a pop is under way.
        marked: bool,
    }

    /// A Harris list's link, which may be null, and whether its node is deleted.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct NullableLink {
        /// The next node.
        next: *mut Node,
        /// Whether the node that holds this link is deleted.
        deleted: bool,
    }

    /// A link that is never null, so `Option<Link>`'s `None` takes the null repr.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Link {
        /// Whether the node that holds this link is deleted, declared before the pointer.
        deleted: bool,
        /// The next node.
        next: NonNull<Node>,
    }

    /// A link as a tuple struct, whose projection keeps each field's index.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Pair(NonNull<Node>, bool);

    /// Flags of a page, each pattern of a byte a value.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom, AtomBitwise)]
    struct Flags(u8);

    /// A page and a byte of flags below it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Flagged {
        /// The page.
        page: NonNull<Page>,
        /// Its flags.
        flags: Flags,
    }

    /// A tagged pointer to any `T` aligned to 4 or more.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Tagged<T> {
        /// The pointer.
        pointer: NonNull<T>,
        /// Two bits of tag.
        tag: Version,
    }

    /// A pointer to a node, by a name the derive cannot see through.
    type NodePointer = NonNull<Node>;

    /// A link whose pointer field's type hides the pointer, so it is marked.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Aliased {
        /// The next node.
        #[atom(ptr)]
        next: NodePointer,
        /// Whether the node that holds this link is deleted.
        deleted: bool,
    }

    /// A link, its repr stated as the one word it is.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u64)]
    struct Pinned {
        /// The next node.
        next: NonNull<Node>,
        /// Whether the node that holds this link is deleted.
        deleted: bool,
    }

    /// A reference to a node, a derived newtype, stored as its pointer.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct NodeRef(NonNull<Node>);

    /// A head, a derived newtype of a word, stored as the word is, with its tags.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct WrappedHead(#[atom(ptr)] Head);

    /// A word over any pointer atom, marked since a parameter shows no pointer.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Word<P> {
        /// The pointer.
        #[atom(ptr)]
        pointer: P,
        /// One tag.
        flag: bool,
    }

    /// Compiles only where `T` is stored as a pointer to `P`.
    const fn stored_as_a_pointer_to<T: PtrAtom<Pointee = P>, P>() {}

    /// A node on the heap holding `value`, and the pointer to it, whose provenance is the box's.
    fn node(value: u64) -> NonNull<Node> {
        NonNull::from(Box::leak(Box::new(Node { value })))
    }

    /// A page on the heap holding `value`, as `node` makes a node.
    fn page(value: u64) -> NonNull<Page> {
        NonNull::from(Box::leak(Box::new(Page { value })))
    }

    /// Frees a node or a page `node` or `page` made.
    fn free<T>(pointer: NonNull<T>) {
        // SAFETY: `node` or `page` made it with `Box::leak`, and no pointer to it is used after.
        drop(unsafe { Box::from_raw(pointer.as_ptr()) });
    }

    /// The value the node `pointer` points to holds, read through it: Miri refuses the read where
    /// the pointer lost its provenance.
    fn read_node(pointer: NonNull<Node>) -> u64 {
        // SAFETY: each test's node is live while its pointer is read.
        unsafe { pointer.as_ref().value }
    }

    #[test]
    fn a_word_takes_its_pointers_repr_and_the_validity_its_fields_promise() {
        repr_and_validity_are::<Head, *mut Node, ZeroValid>();
        repr_and_validity_are::<NullableLink, *mut Node, Total>();
        repr_and_validity_are::<Link, *mut Node, ZeroNiche>();
        repr_and_validity_are::<Option<Link>, *mut Node, ZeroValid>();
        repr_and_validity_are::<Pair, *mut Node, ZeroNiche>();
        repr_and_validity_are::<Tagged<u32>, *mut u32, Partial>();
        repr_and_validity_are::<Aliased, *mut Node, ZeroNiche>();
        repr_and_validity_are::<Word<Option<NonNull<Node>>>, *mut Node, ZeroValid>();
        repr_and_validity_are::<Pinned, *mut Node, ZeroNiche>();
        stored_as_a_pointer_to::<NodeRef, Node>();
        stored_as_a_pointer_to::<Head, Node>();
        stored_as_a_pointer_to::<Word<NodeRef>, Node>();
        assert_eq!(Head::REPRS, ReprRange::FULL, "a nullable pointer: every repr");
        assert_eq!(Link::REPRS, ReprRange::NONZERO, "a pointer never null: every repr but zero");
        assert_eq!(Tagged::<u32>::REPRS, ReprRange::NONZERO, "and generic alike");
        assert_eq!(size_of::<Atomic<Option<Link>>>(), size_of::<usize>(), "`None` costs nothing");
        assert_eq!(size_of::<Atomic<Option<Tagged<u32>>>>(), size_of::<usize>(), "nor generic");
    }

    #[test]
    fn a_word_and_a_newtype_of_one_say_how_many_low_bits_its_tags_take() {
        assert_eq!(Head::TAG_WIDTH, 3, "three for the counter and the mark");
        assert_eq!(WrappedHead::TAG_WIDTH, 3, "a newtype, its field's");
        assert_eq!(Word::<NodeRef>::TAG_WIDTH, 1, "a generic word, its instance's");
        assert_eq!(NodeRef::TAG_WIDTH, 0, "and a newtype of a plain pointer none");
    }

    #[test]
    fn a_word_round_trips_and_its_pointer_keeps_its_provenance() {
        let top = node(7);
        let head = Head { top: Some(top), version: Version::Three, marked: true };
        let repr = head.to_repr();
        assert_eq!(repr.addr(), top.as_ptr().addr() | 0b111, "the tags in the low bits");
        assert_eq!(Head::from_repr(repr), Some(head), "and back");
        let Some(Head { top: Some(back), .. }) = Head::from_repr(repr) else {
            panic!("a head with a top node, not {:?}", Head::from_repr(repr));
        };
        assert_eq!(read_node(back), 7, "the pointer decoded reads the node");
        let link = Link { deleted: true, next: top };
        assert_eq!(link.to_repr().addr(), top.as_ptr().addr() | 1, "a tag before the pointer");
        free(top);
    }

    #[test]
    fn an_atomic_word_loads_stores_and_exchanges_with_its_pointer_intact() {
        let (first, second) = (node(1), node(2));
        let head = Atomic::new(Head { top: None, version: Version::Zero, marked: false });
        head.store(Head { top: Some(first), version: Version::One, marked: false }, Release);
        let seen = head.load(Acquire);
        let next = Head { top: Some(second), version: seen.version.next(), ..seen };
        assert_eq!(head.compare_exchange(seen, next, AcqRel, Acquire), Ok(seen), "exchanged");
        let Head { top: Some(top), version, .. } = head.load(Acquire) else {
            panic!("a head with a top node, not {:?}", head.load(Acquire));
        };
        assert_eq!(
            (read_node(top), version),
            (2, Version::Two),
            "the second node, read through the load"
        );
        free(first);
        free(second);
    }

    #[test]
    fn each_tag_changes_alone_through_its_place_keeping_the_pointer() {
        let top = node(5);
        let first = Head { top: Some(top), version: Version::One, marked: false };
        let head = Atomic::new(first);
        let fields = head.fields();
        let _: &AtomicField<Field<Head, 1, Version>> = fields.version;
        let _: &AtomicField<Field<Head, 2, bool>> = fields.marked;
        assert!(!fields.marked.test_and_set(AcqRel), "clear before the set");
        assert_eq!(
            fields.version.update(AcqRel, Acquire, Version::next),
            Head { marked: true, ..first },
            "and before the update"
        );
        assert_eq!(fields.version.load(Acquire), Version::Two, "advanced");
        fields.marked.clear(Release);
        assert_eq!(
            canonical(&head),
            Head { version: Version::Two, ..first },
            "each tag changed alone"
        );
        let Head { top: Some(back), .. } = head.load(Acquire) else {
            panic!("a head with a top node, not {:?}", head.load(Acquire));
        };
        assert_eq!(read_node(back), 5, "and the pointer reads the node");
        free(top);
    }

    #[test]
    fn a_bitwise_tag_combines_its_bits_alone() {
        let page = page(3);
        let first = Flagged { page, flags: Flags(0b1010) };
        let flagged = Atomic::new(first);
        let flags = flagged.fields().flags;
        flags.or(Flags(0b0101), Release);
        assert_eq!(flags.load(Acquire), Flags(0b1111), "or");
        flags.and(Flags(0b0110), Release);
        assert_eq!(flags.load(Acquire), Flags(0b0110), "and");
        flags.xor(Flags(0xFF), Release);
        assert_eq!(flags.load(Acquire), Flags(0b1111_1001), "xor");
        flags.not(Release);
        assert_eq!(canonical(&flagged), Flagged { flags: Flags(0b0110), ..first }, "and not");
        // SAFETY: the page is live while its pointer is read.
        let value = unsafe { flagged.load(Acquire).page.as_ref().value };
        assert_eq!(value, 3, "the pointer reads the page");
        free(page);
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn a_tags_fetch_forms_return_the_word_before() {
        let page = page(4);
        let first = Flagged { page, flags: Flags(0b0011) };
        let flagged = Atomic::new(first);
        let flags = flagged.fields().flags;
        assert_eq!(flags.fetch_or(Flags(0b0100), AcqRel), first, "the word before the or");
        let or = Flagged { flags: Flags(0b0111), ..first };
        assert_eq!(flags.fetch_and(Flags(0b0110), AcqRel), or, "before the and");
        free(page);
    }

    #[test]
    fn a_tuple_words_projection_keeps_each_fields_index() {
        let next = node(6);
        let pair = Atomic::new(Pair(next, false));
        let PairFields(pointer, deleted) = pair.fields();
        let _: &AtomicField<Field<Pair, 1, bool>> = deleted;
        let loaded = pointer.load(Acquire);
        // SAFETY: the node is live until the test frees it.
        assert_eq!(unsafe { loaded.as_ref().value }, 6, "the pointer, read through the word");
        pair.fields().1.set(Release);
        assert_eq!(canonical(&pair), Pair(next, true), "the field at 1 set");
        free(next);
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
        let some = Some(Link { deleted: true, next });
        assert_eq!(Option::<Link>::from_repr(some.to_repr()), Some(some), "`Some` round-trips");
        LAST.store(some, Release);
        assert_eq!(LAST.swap(None, AcqRel), some, "through an atomic");
        free(next);
    }

    #[test]
    fn a_generic_word_lays_out_each_instance_by_its_pointee() {
        let mut value = 11_u32;
        let tagged = Tagged { pointer: NonNull::from(&mut value), tag: Version::Two };
        let back = Tagged::<u32>::from_repr(tagged.to_repr());
        assert_eq!(back, Some(tagged), "a `u32`'s two low bits hold the tag");
        let Some(Tagged { pointer, .. }) = back else {
            panic!("decoded, not {back:?}");
        };
        // SAFETY: `value` is live, and nothing else reaches it.
        assert_eq!(unsafe { *pointer.as_ptr() }, 11, "the pointer reads it");
        let atomic = Atomic::new(Some(tagged));
        assert_eq!(atomic.swap(None, AcqRel), Some(tagged), "`Option` of it, in an atomic");
        let instance = Atomic::new(tagged);
        assert_eq!(
            instance.fields().tag.load(Acquire),
            Version::Two,
            "and its tag, through its place"
        );
    }

    #[test]
    fn a_word_over_a_pointer_parameter_or_a_marked_alias_takes_any_pointer_atom() {
        let next = node(4);
        let word = Word { pointer: NodeRef(next), flag: true };
        assert_eq!(Word::<NodeRef>::from_repr(word.to_repr()), Some(word), "a newtype's pointer");
        let nullable = Word { pointer: None::<NonNull<Node>>, flag: true };
        assert_eq!(nullable.to_repr().addr(), 1, "null, tagged");
        let aliased = Atomic::new(Aliased { next, deleted: false });
        aliased.fields().deleted.set(Release);
        assert_eq!(read_node(aliased.load(Acquire).next), 4, "and an alias marked, its tag set");
        free(next);
    }

    #[test]
    fn threads_change_their_own_tags_at_once() {
        let top = node(8);
        let first = Head { top: Some(top), version: Version::Zero, marked: false };
        let head = Atomic::new(first);
        let fields = head.fields();
        thread::scope(|scope| {
            scope.spawn(|| fields.marked.set(Release));
            scope.spawn(|| {
                // The word before holds a `NonNull`, which may not cross threads.
                let _ = fields.version.update(AcqRel, Relaxed, Version::next);
            });
        });
        assert_eq!(canonical(&head), Head { version: Version::One, marked: true, ..first }, "both");
        free(top);
    }

    #[test]
    #[should_panic(
        expected = "Link`: the pointer's address has a bit set where its tags go, so it is not \
                    aligned for them"
    )]
    fn a_pointer_with_a_tag_bit_set_is_refused() {
        let mut bytes = [0_u64; 2];
        let misaligned = bytes.as_mut_ptr().wrapping_byte_add(1).cast::<Node>();
        let link = Link { deleted: false, next: NonNull::new(misaligned).expect("not null") };
        let repr = link.to_repr();
        panic!("encoded as {repr:?}, not refused");
    }

    /// A word built in a constant, of a null pointer and its tags.
    static EMPTY: Atomic<Head> =
        Atomic::new(Head { top: None, version: Version::One, marked: true });

    #[test]
    fn a_constant_builds_a_word_of_a_null_pointer_and_tags() {
        let empty = EMPTY.load(Acquire);
        assert_eq!(
            (empty.top, empty.version, empty.marked),
            (None, Version::One, true),
            "as built"
        );
    }

    #[test]
    fn a_words_projection_prints_each_field() {
        let dangling = format!("{:?}", NonNull::<Node>::dangling());
        let link = Atomic::new(Link { deleted: true, next: NonNull::dangling() });
        let shown = format!("LinkFields {{ deleted: true, next: {dangling} }}");
        assert_eq!(format!("{:?}", link.fields()), shown, "the pointer read through the word");
        let pair = Atomic::new(Pair(NonNull::dangling(), true));
        let shown = format!("PairFields({dangling}, true)");
        assert_eq!(format!("{:?}", pair.fields()), shown, "and in a tuple");
    }
}
