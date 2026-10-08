//! A pointer word or a pointer enum held in an atomic inside the node it points to, as a lock-free
//! list's, stack's and table's nodes hold their links: each derives, since its layout reads its
//! pointer's tag width and never the node's alignment, which the node's own layout, holding the
//! atomic, decides. Two threads run a Harris list's delete, a mark then an unlink, beside an insert
//! before the node deleted, and a Treiber stack's pushes beside its pops, under Miri with strict
//! provenance, reading every node through the pointer decoded.

#![cfg(feature = "derive")]
// Loom's `Atomic::new` is not `const`, and its cells exist only inside a model.
#![cfg(not(loom))]
// The derives on the generic `GenericLink` and `GenericSlot` need it.
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
    use core::ptr::NonNull;
    use std::thread;

    use atomix::ordering::{AcqRel, Acquire, Relaxed, Release};
    use atomix::validity::{Partial, Total, ZeroValid};
    use atomix::{Atom, Atomic, RangedU8};

    use crate::testing::atom::{low_bit_widths, repr_and_validity_are};

    /// A Harris list's node: a key, and the link to the next node, marked once this one is deleted.
    struct ListNode {
        /// Its key.
        key: u64,
        /// The next node, and whether this one is deleted.
        next: Atomic<Link>,
    }

    /// The link a list's node holds: the next node or none, and whether the node that holds the
    /// link is deleted.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Link {
        /// The next node.
        next: Option<NonNull<ListNode>>,
        /// Whether the node that holds this link is deleted.
        deleted: bool,
    }

    /// A Treiber stack's node: a value, and the head the stack had below it.
    struct StackNode {
        /// What it holds.
        value: u64,
        /// The head below it, as it was pushed.
        next: Atomic<Head>,
    }

    /// A Treiber stack's head: the top node or none, and a version, which tells a top popped and
    /// pushed again from the one read.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Head {
        /// The top node.
        top: Option<NonNull<StackNode>>,
        /// How many times the head changed, wrapping.
        version: RangedU8<0, 3>,
    }

    /// A node of a chain that ends in a unit: a pointer enum in its own pointee, which fills its
    /// pointer's niche.
    struct ChainNode {
        /// What it holds.
        value: u64,
        /// The next node, or the end.
        next: Atomic<Next>,
    }

    /// The next node of a chain, or its end, which takes null.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Next {
        /// The end.
        End,
        /// The next node.
        Node(NonNull<ChainNode>),
    }

    /// A table's bucket, which holds a slot: a tagged pointer enum in its own pointee, whose value
    /// held inline lies above the bits the bucket's alignment leaves clear.
    struct Bucket {
        /// What it holds.
        value: u64,
        /// Its slot.
        slot: Atomic<Slot>,
    }

    /// A bucket's slot: empty, a value held inline, or the next bucket.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Slot {
        /// Nothing.
        Empty,
        /// A value, held inline.
        Inline(u32),
        /// The next bucket.
        Bucket(NonNull<Bucket>),
    }

    /// A node of a list whose links are locked: a word over a word, in its own pointee.
    struct GuardedNode {
        /// Its key.
        key: u64,
        /// The next node, whether this one is deleted, and whether the link is locked.
        next: Atomic<Guarded>,
    }

    /// The marked link a guarded node holds.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Marked {
        /// The next node.
        next: Option<NonNull<GuardedNode>>,
        /// Whether the node that holds this link is deleted.
        deleted: bool,
    }

    /// A marked link and a lock above its mark.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Guarded {
        /// The marked link.
        #[atom(ptr)]
        link: Marked,
        /// Whether the link is locked.
        locked: bool,
    }

    /// A list's node of any value, which holds the link to the next.
    struct Node<T> {
        /// What it holds.
        value: T,
        /// The next node, and whether this one is deleted.
        next: Atomic<GenericLink<T>>,
    }

    /// The link a node of any value holds: an instance of a generic word, in its own pointee.
    #[derive(Debug, PartialEq, Eq, Atom)]
    struct GenericLink<T> {
        /// The next node.
        next: Option<NonNull<Node<T>>>,
        /// Whether the node that holds this link is deleted.
        deleted: bool,
    }

    // By hand, so a link of any `T` is `Copy`, as a derive's bound on `T` would not make it.
    impl<T> Clone for GenericLink<T> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<T> Copy for GenericLink<T> {}

    /// A lock above a generic link's mark, held in the value of the node the link points to: its
    /// layout reads the instance's tag width, which reads no node's alignment.
    #[derive(Clone, Copy, Atom)]
    struct LockedLink {
        /// The link.
        #[atom(ptr)]
        link: GenericLink<Entry>,
        /// Whether the link is locked.
        locked: bool,
    }

    /// The value of a generic list's node, which holds a lock on a link to a node like its own.
    struct Entry {
        /// Its key.
        key: u64,
        /// The locked link.
        lock: Atomic<LockedLink>,
    }

    /// A table's bucket of any value, which holds a slot: an instance of a generic pointer enum in
    /// its own pointee, whose code reads the alignment its layout does not.
    struct GenericBucket<T> {
        /// What it holds.
        value: T,
        /// Its slot.
        slot: Atomic<GenericSlot<T>>,
    }

    /// A slot of a bucket of any value: empty, a value held inline, or the next bucket.
    #[derive(Debug, PartialEq, Eq, Atom)]
    enum GenericSlot<T> {
        /// Nothing.
        Empty,
        /// A value, held inline.
        Inline(u32),
        /// The next bucket.
        Bucket(NonNull<GenericBucket<T>>),
    }

    // By hand, as `GenericLink`'s are.
    impl<T> Clone for GenericSlot<T> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<T> Copy for GenericSlot<T> {}

    /// A pointer to `node`, boxed on the heap, whose provenance is the box's; `free` frees it.
    fn boxed<T>(node: T) -> NonNull<T> {
        NonNull::from(Box::leak(Box::new(node)))
    }

    /// Frees a node `boxed` boxed.
    fn free<T>(node: NonNull<T>) {
        // SAFETY: `boxed` boxed it, and no pointer to it is used after.
        drop(unsafe { Box::from_raw(node.as_ptr()) });
    }

    /// The node `pointer` points to, read through it: Miri refuses the read where the pointer lost
    /// its provenance.
    fn node<'a, T>(pointer: NonNull<T>) -> &'a T {
        // SAFETY: `boxed` made the node, aligned and whole, and no test frees one a thread may
        // still read: the list's test frees its nodes once its threads have joined, and the
        // stack's one popper frees a node it popped, which its push no longer reads once the
        // exchange that linked it landed.
        unsafe { pointer.as_ref() }
    }

    /// The version after `version`, wrapping.
    fn next_version(version: RangedU8<0, 3>) -> RangedU8<0, 3> {
        version.checked_add(1).unwrap_or(RangedU8::MIN)
    }

    /// A list's node of `key`, before `next`.
    fn list_node(key: u64, next: Option<NonNull<ListNode>>) -> NonNull<ListNode> {
        boxed(ListNode { key, next: Atomic::new(Link { next, deleted: false }) })
    }

    /// The last node from `head`, the sentinel, whose key lies below `key`, and its link, as read.
    fn find(head: NonNull<ListNode>, key: u64) -> (NonNull<ListNode>, Link) {
        let mut before = head;
        loop {
            // ORDERING: Acquire, pairing with the Release of the exchange that linked the next
            // node, so its key reads whole.
            let link = node(before).next.load(Acquire);
            match link.next {
                Some(next) if node(next).key < key => before = next,
                _ => return (before, link),
            }
        }
    }

    /// Inserts a node of `key` before the first of a key as large, as Harris inserts: an exchange
    /// of the link before it, while it is not deleted, retried where it changed.
    fn insert(head: NonNull<ListNode>, key: u64) -> NonNull<ListNode> {
        let inserted = list_node(key, None);
        loop {
            let (before, link) = find(head, key);
            if link.deleted {
                continue;
            }
            // ORDERING: Relaxed, as no other thread sees the node until the exchange below
            // publishes it with Release, to `find`'s Acquire; one that fails reads nothing.
            node(inserted).next.store(Link { next: link.next, deleted: false }, Relaxed);
            let linked = Link { next: Some(inserted), deleted: false };
            if node(before).next.compare_exchange(link, linked, Release, Relaxed).is_ok() {
                return inserted;
            }
        }
    }

    /// Deletes the node of `key`, as Harris deletes: marks its link, so no insert lands after it,
    /// then swings the link before it past it, finding that link again where an insert changed it.
    fn delete(head: NonNull<ListNode>, key: u64) -> NonNull<ListNode> {
        let (_, link) = find(head, key);
        let deleted = link.next.filter(|&next| node(next).key == key).expect("the key is listed");
        // ORDERING: AcqRel on each change of a link here: Acquire reads the node the link holds,
        // which an insert published with Release, and Release publishes the change to `find`.
        let was_deleted = node(deleted).next.fields().deleted.test_and_set(AcqRel);
        assert!(!was_deleted, "the node was live before its mark");
        loop {
            let (before, link) = find(head, key);
            assert_eq!(link, Link { next: Some(deleted), deleted: false }, "it is still linked");
            let after = node(deleted).next.load(Acquire);
            let unlinked = Link { next: after.next, deleted: false };
            if node(before).next.compare_exchange(link, unlinked, AcqRel, Acquire).is_ok() {
                return deleted;
            }
        }
    }

    /// Each key after `head`, the sentinel, read through the links, beside whether it is deleted.
    fn keys(head: NonNull<ListNode>) -> Vec<(u64, bool)> {
        let mut keys = Vec::new();
        // ORDERING: Acquire, as `find`'s.
        let mut link = node(head).next.load(Acquire);
        while let Some(next) = link.next {
            link = node(next).next.load(Acquire);
            keys.push((node(next).key, link.deleted));
        }
        keys
    }

    /// Pushes `value` on `stack`.
    fn push(stack: &Atomic<Head>, value: u64) {
        let empty = Head { top: None, version: RangedU8::MIN };
        let pushed = boxed(StackNode { value, next: Atomic::new(empty) });
        // ORDERING: Relaxed but for the exchange that lands, whose Release publishes the node to
        // `pop`'s Acquire: the head is only compared, and no thread sees the node before.
        let mut seen = stack.load(Relaxed);
        loop {
            node(pushed).next.store(seen, Relaxed);
            let next = Head { top: Some(pushed), version: next_version(seen.version) };
            match stack.compare_exchange_weak(seen, next, Release, Relaxed) {
                Ok(_) => return,
                Err(found) => seen = found,
            }
        }
    }

    /// Pops the top node of `stack`, where it has one, which the caller frees.
    fn pop(stack: &Atomic<Head>) -> Option<NonNull<StackNode>> {
        // ORDERING: Acquire on every read of the head, pairing with `push`'s Release, so the top
        // node it names reads whole, its own head below Relaxed.
        let mut seen = stack.load(Acquire);
        loop {
            let top = seen.top?;
            let below = node(top).next.load(Relaxed);
            let next = Head { top: below.top, version: next_version(seen.version) };
            match stack.compare_exchange_weak(seen, next, Acquire, Acquire) {
                Ok(_) => return Some(top),
                Err(found) => seen = found,
            }
        }
    }

    #[test]
    fn each_shape_derives_in_its_own_pointee_and_holds_its_pointer_in_one_word() {
        repr_and_validity_are::<Link, *mut ListNode, Total>();
        repr_and_validity_are::<Head, *mut StackNode, Partial>();
        repr_and_validity_are::<Next, *mut (), ZeroValid>();
        repr_and_validity_are::<Slot, *mut (), ZeroValid>();
        repr_and_validity_are::<Guarded, *mut GuardedNode, Total>();
        repr_and_validity_are::<GenericLink<u64>, *mut Node<u64>, ZeroValid>();
        repr_and_validity_are::<LockedLink, *mut Node<Entry>, ZeroValid>();
        repr_and_validity_are::<GenericSlot<u64>, *mut (), ZeroValid>();
        assert_eq!(size_of::<Atomic<Slot>>(), size_of::<usize>(), "a pointer's word");
        assert_eq!(low_bit_widths::<Link>(), (1, 3), "a mark below a list node's three clear bits");
        assert_eq!(low_bit_widths::<Head>(), (2, 3), "a version below a stack node's");
        assert_eq!(low_bit_widths::<Next>(), (0, 3), "a niche, no tag");
        assert_eq!(low_bit_widths::<Slot>(), (2, 3), "a tag of three variants");
        assert_eq!(low_bit_widths::<Guarded>(), (2, 3), "a lock above a mark");
        assert_eq!(low_bit_widths::<GenericLink<u64>>(), (1, 3), "a generic link, its instance's");
        assert_eq!(low_bit_widths::<LockedLink>(), (2, 3), "and a lock above it");
        assert_eq!(low_bit_widths::<GenericSlot<u64>>(), (2, 3), "a generic slot, its instance's");
    }

    #[test]
    fn a_harris_list_deletes_a_node_beside_an_insert_before_it() {
        let last = list_node(30, None);
        let deleted = list_node(20, Some(last));
        let first = list_node(10, Some(deleted));
        let head = list_node(0, Some(first));
        // A node is `Sync`, so each thread takes a reference to the head, and hands back an
        // address, which it reads no node through.
        let sentinel = node(head);
        let (unlinked, inserted) = thread::scope(|scope| {
            let deletion = scope.spawn(|| delete(NonNull::from(sentinel), 20).addr());
            let insertion = scope.spawn(|| insert(NonNull::from(sentinel), 15).addr());
            let unlinked = deletion.join().expect("the deleting thread does not panic");
            (unlinked, insertion.join().expect("nor the inserting one"))
        });
        assert_eq!(unlinked, deleted.addr(), "the node of 20 unlinked");
        assert_eq!(keys(head), [(10, false), (15, false), (30, false)], "and the insert kept");
        let (_, before) = find(head, 15);
        let inserted = before.next.filter(|next| next.addr() == inserted);
        let inserted = inserted.expect("the node of 15 follows the last key below 15");
        assert_eq!(node(inserted).next.load(Acquire).next, Some(last), "before the last node");
        assert_eq!(node(deleted).next.load(Acquire).next, Some(last), "as the deleted one was");
        for list_node in [head, first, inserted, deleted, last] {
            free(list_node);
        }
    }

    #[test]
    fn a_treiber_stack_pops_each_node_pushed_beside_it_once() {
        let stack = Atomic::new(Head { top: None, version: RangedU8::MIN });
        // No other thread pops, so the popper frees each node it pops.
        let mut values: Vec<u64> = thread::scope(|scope| {
            scope.spawn(|| (1..=8).for_each(|value| push(&stack, value)));
            let popper = scope.spawn(|| {
                let popped = (0..8).filter_map(|_| pop(&stack));
                let read_and_freed = |top: NonNull<StackNode>| {
                    let value = node(top).value;
                    free(top);
                    value
                };
                popped.map(read_and_freed).collect()
            });
            popper.join().expect("the popper does not panic")
        });
        while let Some(top) = pop(&stack) {
            values.push(node(top).value);
            free(top);
        }
        values.sort_unstable();
        assert_eq!(values, (1..=8).collect::<Vec<_>>(), "each value, once");
        assert_eq!(stack.load(Acquire).top, None, "and the stack empty");
    }

    #[test]
    fn a_pointer_enum_in_its_own_pointee_links_nodes_through_its_niche_and_its_tag() {
        let end = boxed(ChainNode { value: 2, next: Atomic::new(Next::End) });
        let start = ChainNode { value: 1, next: Atomic::new(Next::Node(end)) };
        let Next::Node(second) = start.next.load(Acquire) else {
            panic!("a node after the start, not {:?}", start.next.load(Acquire));
        };
        assert_eq!(
            (node(second).value, node(second).next.load(Acquire)),
            (2, Next::End),
            "the second node, then the end"
        );
        let last = boxed(Bucket { value: 9, slot: Atomic::new(Slot::Inline(u32::MAX)) });
        let bucket = Bucket { value: 8, slot: Atomic::new(Slot::Empty) };
        assert_eq!(bucket.slot.swap(Slot::Bucket(last), AcqRel), Slot::Empty, "was empty");
        let Slot::Bucket(next) = bucket.slot.load(Acquire) else {
            panic!("the next bucket, not {:?}", bucket.slot.load(Acquire));
        };
        let held = (bucket.value, node(next).value, node(next).slot.load(Acquire));
        assert_eq!(held, (8, 9, Slot::Inline(u32::MAX)), "a value inline above the alignment");
        free(end);
        free(last);
    }

    #[test]
    fn a_nested_word_in_its_own_pointee_marks_and_locks_the_link() {
        let unmarked = |next| Guarded { link: Marked { next, deleted: false }, locked: false };
        let last = boxed(GuardedNode { key: 2, next: Atomic::new(unmarked(None)) });
        let first = GuardedNode { key: 1, next: Atomic::new(unmarked(Some(last))) };
        assert!(!first.next.fields().locked.test_and_set(AcqRel), "the link was unlocked");
        assert!(!first.next.fields().link.fields().deleted.test_and_set(AcqRel), "and live");
        let Guarded { link: Marked { next: Some(next), deleted: true }, locked: true } =
            first.next.load(Acquire)
        else {
            panic!("a locked, deleted link, not {:?}", first.next.load(Acquire));
        };
        assert_eq!((first.key, node(next).key), (1, 2), "read through the link");
        free(last);
    }

    #[test]
    fn a_generic_link_and_a_word_over_it_link_nodes_of_their_own_instance() {
        let end = GenericLink { next: None, deleted: false };
        let last = boxed(Node { value: 2_u64, next: Atomic::new(end) });
        let first =
            Node { value: 1_u64, next: Atomic::new(GenericLink { next: Some(last), ..end }) };
        assert!(!first.next.fields().deleted.test_and_set(AcqRel), "the first was live");
        let link = first.next.load(Acquire);
        let next = link.next.expect("`first` was built linked to `last`");
        assert_eq!(
            (first.value, node(next).value, link.deleted),
            (1, 2, true),
            "the first, deleted"
        );
        let unlocked =
            |next| LockedLink { link: GenericLink { next, deleted: false }, locked: false };
        let entry = |key, next| Entry { key, lock: Atomic::new(unlocked(next)) };
        let tail = boxed(Node {
            value: entry(4, None),
            next: Atomic::new(GenericLink { next: None, deleted: false }),
        });
        let head = Node {
            value: entry(3, Some(tail)),
            next: Atomic::new(GenericLink { next: Some(tail), deleted: false }),
        };
        assert!(!head.value.lock.fields().locked.test_and_set(AcqRel), "the lock was free");
        let locked = head.value.lock.load(Acquire);
        let locked_next = locked.link.next.expect("`head`'s entry was built linked to `tail`");
        assert!(locked.locked, "and is taken");
        assert_eq!((head.value.key, node(locked_next).value.key), (3, 4), "read through the lock");
        free(last);
        free(tail);
    }

    #[test]
    fn a_generic_pointer_enum_in_its_own_pointee_holds_a_value_above_its_alignment() {
        let inline = GenericSlot::<u64>::Inline(5);
        assert_eq!(inline.to_repr().addr(), 5 << 3 | 1, "above a bucket's three clear bits");
        let last = boxed(GenericBucket { value: 2_u64, slot: Atomic::new(inline) });
        let bucket = GenericBucket { value: 1_u64, slot: Atomic::new(GenericSlot::Empty) };
        let before = bucket.slot.swap(GenericSlot::Bucket(last), AcqRel);
        assert_eq!(before, GenericSlot::Empty, "was empty");
        let GenericSlot::Bucket(next) = bucket.slot.load(Acquire) else {
            panic!("the next bucket, not {:?}", bucket.slot.load(Acquire));
        };
        let held = (bucket.value, node(next).value, node(next).slot.load(Acquire));
        assert_eq!(held, (1, 2, inline), "the value, read through the bucket's slot");
        free(last);
    }
}
