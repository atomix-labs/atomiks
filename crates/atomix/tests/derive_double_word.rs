//! `#[derive(Atom)]` on a struct of pointers two words wide: a pointer beside tags no alignment
//! holds, or beside an integer word `repr = u128` states, which takes the tags; two pointers, the
//! first taking the tags in its low bits; and a pointer to a slice, whose data pointer takes them.
//! Each keeps every pointer's provenance through each encode, decode, exchange and field update,
//! as Miri checks under `-Zmiri-strict-provenance` by reading through each pointer decoded; its
//! range and validity are what its fields promise, and its `Option` is free where its first
//! pointer is never null; a generic one takes two words where its pointers or its repr say so; a
//! tagged word as its pointer keeps its tags reachable; and a node holds an atomic of its own
//! two-word link.

#![cfg(feature = "derive")]
// Loom's `Atomic::new` is not `const`, and its cells exist only inside a model.
#![cfg(not(loom))]
#![cfg(any(target_arch = "aarch64", all(target_arch = "x86_64", target_feature = "cmpxchg16b")))]
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
    use atomix::{Atom, Atomic, DoubleWord, PtrAtom, ReprRange};

    use crate::testing::atom::{low_bit_widths, repr_and_validity_are};

    /// A node, aligned to 8, so a pointer to one leaves three low bits clear.
    #[derive(Debug)]
    #[repr(align(8))]
    struct Node {
        /// What it holds.
        value: u64,
        /// The node below it on a stack.
        below: Atomic<Option<NonNull<Self>>>,
    }

    /// A Treiber stack's head: the top node or none, and a counter of 64 bits, more than any
    /// alignment leaves clear, so it takes a word of its own.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Head {
        /// The top node.
        top: Option<NonNull<Node>>,
        /// How many times the head changed, wrapping.
        version: u64,
    }

    /// A pointer and a mark, which fits the pointer's low bits: one word.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Marked {
        /// The node.
        node: NonNull<Node>,
        /// Whether it is marked.
        marked: bool,
    }

    /// A link and a mark, which would fit the pointer's low bits, in two words, as stated.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u128)]
    struct StatedLink {
        /// The next node, or none.
        next: Option<NonNull<Node>>,
        /// Whether the link is marked.
        marked: bool,
    }

    /// Two pointers, and a mark in the first one's low bits.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Pair {
        /// A node.
        first: NonNull<Node>,
        /// Its partner, or none.
        second: Option<NonNull<Node>>,
        /// Whether the pair is logically deleted.
        marked: bool,
    }

    /// A slice of words, and a seal in its data pointer's low bits.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Chunk {
        /// The words.
        words: NonNull<[u64]>,
        /// Whether the chunk is sealed.
        sealed: bool,
    }

    /// A callback, a trait object's pointer, alone: its vtable pointer beside its data pointer.
    #[derive(Clone, Copy, Debug, Atom)]
    struct Callback(NonNull<dyn Fn(u64) -> u64>);

    /// Two pointers of any two types, and a mark.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct GenericPair<A, B> {
        /// The first.
        first: NonNull<A>,
        /// The second.
        second: NonNull<B>,
        /// A mark, in the first's low bits.
        marked: bool,
    }

    /// A pointer of any type beside a counter, in two words, as stated.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[atom(repr = u128)]
    struct GenericHead<T> {
        /// The top.
        top: Option<NonNull<T>>,
        /// A counter.
        version: u64,
    }

    /// A pointer word of one word: a node, and a mark in its low bits.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct MarkedNode {
        /// The node.
        node: NonNull<Node>,
        /// Whether the node is marked.
        marked: bool,
    }

    /// A marked node beside a counter no alignment holds: a tagged word as a two-word struct's
    /// pointer, its mark below the second word's counter.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct CountedMark {
        /// The marked node.
        #[atom(ptr)]
        top: MarkedNode,
        /// How many times it changed.
        version: u64,
    }

    /// A node, and a marked node beside it: a tagged word as a pair's second pointer, its mark in
    /// the second word's low bits.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct MarkedEdge {
        /// The node the edge leaves.
        from: NonNull<Node>,
        /// The node it reaches, marked when the edge is deleted.
        #[atom(ptr)]
        to: MarkedNode,
    }

    /// A slice of any element, and a seal in its data pointer's low bits.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct GenericChunk<T> {
        /// The elements.
        elements: NonNull<[T]>,
        /// Whether the chunk is sealed.
        sealed: bool,
    }

    /// A boxed node holding `value`, leaked until `free`.
    fn node(value: u64) -> NonNull<Node> {
        NonNull::from(Box::leak(Box::new(Node { value, below: Atomic::new(None) })))
    }

    /// Frees what `pointer` points to, which a box held.
    fn free<T: ?Sized>(pointer: NonNull<T>) {
        // SAFETY: a box held it, and nothing reads it after.
        drop(unsafe { Box::from_raw(pointer.as_ptr()) });
    }

    /// What the node `pointer` points to holds, read through it: Miri checks its provenance.
    fn value(pointer: NonNull<Node>) -> u64 {
        // SAFETY: every node a test reads through is live, and only `below` is written shared.
        unsafe { pointer.as_ref() }.value
    }

    /// Compiles only where `T` is stored as one pointer.
    const fn stored_as_a_pointer<T: PtrAtom>() {}

    #[test]
    fn each_shape_takes_the_words_its_fields_need() {
        repr_and_validity_are::<Head, DoubleWord<*mut Node, usize>, Total>();
        repr_and_validity_are::<StatedLink, DoubleWord<*mut Node, usize>, ZeroValid>();
        repr_and_validity_are::<Pair, DoubleWord<*mut Node, *mut Node>, ZeroNiche>();
        repr_and_validity_are::<Chunk, DoubleWord<*mut (), usize>, ZeroNiche>();
        repr_and_validity_are::<Callback, DoubleWord<*mut (), _>, ZeroNiche>();
        repr_and_validity_are::<GenericPair<u8, u16>, DoubleWord<*mut u8, *mut u16>, Partial>();
        repr_and_validity_are::<GenericHead<u8>, DoubleWord<*mut u8, usize>, ZeroValid>();
        repr_and_validity_are::<GenericChunk<u32>, DoubleWord<*mut (), usize>, Partial>();
        stored_as_a_pointer::<Marked>();
        assert_eq!(size_of::<Atomic<Marked>>(), 8, "a mark that fits: one word");
        assert_eq!(size_of::<Atomic<Head>>(), 16, "a counter that does not: two");
        assert_eq!(low_bit_widths::<Pair>(), (1, 3), "a pair's tags, in its first pointer's bits");
        assert_eq!(low_bit_widths::<Head>(), (0, 3), "an integer word's, none of them");
        assert_eq!(Head::REPRS, ReprRange::FULL, "a nullable pointer: every repr");
        assert_eq!(Pair::REPRS, ReprRange::NONZERO, "a pointer never null: every one but zero");
    }

    #[test]
    fn a_counter_beside_a_pointer_takes_the_second_word() {
        let top = node(1);
        let head = Head { top: Some(top), version: u64::MAX };
        assert_eq!(head.to_repr(), DoubleWord { first: top.as_ptr(), second: usize::MAX }, "split");
        let stated = StatedLink { next: Some(top), marked: true };
        assert_eq!(stated.to_repr().first, top.as_ptr(), "the pointer whole");
        assert_eq!(stated.to_repr().second, 1, "the mark, which would fit its low bits, beside");
        let stray = DoubleWord { first: top.as_ptr(), second: 0b11 };
        assert_eq!(StatedLink::from_repr(stray), None, "a bit above the mark, refused");
        free(top);
    }

    /// Pushes `node` on the stack `head`.
    fn push(head: &Atomic<Head>, node: NonNull<Node>) {
        // ORDERING: Relaxed but for the exchange that lands, whose Release publishes `below` to
        // `pop`'s Acquire: the head is only compared, and the node is no other thread's yet.
        let mut seen = head.load_rmw(Relaxed);
        loop {
            // SAFETY: the node is live; its link is atomic, since a popper may read it.
            unsafe { node.as_ref() }.below.store(seen.top, Relaxed);
            let next = Head { top: Some(node), version: seen.version.wrapping_add(1) };
            match head.compare_exchange_weak(seen, next, Release, Relaxed) {
                Ok(_) => return,
                Err(found) => seen = found,
            }
        }
    }

    /// Pops the top node of the stack `head`, and reads its value through the pointer popped.
    fn pop(head: &Atomic<Head>) -> Option<(NonNull<Node>, u64)> {
        // ORDERING: Acquire on every read of the head, failed exchanges' too, pairing with
        // `push`'s Release, so the top's `below` reads as it was pushed.
        let mut seen = head.load_rmw(Acquire);
        loop {
            let top = seen.top?;
            // SAFETY: nodes go back to the stack, never freed while it lives, so `top` is live
            // even if another thread popped it since.
            let below = unsafe { top.as_ref() }.below.load(Relaxed);
            let after = Head { top: below, version: seen.version.wrapping_add(1) };
            match head.compare_exchange_weak(seen, after, Acquire, Acquire) {
                Ok(_) => return Some((top, value(top))),
                Err(found) => seen = found,
            }
        }
    }

    #[test]
    fn a_counted_head_pops_and_pushes_through_its_pointer_across_threads() {
        let head = Atomic::new(Head { top: None, version: 0 });
        let nodes: Vec<_> = (0..4).map(node).collect();
        for &each in &nodes {
            push(&head, each);
        }
        thread::scope(|scope| {
            for _ in 0..2 {
                scope.spawn(|| {
                    for _ in 0..3 {
                        // A pop and a push back: the reuse a counter tells apart.
                        if let Some((popped, popped_value)) = pop(&head) {
                            assert!(popped_value < 4, "a node's own value, read through it");
                            push(&head, popped);
                        }
                    }
                });
            }
        });
        let mut seen = Vec::new();
        while let Some((_, popped_value)) = pop(&head) {
            seen.push(popped_value);
        }
        seen.sort_unstable();
        assert_eq!(seen, [0, 1, 2, 3], "every node, once");
        // Four pushes, each thread's three pops and pushes back, which never find the stack
        // empty, as two threads hold two of the four nodes at most, and four pops.
        assert_eq!(head.load_rmw(Acquire).version, 20, "every push and pop counted");
        for each in nodes {
            free(each);
        }
    }

    #[test]
    fn each_field_of_a_double_word_is_a_place_that_updates_alone() {
        let (a, b) = (node(2), node(3));
        let head = Atomic::new(Head { top: Some(a), version: 5 });
        let before = head.fields().version.update(AcqRel, Acquire, |version| version * 2);
        assert_eq!(before, Head { top: Some(a), version: 5 }, "the head before");
        assert_eq!(head.fields().version.load_rmw(Acquire), 10, "the counter, doubled");
        let top = head.fields().top.load_rmw(Acquire).expect("the head holds a top");
        assert_eq!(value(top), 2, "the top, its provenance kept");
        let pair = Atomic::new(Pair { first: a, second: Some(b), marked: false });
        let _ = pair.fields().marked.update(AcqRel, Acquire, |marked| !marked);
        let seen = pair.load_rmw(Acquire);
        assert!(seen.marked, "marked");
        assert_eq!((value(seen.first), seen.second.map(value)), (2, Some(3)), "both whole");
        let second = pair.fields().second.load_rmw(Acquire).expect("the pair holds a second");
        assert_eq!(value(second), 3, "the second, read through its place");
        free(a);
        free(b);
    }

    #[cfg(any(target_feature = "lse2", target_feature = "avx"))]
    #[test]
    fn a_double_word_and_its_fields_load_and_store_where_the_target_has_both() {
        let (a, b) = (node(4), node(5));
        let pair = Atomic::new(Pair { first: a, second: None, marked: false });
        pair.store(Pair { first: b, second: Some(a), marked: true }, Release);
        let seen = pair.load(Acquire);
        assert_eq!((value(seen.first), seen.second.map(value)), (5, Some(4)), "stored, loaded");
        assert!(pair.fields().marked.load(Acquire), "and the mark, loaded alone");
        free(a);
        free(b);
    }

    #[test]
    fn a_mark_beside_two_pointers_lies_in_the_first_ones_low_bits() {
        let (a, b) = (node(6), node(7));
        let pair = Pair { first: a, second: Some(b), marked: true };
        assert_eq!(pair.to_repr().first.addr(), a.addr().get() | 1, "the mark in bit 0");
        assert_eq!(pair.to_repr().second, b.as_ptr(), "the second, whole");
        let atomic = Atomic::new(None);
        assert_eq!(size_of::<Atomic<Option<Pair>>>(), 16, "`None` is free");
        assert_eq!(atomic.compare_exchange(None, Some(pair), AcqRel, Acquire), Ok(None), "set");
        let seen = atomic.load_rmw(Acquire).expect("the pair stored");
        assert_eq!((value(seen.first), seen.second.map(value)), (6, Some(7)), "read through");
        free(a);
        free(b);
    }

    #[test]
    #[should_panic(
        expected = "`derive_double_word::tests::Pair`: the pointer's address has a bit \
                               set where its tags go, so it is not aligned for them"
    )]
    fn a_first_pointer_with_its_mark_bit_set_is_refused() {
        let pointer =
            NonNull::new(ptr::without_provenance_mut::<Node>(0x1001)).expect("an address not null");
        let _ = Pair { first: pointer, second: None, marked: false }.to_repr();
    }

    #[test]
    fn a_seal_lies_in_a_slices_data_pointer_and_its_length_beside() {
        let words = NonNull::from(Box::leak(vec![1_u64, 2, 3].into_boxed_slice()));
        let chunk = Atomic::new(Chunk { words, sealed: false });
        let before = chunk.fields().sealed.update(AcqRel, Acquire, |_| true);
        assert!(!before.sealed, "unsealed before");
        let seen = chunk.load_rmw(Acquire);
        assert!(seen.sealed, "sealed after");
        // SAFETY: `words` is live, and nothing writes it.
        assert_eq!(unsafe { seen.words.as_ref() }, [1, 2, 3], "the slice, read through");
        assert_eq!(seen.to_repr().first.addr(), words.cast::<u64>().addr().get() | 1, "bit 0");
        free(words);
    }

    #[test]
    fn a_callback_keeps_its_vtables_provenance_for_a_call() {
        let doubled: Box<dyn Fn(u64) -> u64> = Box::new(|value| value * 2);
        let pointer = NonNull::from(Box::leak(doubled));
        let callback = Atomic::new(Callback(pointer));
        let Callback(seen) = callback.load_rmw(Acquire);
        // SAFETY: the closure is live, and nothing writes it.
        assert_eq!(unsafe { seen.as_ref() }(21), 42, "called through the vtable read back");
        free(pointer);
    }

    #[test]
    fn a_tagged_word_as_a_two_word_structs_pointer_keeps_its_tags_reachable() {
        let a = node(12);
        let counted =
            Atomic::new(CountedMark { top: MarkedNode { node: a, marked: false }, version: 3 });
        assert_eq!(size_of::<Atomic<CountedMark>>(), 16, "the word beside its counter");
        assert!(!counted.fields().top.fields().marked.load_rmw(Acquire), "unmarked");
        let _ = counted.fields().top.fields().marked.update(AcqRel, Acquire, |_| true);
        let seen = counted.load_rmw(Acquire);
        assert!(seen.top.marked, "marked through the word's place");
        assert_eq!(seen.to_repr().first.addr(), a.addr().get() | 1, "the mark in bit 0");
        assert_eq!((value(seen.top.node), seen.version), (12, 3), "the node and its counter");
        let b = node(13);
        let edge = Atomic::new(MarkedEdge { from: a, to: MarkedNode { node: b, marked: false } });
        let _ = edge.fields().to.fields().marked.update(AcqRel, Acquire, |_| true);
        let seen = edge.load_rmw(Acquire);
        assert!(seen.to.marked, "the second pointer's mark, through its place");
        assert_eq!(seen.to_repr().second.addr(), b.addr().get() | 1, "in the second word's bit 0");
        assert_eq!((value(seen.from), value(seen.to.node)), (12, 13), "both nodes, read through");
        free(a);
        free(b);
    }

    /// A boxed `value`, leaked until `free`.
    fn boxed<T>(value: T) -> NonNull<T> {
        NonNull::from(Box::leak(Box::new(value)))
    }

    /// What `pointer` points to, read through it.
    fn read<T: Copy>(pointer: NonNull<T>) -> T {
        // SAFETY: every value a test reads through is live, and nothing writes it.
        unsafe { pointer.read() }
    }

    #[test]
    fn a_generic_pair_head_and_chunk_take_two_words_in_each_instance() {
        let (a, b) = (boxed(8_u64), boxed(9_u16));
        let pair = Atomic::new(GenericPair { first: a, second: b, marked: false });
        let _ = pair.update(AcqRel, Acquire, |seen| GenericPair { marked: true, ..seen });
        let seen = pair.load_rmw(Acquire);
        assert_eq!((read(seen.first), read(seen.second), seen.marked), (8, 9, true), "whole");
        let head = Atomic::new(GenericHead { top: Some(a), version: u64::MAX });
        let seen = head.load_rmw(Acquire);
        assert_eq!((seen.top.map(read), seen.version), (Some(8), u64::MAX), "beside its count");
        let elements = NonNull::from(Box::leak(vec![1_u32, 2].into_boxed_slice()));
        let chunk = Atomic::new(GenericChunk { elements, sealed: true });
        let seen = chunk.load_rmw(Acquire);
        // SAFETY: `elements` is live, and nothing writes it.
        assert_eq!((unsafe { seen.elements.as_ref() }, seen.sealed), (&[1, 2][..], true), "sealed");
        free(elements);
        free(a);
        free(b);
    }

    /// A node of a list, which holds an atomic of its own two-word link: no layout reads the
    /// node's alignment, which its own layout, holding the link, decides.
    struct ListNode {
        /// What it holds.
        value: u64,
        /// The next node, and how many times the link changed.
        next: Atomic<ListLink>,
    }

    /// A list's link to the next node, beside a counter no alignment holds.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct ListLink {
        /// The next node, or none.
        next: Option<NonNull<ListNode>>,
        /// How many times the link changed.
        version: u64,
    }

    #[test]
    fn a_node_holds_an_atomic_of_its_own_two_word_link() {
        let last = NonNull::from(Box::leak(Box::new(ListNode {
            value: 11,
            next: Atomic::new(ListLink { next: None, version: 0 }),
        })));
        let first = NonNull::from(Box::leak(Box::new(ListNode {
            value: 10,
            next: Atomic::new(ListLink { next: Some(last), version: 0 }),
        })));
        assert_eq!(size_of::<Atomic<ListLink>>(), 16, "two words, in the node");
        // SAFETY: both nodes are live, and only their links are written while shared.
        let link = unsafe { first.as_ref() }.next.update(AcqRel, Acquire, |link| ListLink {
            version: link.version.wrapping_add(1),
            ..link
        });
        let next = link.next.expect("the first links the last");
        // SAFETY: as above.
        assert_eq!(unsafe { next.as_ref() }.value, 11, "the last, read through the link");
        // SAFETY: as above.
        assert_eq!(unsafe { first.as_ref() }.next.load_rmw(Acquire).version, 1, "counted");
        // SAFETY: as above.
        assert_eq!(unsafe { first.as_ref() }.value, 10, "the first");
        free(first);
        free(last);
    }
}
