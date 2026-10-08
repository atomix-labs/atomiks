//! `#[derive(Atom)]` on an enum whose variants hold a pointer, and on a pointer word whose pointer
//! is a tagged pointer or such an enum: each is one pointer word, whose provenance each encode,
//! decode, exchange and tag operation keeps, as Miri checks under `-Zmiri-strict-provenance` by
//! reading each node through the pointer decoded.

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
    use core::fmt::Debug;
    use core::mem::size_of;
    use core::ptr::{self, NonNull};
    use std::thread;

    use atomix::ordering::{AcqRel, Acquire, Relaxed, Release};
    use atomix::validity::{Partial, ZeroNiche, ZeroValid};
    use atomix::{Atom, Atomic, AtomicField, Field, ReprRange};

    use crate::testing::atom::{low_bit_widths, repr_and_validity_are};

    /// A node, aligned to 8, so a pointer to one leaves three low bits clear.
    #[derive(Clone, Copy, Debug)]
    #[repr(align(8))]
    struct Node {
        /// What it holds.
        value: u64,
    }

    /// A leaf of a tree, aligned to 8.
    #[derive(Clone, Copy, Debug)]
    #[repr(align(8))]
    struct Leaf {
        /// What it holds.
        value: u64,
    }

    /// A branch of a tree, aligned to 16.
    #[derive(Clone, Copy, Debug)]
    #[repr(align(16))]
    struct Branch {
        /// What its left side holds.
        left: u64,
        /// What its right side holds.
        right: u64,
    }

    /// The next node of a list, or its end: null, as `Option<NonNull<Node>>`, with no tag.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Next {
        /// The end of the list.
        End,
        /// A node.
        Node(NonNull<Node>),
    }

    /// The next node and whether it is deleted, or the end: one unit beside a pointer never null
    /// fills the niche, the end null, and the mark keeps a bit of the enum's own, at bit 0.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum MarkedNext {
        /// The end of the list, null.
        End,
        /// A node, and whether it is deleted.
        Node {
            /// The node.
            node: NonNull<Node>,
            /// Whether it is deleted.
            deleted: bool,
        },
    }

    /// A child of a tree, triomphe's `ArcUnion` and ptr-union's shape: one tag bit, never null.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Child {
        /// A leaf, tag 0.
        Leaf(NonNull<Leaf>),
        /// A branch, tag 1.
        Branch(NonNull<Branch>),
    }

    /// A slot of a table: empty, an inline value, or a node; two tag bits, the value above the
    /// three bits a node's alignment clears.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Slot {
        /// Nothing, tag 0: null.
        Empty,
        /// A value held inline, tag 1.
        Inline(u32),
        /// A node, tag 2.
        Node(NonNull<Node>),
    }

    /// An entry of a slab: the index of the next vacant entry, or the value it holds.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Entry {
        /// Vacant, tag 0, beside the next vacant entry's index.
        Vacant(u32),
        /// Occupied, tag 1.
        Occupied(NonNull<Node>),
    }

    /// An entry declared pointer first, so null, a null pointer's tag 0, is no value's repr.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Occupancy {
        /// Occupied, tag 0.
        Occupied(NonNull<Node>),
        /// Vacant, tag 1.
        Vacant(u32),
    }

    /// A pointer word whose pointer is a node, and a mark at bit 0.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Inner {
        /// The node.
        node: NonNull<Node>,
        /// Whether the node is marked.
        marked: bool,
    }

    /// A pointer word whose pointer is a pointer word: its lock at bit 1, above the mark.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Outer {
        /// The marked node.
        #[atom(ptr)]
        inner: Inner,
        /// Whether it is locked.
        locked: bool,
    }

    /// A list's head: its next node or its end, beside a mark at bit 0, above `Next`'s no tags.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Head {
        /// The next node.
        #[atom(ptr)]
        next: Next,
        /// Whether the list is closed.
        closed: bool,
    }

    /// A tag beside a variant's tag: the slot, its two bits, and a lock at bit 2.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct LockedSlot {
        /// The slot.
        #[atom(ptr)]
        slot: Slot,
        /// Whether it is locked.
        locked: bool,
    }

    /// A tag beside an enum of two pointers: the child, its tag bit, and a lock at bit 1.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct LockedChild {
        /// The child.
        #[atom(ptr)]
        child: Child,
        /// Whether it is locked.
        locked: bool,
    }

    /// A variant of a pointer and a tag field: the tag field above the enum's tag.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[expect(
        variant_size_differences,
        reason = "in memory a node is wider than a count; in an atomic, one word"
    )]
    enum Link {
        /// A count, tag 0.
        Count(u16),
        /// A node and whether it is deleted, tag 1, the mark at bit 1.
        Node {
            /// The node.
            node: NonNull<Node>,
            /// Whether it is deleted.
            deleted: bool,
        },
    }

    /// A marked node or a plain one: the marked one keeps its mark at bit 0, so the enum's tag lies
    /// at bit 1, above every variant's pointer's own tags.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Either {
        /// A marked node, tag 0.
        Marked(#[atom(ptr)] Inner),
        /// A plain node, tag 1.
        Plain(NonNull<Node>),
    }

    /// A variant of a nullable pointer, which leaves no niche, so the unit takes tag 0.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Maybe {
        /// Nothing, tag 0: null.
        Unset,
        /// A node or none, tag 1.
        Set(Option<NonNull<Node>>),
    }

    /// `Next` over any pointer, its unit first, so zero is the end in every instance: the niche's
    /// null beside a pointer never null, tag 0 beside one that may be null.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum List<P> {
        /// The end of the list.
        End,
        /// A node.
        Node(#[atom(ptr)] P),
    }

    /// `Next`, its repr stated as the one word it is.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = usize)]
    enum Pinned {
        /// The end of the list.
        End,
        /// A node.
        Node(NonNull<Node>),
    }

    /// `Child` with parameters: triomphe's `ArcUnion<A, B>` shape.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Union<A, B> {
        /// The first.
        First(NonNull<A>),
        /// The second.
        Second(NonNull<B>),
    }

    /// A node on the heap holding `value`, and the pointer to it, whose provenance is the box's.
    fn node(value: u64) -> NonNull<Node> {
        NonNull::from(Box::leak(Box::new(Node { value })))
    }

    /// Frees what `node`, or a test's own `Box::leak`, made.
    fn free<T>(pointer: NonNull<T>) {
        // SAFETY: it was made with `Box::leak`, and no pointer to it is used after.
        drop(unsafe { Box::from_raw(pointer.as_ptr()) });
    }

    /// The value the node `pointer` points to holds, read through it: Miri refuses the read where
    /// the pointer lost its provenance.
    fn read_node(pointer: NonNull<Node>) -> u64 {
        // SAFETY: each test's node is live while its pointer is read.
        unsafe { pointer.as_ref().value }
    }

    /// The value of `child`'s leaf, or the right side of its branch, read through its pointer, as
    /// [`read_node`] reads.
    fn read_child(child: Child) -> u64 {
        match child {
            // SAFETY: each test's leaf is live while its pointer is read.
            Child::Leaf(leaf) => unsafe { leaf.as_ref().value },
            // SAFETY: each test's branch is live while its pointer is read.
            Child::Branch(branch) => unsafe { branch.as_ref().right },
        }
    }

    /// Stores each of `values` in an atomic, then loads, exchanges and decodes it, checking the
    /// value back each time, and reading each node through the pointer decoded.
    fn round_trips<T: Atom<Repr = *mut ()> + PartialEq + Debug>(
        values: &[T], nodes: impl Fn(T) -> Vec<u64>, expected: &[Vec<u64>],
    ) {
        let atomic = Atomic::from(values[0]);
        for (value, expected) in values.iter().zip(expected) {
            assert_eq!(T::from_repr(value.to_repr()), Some(*value), "{value:?} round-trips");
            atomic.store(*value, Release);
            let loaded = atomic.load(Acquire);
            assert_eq!(loaded, *value, "{value:?} loads back");
            assert_eq!(&nodes(loaded), expected, "{value:?} reads its nodes through the load");
            assert_eq!(
                atomic.compare_exchange(*value, *value, AcqRel, Acquire),
                Ok(*value),
                "{value:?} exchanges"
            );
        }
    }

    #[test]
    fn each_enum_takes_a_pointer_repr_and_the_validity_of_the_variant_zero_holds() {
        repr_and_validity_are::<Next, *mut (), ZeroValid>();
        repr_and_validity_are::<Child, *mut (), ZeroNiche>();
        repr_and_validity_are::<Slot, *mut (), ZeroValid>();
        repr_and_validity_are::<Entry, *mut (), ZeroValid>();
        repr_and_validity_are::<Occupancy, *mut (), ZeroNiche>();
        repr_and_validity_are::<Link, *mut (), ZeroValid>();
        repr_and_validity_are::<Either, *mut (), ZeroNiche>();
        repr_and_validity_are::<Union<Leaf, Branch>, *mut (), Partial>();
        repr_and_validity_are::<Outer, *mut Node, ZeroNiche>();
        repr_and_validity_are::<Head, *mut (), ZeroValid>();
        repr_and_validity_are::<Pinned, *mut (), ZeroValid>();
        assert_eq!(Pinned::End.to_repr(), Next::End.to_repr(), "pinned alike, the end null");
        assert_eq!(Next::REPRS, ReprRange::FULL, "null is the end");
        assert_eq!(Child::REPRS, ReprRange::NONZERO, "no child is null");
        assert_eq!(Occupancy::REPRS, ReprRange::NONZERO, "nor an occupancy");
        assert_eq!(Union::<Leaf, Branch>::REPRS, ReprRange::NONZERO, "nor a generic union");
    }

    #[test]
    fn a_generic_enum_whose_first_variant_is_a_unit_decodes_zero_as_it() {
        repr_and_validity_are::<List<NonNull<Node>>, *mut (), ZeroValid>();
        repr_and_validity_are::<List<*mut Node>, *mut (), ZeroValid>();
        let null = ptr::null_mut();
        assert_eq!(List::<NonNull<Node>>::from_repr(null), Some(List::End), "the niche's null");
        assert_eq!(List::<*mut Node>::from_repr(null), Some(List::End), "and tag 0's null");
        let tagged = List::<*mut Node>::Node(ptr::null_mut()).to_repr();
        assert_eq!(tagged.addr(), 1, "beside a null node, tagged 1");
    }

    #[test]
    fn each_layout_keeps_its_tags_where_the_pointers_alignment_leaves_room() {
        assert_eq!(low_bit_widths::<Next>(), (0, 3), "the niche takes no tag");
        assert_eq!(low_bit_widths::<MarkedNext>(), (1, 3), "and keeps a tag field's bit");
        assert_eq!(
            low_bit_widths::<Child>(),
            (1, 3),
            "two pointers, one tag bit, below the least aligned"
        );
        assert_eq!(low_bit_widths::<Slot>(), (2, 3), "three variants, two bits");
        assert_eq!(low_bit_widths::<Entry>(), (1, 3), "two variants, one bit");
        assert_eq!(low_bit_widths::<Link>(), (2, 3), "a tag field above the tag");
        assert_eq!(low_bit_widths::<Either>(), (2, 3), "the tag above the pointers' own tag");
        assert_eq!(low_bit_widths::<Inner>(), (1, 3), "a word of one tag");
        assert_eq!(low_bit_widths::<Outer>(), (2, 3), "and one above it");
        assert_eq!(low_bit_widths::<Head>(), (1, 3), "a tag beside the niche");
        assert_eq!(low_bit_widths::<LockedSlot>(), (3, 3), "a tag beside a slot's two");
        assert_eq!(low_bit_widths::<Union<Leaf, Branch>>(), (1, 3), "and generic alike");
    }

    #[test]
    fn each_variant_round_trips_and_its_pointer_keeps_its_provenance() {
        let (first, second) = (node(1), node(2));
        round_trips(
            &[Next::End, Next::Node(first)],
            |next| match next {
                Next::End => vec![],
                Next::Node(node) => vec![read_node(node)],
            },
            &[vec![], vec![1]],
        );
        assert_eq!(Next::End.to_repr(), ptr::null_mut(), "the end is null");
        assert_eq!(Next::Node(first).to_repr(), first.as_ptr().cast(), "a node its pointer");
        round_trips(
            &[
                MarkedNext::End,
                MarkedNext::Node { node: first, deleted: true },
                MarkedNext::Node { node: second, deleted: false },
            ],
            |next| match next {
                MarkedNext::End => vec![],
                MarkedNext::Node { node, .. } => vec![read_node(node)],
            },
            &[vec![], vec![1], vec![2]],
        );
        assert_eq!(MarkedNext::End.to_repr(), ptr::null_mut(), "the end of a marked list is null");
        assert_eq!(
            MarkedNext::Node { node: first, deleted: true }.to_repr().addr(),
            first.as_ptr().addr() | 1,
            "a node beside its mark, with no tag"
        );
        round_trips(
            &[Slot::Empty, Slot::Inline(u32::MAX), Slot::Inline(0), Slot::Node(first)],
            |slot| match slot {
                Slot::Node(node) => vec![read_node(node)],
                Slot::Empty | Slot::Inline(_) => vec![],
            },
            &[vec![], vec![], vec![], vec![1]],
        );
        assert_eq!(Slot::Inline(5).to_repr().addr(), 5 << 3 | 1, "a value above the alignment");
        assert_eq!(
            Slot::Node(first).to_repr().addr(),
            first.as_ptr().addr() | 2,
            "a node tagged 2"
        );
        round_trips(
            &[Entry::Vacant(7), Entry::Occupied(second)],
            |entry| match entry {
                Entry::Occupied(node) => vec![read_node(node)],
                Entry::Vacant(_) => vec![],
            },
            &[vec![], vec![2]],
        );
        round_trips(
            &[
                Link::Count(9),
                Link::Node { node: first, deleted: true },
                Link::Node { node: second, deleted: false },
            ],
            |link| match link {
                Link::Node { node, .. } => vec![read_node(node)],
                Link::Count(_) => vec![],
            },
            &[vec![], vec![1], vec![2]],
        );
        assert_eq!(
            Link::Node { node: first, deleted: true }.to_repr().addr(),
            first.as_ptr().addr() | 0b11,
            "the mark above the tag"
        );
        let marked = Inner { node: first, marked: true };
        round_trips(
            &[Either::Marked(marked), Either::Plain(second)],
            |either| match either {
                Either::Marked(Inner { node, .. }) | Either::Plain(node) => vec![read_node(node)],
            },
            &[vec![1], vec![2]],
        );
        assert_eq!(
            Either::Plain(second).to_repr().addr(),
            second.as_ptr().addr() | 0b10,
            "the tag above bit 0"
        );
        round_trips(
            &[Maybe::Unset, Maybe::Set(None), Maybe::Set(Some(first))],
            |maybe| match maybe {
                Maybe::Set(Some(node)) => vec![read_node(node)],
                Maybe::Unset | Maybe::Set(None) => vec![],
            },
            &[vec![], vec![], vec![1]],
        );
        assert_eq!(Maybe::Set(None).to_repr().addr(), 1, "a null pointer, tagged 1");
        free(first);
        free(second);
    }

    #[test]
    fn two_pointees_of_two_alignments_share_one_word() {
        let leaf = NonNull::from(Box::leak(Box::new(Leaf { value: 3 })));
        let branch = NonNull::from(Box::leak(Box::new(Branch { left: 4, right: 5 })));
        let child = Atomic::new(Child::Leaf(leaf));
        let Child::Leaf(back) = child.load(Acquire) else { panic!("a leaf") };
        // SAFETY: the leaf is live.
        assert_eq!(unsafe { back.as_ref().value }, 3, "the leaf, read through the load");
        child.store(Child::Branch(branch), Release);
        let Child::Branch(back) = child.load(Acquire) else { panic!("a branch") };
        // SAFETY: the branch is live.
        assert_eq!(unsafe { back.as_ref().right }, 5, "the branch, read through the load");
        let union = Atomic::new(Union::<Leaf, Branch>::Second(branch));
        let Union::Second(back) = union.swap(Union::First(leaf), AcqRel) else {
            panic!("the second")
        };
        // SAFETY: the branch is live.
        assert_eq!(unsafe { back.as_ref().left }, 4, "a generic union's, through the swap");
        free(leaf);
        free(branch);
    }

    #[test]
    fn option_of_an_enum_whose_zero_never_decodes_takes_null() {
        let first = node(6);
        assert_eq!(size_of::<Atomic<Option<Child>>>(), size_of::<usize>(), "`None` costs nothing");
        assert_eq!(Option::<Occupancy>::None.to_repr(), ptr::null_mut(), "`None` is null");
        let slot = Atomic::new(Option::<Occupancy>::None);
        slot.store(Some(Occupancy::Vacant(0)), Release);
        assert_eq!(slot.load(Acquire), Some(Occupancy::Vacant(0)), "a vacant entry is not `None`");
        slot.store(Some(Occupancy::Occupied(first)), Release);
        let Some(Occupancy::Occupied(back)) = slot.load(Acquire) else { panic!("occupied") };
        assert_eq!(read_node(back), 6, "read through the `Option`");
        let generic = Atomic::new(Option::<Union<Node, Leaf>>::None);
        generic.store(Some(Union::First(first)), Release);
        let Some(Union::First(back)) = generic.load(Acquire) else { panic!("the first") };
        assert_eq!(read_node(back), 6, "and a generic union's");
        let outer = Atomic::new(Option::<Outer>::None);
        outer.store(
            Some(Outer { inner: Inner { node: first, marked: true }, locked: true }),
            Release,
        );
        let Some(Outer { inner: Inner { node: back, .. }, .. }) = outer.load(Acquire) else {
            panic!("an outer")
        };
        assert_eq!(read_node(back), 6, "and a nested word's");
        free(first);
    }

    #[test]
    fn a_word_stacks_its_tags_above_its_pointers() {
        let first = node(8);
        let outer = Outer { inner: Inner { node: first, marked: true }, locked: true };
        assert_eq!(
            outer.to_repr().addr(),
            first.as_ptr().addr() | 0b11,
            "the mark at 0, the lock at 1"
        );
        let atomic =
            Atomic::new(Outer { inner: Inner { node: first, marked: false }, locked: false });
        let fields = atomic.fields();
        let _: &AtomicField<Field<Outer, 1, bool>> = fields.locked;
        fields.locked.set(Release);
        assert!(!fields.inner.fields().marked.test_and_set(AcqRel), "the mark, clear before");
        assert_eq!(atomic.load(Acquire), outer, "both set, each alone");
        let inner = fields.inner.load(Acquire);
        assert_eq!(
            (read_node(inner.node), inner.marked),
            (8, true),
            "the inner word, read through the outer"
        );
        fields.inner.fields().marked.clear(Release);
        assert!(!atomic.load(Relaxed).inner.marked, "the mark cleared");
        assert!(atomic.load(Relaxed).locked, "the lock kept");
        free(first);
    }

    #[test]
    fn a_tag_beside_an_enum_changes_alone() {
        let first = node(9);
        let head = Atomic::new(Head { next: Next::End, closed: false });
        head.fields().closed.set(Release);
        assert_eq!(head.load(Acquire), Head { next: Next::End, closed: true }, "the end, closed");
        assert_eq!(head.fields().next.load(Acquire), Next::End, "the enum read through the word");
        head.store(Head { next: Next::Node(first), closed: false }, Release);
        assert!(!head.fields().closed.test_and_set(AcqRel), "open before");
        let Head { next: Next::Node(back), closed: true } = head.load(Acquire) else {
            panic!("a closed node")
        };
        assert_eq!(read_node(back), 9, "the node kept");
        let locked = Atomic::new(LockedSlot { slot: Slot::Inline(77), locked: false });
        assert!(!locked.fields().locked.test_and_set(AcqRel), "the lock at bit 2, clear");
        assert_eq!(
            locked.load(Acquire),
            LockedSlot { slot: Slot::Inline(77), locked: true },
            "the value kept"
        );
        locked.store(LockedSlot { slot: Slot::Node(first), locked: true }, Release);
        locked.fields().locked.clear(Release);
        let LockedSlot { slot: Slot::Node(back), locked: false } = locked.load(Acquire) else {
            panic!("an unlocked node")
        };
        assert_eq!(read_node(back), 9, "and the node's provenance");
        free(first);
    }

    #[test]
    fn a_repr_no_value_encodes_to_is_refused() {
        // A tag no variant takes, and bits a unit or a value leaves clear.
        assert_eq!(Slot::from_repr(ptr::without_provenance_mut(3)), None, "tag 3");
        assert_eq!(Slot::from_repr(ptr::without_provenance_mut(8)), None, "an empty slot's bits");
        assert_eq!(
            Slot::from_repr(ptr::without_provenance_mut(0b101)),
            None,
            "a value's clear bit"
        );
        assert_eq!(Slot::from_repr(ptr::without_provenance_mut(1 << 35 | 1)), None, "past a `u32`");
        let eight = ptr::without_provenance_mut::<()>(8);
        let node = NonNull::new(eight.cast()).expect("8 is not null");
        assert_eq!(Next::from_repr(eight), Some(Next::Node(node)), "a node anywhere");
        assert_eq!(Child::from_repr(ptr::null_mut()), None, "a null leaf");
        // Bit 2 is the node's: only the tag's and the tag field's bits must be clear.
        let repr = ptr::without_provenance_mut(0b1101);
        assert_eq!(
            Link::from_repr(repr).map(Atom::to_repr),
            Some(repr),
            "a node at 0xC, canonical"
        );
    }

    #[test]
    #[should_panic(expected = "the pointer's address has a bit set where its tags go")]
    fn a_pointer_misaligned_for_the_tag_is_refused() {
        let misaligned =
            NonNull::new(ptr::without_provenance_mut::<Node>(0x1002)).expect("not null");
        let repr = Slot::Node(misaligned).to_repr();
        panic!("encoded as {repr:?}, not refused");
    }

    #[test]
    #[should_panic(expected = "the pointer's address has a bit set where its tags go")]
    fn a_pointer_misaligned_for_the_tag_of_one_of_several_pointer_variants_is_refused() {
        let misaligned =
            NonNull::new(ptr::without_provenance_mut::<Branch>(0x1001)).expect("not null");
        let repr = Child::Branch(misaligned).to_repr();
        panic!("encoded as {repr:?}, not refused");
    }

    #[test]
    fn an_update_that_keeps_the_pointer_writes_back_the_repr_it_read() {
        let first = node(1);
        let leaf = NonNull::from(Box::leak(Box::new(Leaf { value: 3 })));
        let branch = NonNull::from(Box::leak(Box::new(Branch { left: 4, right: 5 })));
        let slot = Atomic::new(Slot::Node(first));
        let repr = Slot::Node(first).to_repr();
        assert_eq!(slot.update(AcqRel, Acquire, |slot| slot), Slot::Node(first), "a slot's node");
        let Slot::Node(back) = slot.load(Acquire) else { panic!("the node") };
        assert_eq!((read_node(back), Slot::Node(back).to_repr()), (1, repr), "kept, and read");
        for (kept, value) in [(Child::Leaf(leaf), 3), (Child::Branch(branch), 5)] {
            let child = Atomic::new(kept);
            assert_eq!(child.update(AcqRel, Acquire, |child| child), kept, "{kept:?}");
            let back = child.load(Acquire);
            assert_eq!((read_child(back), back.to_repr()), (value, kept.to_repr()), "{kept:?}");
        }
        free(first);
        free(leaf);
        free(branch);
    }

    #[test]
    fn an_update_that_changes_a_tag_beside_a_pointer_keeps_the_pointer() {
        let (first, second) = (node(1), node(2));
        let leaf = NonNull::from(Box::leak(Box::new(Leaf { value: 3 })));
        let branch = NonNull::from(Box::leak(Box::new(Branch { left: 4, right: 5 })));
        let either = Atomic::new(Either::Marked(Inner { node: second, marked: false }));
        either.update(AcqRel, Acquire, |either| match either {
            Either::Marked(inner) => Either::Marked(Inner { marked: !inner.marked, ..inner }),
            Either::Plain(node) => Either::Plain(node),
        });
        let Either::Marked(Inner { node: back, marked: true }) = either.load(Acquire) else {
            panic!("the marked node, its mark toggled")
        };
        assert_eq!(read_node(back), 2, "and read through");
        let locked_slot = Atomic::new(LockedSlot { slot: Slot::Node(first), locked: false });
        locked_slot.update(AcqRel, Acquire, |seen| LockedSlot { locked: true, ..seen });
        let LockedSlot { slot: Slot::Node(back), locked: true } = locked_slot.load(Acquire) else {
            panic!("the node, locked")
        };
        assert_eq!(read_node(back), 1, "a word's slot read through");
        for (child, value) in [(Child::Leaf(leaf), 3), (Child::Branch(branch), 5)] {
            let locked_child = Atomic::new(LockedChild { child, locked: false });
            locked_child.update(AcqRel, Acquire, |seen| LockedChild { locked: true, ..seen });
            let LockedChild { child: back, locked: true } = locked_child.load(Acquire) else {
                panic!("{child:?}, locked")
            };
            assert_eq!((back, read_child(back)), (child, value), "a word's {child:?} read through");
        }
        free(first);
        free(second);
        free(leaf);
        free(branch);
    }

    /// A list's sentinel node, a static.
    static SENTINEL: Node = Node { value: 11 };

    /// A niche's pointer, which no tag shares, so a constant checks no alignment: a static of it.
    static FIRST: Atomic<Next> = Atomic::new(Next::Node(NonNull::from_ref(&SENTINEL)));

    /// Statics of each variant but a tagged pointer's, whose alignment a constant cannot check.
    static EMPTY: Atomic<Slot> = Atomic::new(Slot::Empty);
    static INLINE: Atomic<Slot> = Atomic::new(Slot::Inline(42));
    static VACANT: Atomic<Entry> = Atomic::new(Entry::Vacant(3));

    #[test]
    fn a_constant_builds_a_unit_and_a_value() {
        assert_eq!(EMPTY.load(Acquire), Slot::Empty, "empty");
        assert_eq!(INLINE.load(Acquire), Slot::Inline(42), "a value");
        assert_eq!(VACANT.load(Acquire), Entry::Vacant(3), "a vacant entry");
        let Next::Node(sentinel) = FIRST.load(Acquire) else { panic!("the sentinel") };
        assert_eq!(read_node(sentinel), 11, "and a niche's pointer to a static");
    }

    #[test]
    fn threads_exchange_a_pointer_enum_and_read_each_node() {
        let nodes: Vec<NonNull<Node>> = (0..4).map(node).collect();
        // Each thread's node, handed over in an atomic, which may cross threads
        // where a pointer may not.
        let sources: Vec<Atomic<Slot>> =
            nodes.iter().map(|node| Atomic::new(Slot::Node(*node))).collect();
        let slot = Atomic::new(Slot::Empty);
        thread::scope(|scope| {
            for (index, source) in sources.iter().enumerate() {
                let slot = &slot;
                scope.spawn(move || {
                    let mut seen = slot.load(Acquire);
                    let new = if index % 2 == 0 {
                        source.load(Acquire)
                    } else {
                        Slot::Inline(u32::try_from(index).expect("an index below 4"))
                    };
                    while let Err(now) = slot.compare_exchange_weak(seen, new, AcqRel, Acquire) {
                        seen = now;
                    }
                    if let Slot::Node(back) = slot.load(Acquire) {
                        assert!(read_node(back) < 4, "a node of the test's, read through the load");
                    }
                });
            }
        });
        for node in nodes {
            free(node);
        }
    }
}
