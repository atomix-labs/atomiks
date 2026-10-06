//! Models of a derived packed struct's field operations under loom, through the seam: each is one
//! read-modify-write of loom's atomic, or `update`'s loop of them, so concurrent operations on
//! different fields all land, one test-and-set wins, and a bit set with Release publishes what came
//! before it. A pointer enum, one pointer word, exchanges as any atomic does, publishes the node a
//! Release store holds, and keeps a tag beside it that changes alone. A stack whose nodes each hold
//! an atomic of its head takes a push before it is closed, and its pop reads the node whole, or
//! refuses the push.
//!
//! Each `should_panic` model is a bug loom must report; the rest hold under every interleaving.

// Loom's types exist only under `--cfg loom`; a crate-level `cfg` would empty the crate, and with
// it the attributes `just check-rust-lints` injects.
#[cfg(all(loom, feature = "derive"))]
#[cfg(test)]
mod tests {
    use core::ptr::NonNull;

    use atomiks::model::{Arc, check, thread};
    use atomiks::ordering::{AcqRel, Acquire, Relaxed, Release, RmwOrdering, StoreOrdering};
    use atomiks::{Atom, AtomBitwise, Atomic, AtomicU64, RangedU32};

    /// Flags, each pattern of a byte a value.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom, AtomBitwise)]
    struct Flags(u8);

    /// Flags and a bit, then a count in the top half of the word.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Counted {
        flags: Flags,
        ready: bool,
        spare: RangedU32<0, 0x7F_FFFF>,
        count: u32,
    }

    /// The value each model starts from.
    const ZERO: Counted =
        Counted { flags: Flags(0), ready: false, spare: RangedU32::MIN, count: 0 };

    #[test]
    fn operations_on_different_fields_each_land() {
        check(|| {
            let counted = Arc::new(Atomic::new(ZERO));
            let others: Vec<_> = [Flags(0b01), Flags(0b10)]
                .into_iter()
                .map(|flag| {
                    let counted = Arc::clone(&counted);
                    thread::spawn(move || {
                        // ORDERING: Relaxed throughout: the join publishes each write.
                        counted.fields().count.fetch_add(1, Relaxed);
                        counted.fields().flags.or(flag, Relaxed);
                    })
                })
                .collect();
            counted.fields().ready.set(Relaxed);
            // The loop rebuilds the word from what it read: a stale read would drop a write.
            counted.fields().spare.update(Relaxed, Relaxed, |_| RangedU32::MAX);
            for other in others {
                other.join().expect("no thread panics");
            }
            let last = counted.load_rmw(Acquire);
            let expected =
                Counted { flags: Flags(0b11), ready: true, spare: RangedU32::MAX, count: 2 };
            assert_eq!(last, expected, "every operation landed");
        });
    }

    #[test]
    fn one_test_and_set_wins() {
        check(|| {
            let counted = Arc::new(Atomic::new(ZERO));
            let other = {
                let counted = Arc::clone(&counted);
                thread::spawn(move || counted.fields().ready.test_and_set(AcqRel))
            };
            let mine = counted.fields().ready.test_and_set(AcqRel);
            let theirs = other.join().expect("no thread panics");
            assert_ne!(mine, theirs, "exactly one finds the bit clear");
        });
    }

    /// A value published behind the bit `ready`, which the writer sets with `order`.
    fn publish<O: RmwOrdering + Send + Sync + 'static>(order: O) {
        check(move || {
            let shared = Arc::new((AtomicU64::new(0), Atomic::new(ZERO)));
            let writer = {
                let shared = Arc::clone(&shared);
                thread::spawn(move || {
                    shared.0.store(7, Relaxed);
                    shared.1.fields().ready.set(order);
                })
            };
            if shared.1.fields().ready.test_and_clear(Acquire) {
                assert_eq!(shared.0.load(Relaxed), 7, "the bit publishes the value");
            }
            writer.join().expect("the writer does not panic");
        });
    }

    #[test]
    fn a_release_set_publishes() {
        publish(Release);
    }

    #[test]
    #[should_panic(expected = "the bit publishes the value")]
    fn a_relaxed_set_does_not() {
        publish(Relaxed);
    }

    /// A node, aligned to 8, its value a loom atomic so a model sees its accesses.
    #[repr(align(8))]
    struct Node {
        value: AtomicU64,
    }

    /// Empty, a value held inline, or a node: one pointer word, two tag bits.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Slot {
        Empty,
        Inline(u32),
        Node(NonNull<Node>),
    }

    /// A slot beside a lock at bit 2, above the slot's tag.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct LockedSlot {
        #[atom(ptr)]
        slot: Slot,
        locked: bool,
    }

    /// A node on the heap holding `value`, which `free` frees.
    fn node(value: u64) -> NonNull<Node> {
        NonNull::from(Box::leak(Box::new(Node { value: AtomicU64::new(value) })))
    }

    /// Frees a node `node` made.
    #[expect(unsafe_code, reason = "frees what `node` leaked")]
    fn free(node: NonNull<Node>) {
        // SAFETY: `node` leaked it, and no pointer to it is used after.
        drop(unsafe { Box::from_raw(node.as_ptr()) });
    }

    #[test]
    fn one_exchange_of_a_pointer_enum_wins_and_the_other_sees_it() {
        check(|| {
            let slot = Arc::new(Atomic::new(Slot::Empty));
            let other = {
                let slot = Arc::clone(&slot);
                thread::spawn(move || {
                    let mine = node(7);
                    let won = slot.compare_exchange(Slot::Empty, Slot::Node(mine), AcqRel, Acquire);
                    if won.is_err() {
                        free(mine);
                    }
                    won.map(|_| ()).map_err(|seen| seen == Slot::Inline(5))
                })
            };
            let mine = slot.compare_exchange(Slot::Empty, Slot::Inline(5), AcqRel, Acquire);
            let theirs = other.join().expect("no thread panics");
            assert_ne!(mine.is_ok(), theirs.is_ok(), "exactly one exchange wins");
            assert_ne!(theirs, Err(false), "the loser saw the winner's value");
            match slot.load(Acquire) {
                #[expect(unsafe_code, reason = "reads the node through the pointer decoded")]
                Slot::Node(won) => {
                    // SAFETY: the winner's node is live until freed below.
                    assert_eq!(unsafe { won.as_ref() }.value.load(Relaxed), 7, "the node whole");
                    assert_eq!(mine, Err(Slot::Node(won)), "and the value lost to it");
                    free(won);
                },
                last @ (Slot::Empty | Slot::Inline(_)) => {
                    assert_eq!(last, Slot::Inline(5), "else the value won");
                },
            }
        });
    }

    /// A node published through a pointer enum, stored with `order`.
    fn publish_a_node<O: StoreOrdering + Send + Sync + 'static>(order: O) {
        check(move || {
            let slot = Arc::new(Atomic::new(Slot::Empty));
            let writer = {
                let slot = Arc::clone(&slot);
                thread::spawn(move || {
                    let published = node(0);
                    // SAFETY: the node is live, and no other thread holds it yet.
                    #[expect(unsafe_code, reason = "writes the node before it is published")]
                    unsafe { published.as_ref() }.value.store(7, Relaxed);
                    slot.store(Slot::Node(published), order);
                })
            };
            if let Slot::Node(seen) = slot.load(Acquire) {
                // SAFETY: the node is live until freed below.
                #[expect(unsafe_code, reason = "reads the node through the pointer decoded")]
                let value = unsafe { seen.as_ref() }.value.load(Relaxed);
                assert_eq!(value, 7, "the word publishes the node");
            }
            writer.join().expect("the writer does not panic");
            let Slot::Node(published) = slot.load(Acquire) else {
                panic!("the node is published once the writer joins");
            };
            free(published);
        });
    }

    #[test]
    fn a_release_store_of_a_pointer_enum_publishes_its_node() {
        publish_a_node(Release);
    }

    // Loom reports the node's construction racing the read through the pointer.
    #[test]
    #[should_panic(expected = "Causality violation")]
    fn a_relaxed_store_of_a_pointer_enum_does_not() {
        publish_a_node(Relaxed);
    }

    #[test]
    fn a_tag_beside_a_pointer_enum_and_an_exchange_of_the_enum_both_land() {
        check(|| {
            let locked = Arc::new(Atomic::new(LockedSlot { slot: Slot::Empty, locked: false }));
            let other = {
                let locked = Arc::clone(&locked);
                thread::spawn(move || locked.fields().locked.test_and_set(AcqRel))
            };
            // The loop rebuilds the word from what it read: a stale read would drop the lock.
            let before = locked.fields().slot.load(Relaxed);
            locked.update(AcqRel, Acquire, |seen| LockedSlot { slot: Slot::Inline(9), ..seen });
            let was_locked = other.join().expect("no thread panics");
            assert!(!was_locked, "the lock was clear before");
            assert_eq!(before, Slot::Empty, "the slot read through the word");
            let last = locked.load(Acquire);
            assert_eq!(last, LockedSlot { slot: Slot::Inline(9), locked: true }, "both landed");
        });
    }

    /// A node of a stack that closes, which holds the head below it: a word held in its own
    /// pointee. Its value is a loom atomic, so a model sees its accesses.
    struct StackNode {
        value: AtomicU64,
        next: Atomic<StackHead>,
    }

    /// A Treiber stack's head: the top node or none, and whether the stack is closed.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct StackHead {
        top: Option<NonNull<StackNode>>,
        closed: bool,
    }

    /// An open stack's head, and the head a node holds before it is pushed.
    const OPEN: StackHead = StackHead { top: None, closed: false };

    /// Frees a node `push` made.
    #[expect(unsafe_code, reason = "frees what `push` leaked")]
    fn free_stack_node(node: NonNull<StackNode>) {
        // SAFETY: `push` leaked it, and no pointer to it is used after.
        drop(unsafe { Box::from_raw(node.as_ptr()) });
    }

    /// The node `node` points to, read through it.
    #[expect(unsafe_code, reason = "reads a node through the pointer to it")]
    fn stack_node<'a>(node: NonNull<StackNode>) -> &'a StackNode {
        // SAFETY: no node is freed while another thread may read it: `push` frees one the closed
        // stack refused before any thread could see it, and the model frees one it popped once
        // the pusher has joined.
        unsafe { node.as_ref() }
    }

    /// Pushes a node of `value` with `order`, so a pop with Acquire reads it whole where that is
    /// Release; or, where the stack is closed, frees it. Whether it pushed.
    ///
    /// Its first exchange guesses the stack open and empty, and one that fails reads the head: loom
    /// backtracks to an atomic's last access alone, so a load before the exchange would hide the
    /// exchange from the pop's load, and no model would pop the node beside the push.
    fn push<O: RmwOrdering>(stack: &Atomic<StackHead>, value: u64, order: O) -> bool {
        let node = StackNode { value: AtomicU64::new(0), next: Atomic::new(OPEN) };
        let pushed = NonNull::from(Box::leak(Box::new(node)));
        // ORDERING: Relaxed but for the exchange that lands, whose `order` publishes the node,
        // and its value, to `pop`'s Acquire where it is Release.
        stack_node(pushed).value.store(value, Relaxed);
        let mut seen = OPEN;
        loop {
            if seen.closed {
                free_stack_node(pushed);
                return false;
            }
            stack_node(pushed).next.store(seen, Relaxed);
            let next = StackHead { top: Some(pushed), ..seen };
            match stack.compare_exchange(seen, next, order, Relaxed) {
                Ok(_) => return true,
                Err(found) => seen = found,
            }
        }
    }

    /// Pops the top node, where there is one, which the caller frees.
    fn pop(stack: &Atomic<StackHead>) -> Option<NonNull<StackNode>> {
        // ORDERING: Acquire on every read of the head, pairing with a Release push, so the top
        // node it names reads whole, its own head below Relaxed.
        let mut seen = stack.load(Acquire);
        loop {
            let top = seen.top?;
            let below = stack_node(top).next.load(Relaxed).top;
            match stack.compare_exchange(seen, StackHead { top: below, ..seen }, Acquire, Acquire) {
                Ok(_) => return Some(top),
                Err(found) => seen = found,
            }
        }
    }

    /// A push with `order` beside a pop and a close of a stack held in its own nodes.
    fn push_beside_a_close<O: RmwOrdering + Send + Sync + 'static>(order: O) {
        check(move || {
            let stack = Arc::new(Atomic::new(OPEN));
            let pusher = {
                let stack = Arc::clone(&stack);
                thread::spawn(move || push(&stack, 7, order))
            };
            // A pop beside the push reads its node through the Release the push stores with.
            let early = pop(&stack);
            let was_closed = stack.fields().closed.test_and_set(AcqRel);
            let landed = pusher.join().expect("the pusher does not panic");
            assert!(!was_closed, "open until closed");
            match early.or_else(|| pop(&stack)) {
                Some(popped) => {
                    assert!(landed, "a node popped was pushed");
                    assert_eq!(stack_node(popped).value.load(Relaxed), 7, "and is read whole");
                    free_stack_node(popped);
                },
                None => assert!(!landed, "else the push was refused"),
            }
            assert_eq!(stack.load(Acquire), StackHead { closed: true, ..OPEN }, "and it is closed");
        });
    }

    #[test]
    fn a_stack_held_in_its_own_nodes_takes_a_push_before_it_closes_or_refuses_it() {
        push_beside_a_close(Release);
    }

    // Loom reports the node's construction racing the pop that reads through the pointer.
    #[test]
    #[should_panic(expected = "Causality violation")]
    fn a_relaxed_push_does_not_publish_its_node() {
        push_beside_a_close(Relaxed);
    }
}
