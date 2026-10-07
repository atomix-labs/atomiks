//! A double word keeps each pointer's provenance through every operation, as Miri checks under
//! `-Zmiri-strict-provenance` by reading through each pointer decoded: a pair of pointers, a
//! pointer and a counter (a Treiber stack's head, written out as the derive writes it, popped and
//! pushed by two threads), two pointers with a mark in the first one's low bits, and the wide
//! pointers, a slice's, a `str`'s and a trait object's, whose vtable pointer keeps its provenance
//! for a call through it, and whose exchange compares it. Each obeys the `Atom` laws, and its
//! `Option` costs nothing; a `static` holds pointers that have no provenance, a marked null among
//! them, and takes pointers at run time.

#![feature(const_trait_impl)]
#![cfg_attr(all(wide, not(loom)), feature(ptr_metadata))]
// Loom's cells exist only inside a model, and a `const` `new` not at all.
#![cfg(not(loom))]
#![cfg(wide)]

#[cfg(test)]
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
    use core::sync::atomic::{AtomicPtr, Ordering};
    use std::thread;

    use atomiks_core::__private::{
        PackedField, PackedLayout, PointeeAlignment, PointerWordLayout, SelectPointerWordRepr,
        Tags, Words, assert_aligned, from_bits, from_repr, to_bits, to_tagged_repr,
    };
    use atomiks_core::ordering::{AcqRel, Acquire, Relaxed, Release};
    use atomiks_core::validity::{Partial, Total, ZeroNiche, ZeroValid};
    use atomiks_core::{Atom, Atomic, DoubleWord, Primitive, ReprRange};

    use crate::testing::atom::{low_bit_widths, repr_and_validity_are};
    use crate::testing::law::{
        Promise, assert_holds, decodes_as_promised, keeps_provenance, round_trips,
    };
    use crate::testing::pointer_word::pointer_word;

    /// A node, aligned to 8, so a pointer to one leaves three low bits clear.
    #[derive(Debug)]
    #[repr(align(8))]
    struct Node {
        /// What it holds.
        value: u64,
        /// The node below it on a stack.
        next: AtomicPtr<Self>,
    }

    /// A boxed node holding `value`, leaked until `free`.
    fn node(value: u64) -> NonNull<Node> {
        NonNull::from(Box::leak(Box::new(Node { value, next: AtomicPtr::new(ptr::null_mut()) })))
    }

    /// Frees what `pointer` points to, which a box held.
    fn free<T: ?Sized>(pointer: NonNull<T>) {
        // SAFETY: a box held it, and nothing reads it after.
        drop(unsafe { Box::from_raw(pointer.as_ptr()) });
    }

    /// What the node `pointer` points to holds, read through it: Miri checks its provenance.
    fn value(pointer: NonNull<Node>) -> u64 {
        // SAFETY: every node a test reads through is live, and only `next` is written while shared.
        unsafe { pointer.as_ref() }.value
    }

    pointer_word! {
        /// A link to a node, or none, marked when its node is deleted.
        struct Link, projected as LinkFields { 0 => next: Option<NonNull<Node>>, 1 => deleted: bool }
    }

    /// A marked link to no node beside no partner, built in a constant: a null pointer with its
    /// mark set has no provenance for the constant to expose.
    static MARKED_NULLS: Atomic<(Link, Option<NonNull<Node>>)> =
        Atomic::new((Link { next: None, deleted: true }, None));

    #[test]
    fn a_static_holds_a_marked_null_and_takes_pointers_at_run_time() {
        let (link, partner) = MARKED_NULLS.load_rmw(Acquire);
        assert_eq!((link, partner), (Link { next: None, deleted: true }, None), "as built");
        assert_eq!(link.to_repr().addr(), 1, "the mark in a null pointer's bit 0");
        let (a, b) = (node(14), node(15));
        let marked = (Link { next: Some(a), deleted: true }, Some(b));
        let found = MARKED_NULLS.compare_exchange((link, partner), marked, AcqRel, Acquire);
        assert_eq!(found, Ok((link, partner)), "the nulls, exchanged for pointers");
        let (link, partner) = MARKED_NULLS.load_rmw(Acquire);
        assert_eq!(
            (link.next.map(value), partner.map(value)),
            (Some(14), Some(15)),
            "read through"
        );
        MARKED_NULLS.store_rmw((Link { next: None, deleted: true }, None), Release);
        free(a);
        free(b);
    }

    #[test]
    fn a_pair_of_pointers_is_a_double_word_of_them() {
        repr_and_validity_are::<
            (NonNull<Node>, NonNull<u8>),
            DoubleWord<*mut Node, *mut u8>,
            Partial,
        >();
        repr_and_validity_are::<(*mut Node, *const u8), DoubleWord<*mut Node, *mut u8>, ZeroValid>(
        );
        assert_eq!(<(NonNull<Node>, *mut u8) as Atom>::REPRS, ReprRange::NONZERO, "never zero");
        assert_eq!(<(*mut Node, *mut u8) as Atom>::REPRS, ReprRange::FULL, "or any repr");
        let widths = low_bit_widths::<(NonNull<Node>, NonNull<Node>)>();
        assert_eq!(widths, (0, 3), "no tags, and the first pointer's three clear low bits");
    }

    #[test]
    fn two_pointers_keep_their_provenance_through_every_operation() {
        let (a, b, c, d) = (node(1), node(2), node(3), node(4));
        let pair = Atomic::new((a, b));
        let (first, second) = pair.load_rmw(Acquire);
        assert_eq!((value(first), value(second)), (1, 2), "built, then read back");

        assert_eq!(pair.compare_exchange((a, b), (c, d), AcqRel, Acquire), Ok((a, b)), "exchanged");
        let (first, second) = pair.load_rmw(Acquire);
        assert_eq!((value(first), value(second)), (3, 4), "the new pair, read through");

        let found = pair.compare_exchange((a, b), (a, a), AcqRel, Acquire);
        let Err((first, second)) = found else { panic!("exchanged over a stale pair: {found:?}") };
        assert_eq!((value(first), value(second)), (3, 4), "what a failed exchange found");

        let (first, second) = pair.update(AcqRel, Acquire, |(first, second)| (second, first));
        assert_eq!((value(first), value(second)), (3, 4), "the pair before the update");
        pair.store_rmw((d, a), Release);
        let (first, second) = pair.into_inner();
        assert_eq!((value(first), value(second)), (4, 1), "stored, out of the atomic");
        for each in [a, b, c, d] {
            free(each);
        }
    }

    #[cfg(wide_load_store)]
    #[test]
    fn a_load_and_a_store_keep_both_pointers_provenance() {
        let (a, b) = (node(5), node(6));
        let pair = Atomic::new((a, a));
        pair.store((b, a), Release);
        let (first, second) = pair.load(Acquire);
        assert_eq!((value(first), value(second)), (6, 5), "stored, then loaded");
        free(a);
        free(b);
    }

    #[test]
    fn an_option_of_a_pair_never_null_takes_the_null_pair_and_costs_nothing() {
        type Pair = (NonNull<Node>, NonNull<Node>);
        assert_eq!(size_of::<Atomic<Option<Pair>>>(), 16, "one 16-byte atomic");
        assert_eq!(None::<Pair>.to_repr().packed_bits(), 0, "`None` the null pair");
        let (a, b) = (node(7), node(8));
        let maybe: Atomic<Option<Pair>> = Atomic::new(None);
        assert_eq!(maybe.compare_exchange(None, Some((a, b)), AcqRel, Acquire), Ok(None), "filled");
        let Some((first, second)) = maybe.load_rmw(Acquire) else {
            panic!("the pair went missing")
        };
        assert_eq!((value(first), value(second)), (7, 8), "the pair, read through");
        free(a);
        free(b);
    }

    /// A Treiber stack's head: the top node, or none, and an ABA counter in a word of its own, as
    /// the derive lays out a struct of a pointer and tags no alignment holds.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Head {
        /// The top node.
        top: Option<NonNull<Node>>,
        /// How many times the head has changed.
        version: u64,
    }

    /// How many words `Head` takes: two, since 64 bits of tags are more than any alignment holds.
    const HEAD_WORDS: u32 = PointerWordLayout::word_count(
        <Option<NonNull<Node>> as Atom>::TAG_WIDTH,
        &[PackedField::new(<u64 as Atom>::REPRS, 0)],
    );

    /// Where `version` lies: at bit 64, the integer word's first.
    const VERSION: PackedField = PackedField::new(
        <u64 as Atom>::REPRS,
        PointerWordLayout::tag_fields_offset(
            HEAD_WORDS,
            <Option<NonNull<Node>> as Atom>::TAG_WIDTH,
        ),
    );

    /// How `Head` lays out its fields.
    const HEAD: PointerWordLayout = PointerWordLayout::in_words(
        HEAD_WORDS,
        <Option<NonNull<Node>> as Atom>::TAG_WIDTH,
        PackedLayout::new(&[VERSION]),
    );

    /// `Head`'s repr, of as many words as it takes.
    type HeadRepr = <Words<HEAD_WORDS> as SelectPointerWordRepr<*mut Node>>::Repr;

    const _: () = HEAD.assert_tags_fit::<Head, Option<NonNull<Node>>>("tag field `version` needs");

    // SAFETY: the repr is the top node's pointer beside the counter's bits in the integer word,
    // which `join` writes and `split_words` reads back; every pointer and every counter is a value
    // and the counter fills its word, so every repr decodes, as `Total` says, and its range is
    // every repr; `from_repr` gives the value whose repr it is; and the pointer may cross threads,
    // as `AtomicPtr`'s does.
    const unsafe impl Atom for Head {
        type Repr = HeadRepr;
        type Validity = Total;
        const REPRS: ReprRange<HeadRepr> = HEAD.range(<Option<NonNull<Node>> as Atom>::REPRS);
        const TAG_WIDTH: u32 = HEAD.tag_width();
        const POINTEE_ALIGNMENT: PointeeAlignment =
            <Option<NonNull<Node>> as Atom>::POINTEE_ALIGNMENT;
        #[inline]
        fn to_repr(self) -> HeadRepr {
            let (repr, misaligned) = self.to_tagged_repr(Tags::EMPTY);
            assert_aligned::<Self>(misaligned);
            repr
        }
        #[inline]
        fn to_tagged_repr(self, tags: Tags) -> (HeadRepr, usize) {
            let bits = VERSION.pack(to_bits(self.version));
            let (pointer, misaligned) = to_tagged_repr(self.top, HEAD.pointer_tags(bits, tags));
            (HEAD.join_words(pointer, bits), misaligned)
        }
        #[inline]
        fn from_repr(repr: HeadRepr) -> Option<Self> {
            let Some((pointer, bits)) = HEAD.split_words(repr) else {
                return None;
            };
            match (from_repr(pointer), from_bits(VERSION.unpack(bits))) {
                (Some(top), Some(version)) => Some(Self { top, version }),
                _ => None,
            }
        }
    }

    /// Pushes `node` on the stack `head`.
    fn push(head: &Atomic<Head>, node: NonNull<Node>) {
        // ORDERING: Relaxed but for the exchange that lands, whose Release publishes `next` to
        // `pop`'s Acquire: the head is only compared, and the node is no other thread's yet.
        let mut seen = head.load_rmw(Relaxed);
        loop {
            let below = seen.top.map_or(ptr::null_mut(), NonNull::as_ptr);
            // SAFETY: the node is live; its `next` is atomic, since a popper may read it.
            // ORDERING: Relaxed; the exchange below publishes it, with Release.
            unsafe { node.as_ref() }.next.store(below, Ordering::Relaxed);
            let next = Head { top: Some(node), version: seen.version.wrapping_add(1) };
            match head.compare_exchange(seen, next, Release, Relaxed) {
                Ok(_) => return,
                Err(found) => seen = found,
            }
        }
    }

    /// Pops the top node of the stack `head`, and reads its value through the pointer popped.
    fn pop(head: &Atomic<Head>) -> Option<(NonNull<Node>, u64)> {
        // ORDERING: Acquire on every read of the head, failed exchanges' too, pairing with
        // `push`'s Release, so the top's `next` reads as it was pushed.
        let mut seen = head.load_rmw(Acquire);
        loop {
            let top = seen.top?;
            // SAFETY: nodes go back to the stack, never freed while it lives, so `top` is live
            // even if another thread popped it since.
            // ORDERING: Relaxed; the Acquire read of the head that found `top` pairs with the
            // Release exchange that pushed it.
            let next = unsafe { top.as_ref() }.next.load(Ordering::Relaxed);
            let after = Head { top: NonNull::new(next), version: seen.version.wrapping_add(1) };
            match head.compare_exchange(seen, after, Acquire, Acquire) {
                Ok(_) => return Some((top, value(top))),
                Err(found) => seen = found,
            }
        }
    }

    #[test]
    fn a_pointer_beside_tags_no_alignment_holds_takes_a_second_word_for_them() {
        repr_and_validity_are::<Head, DoubleWord<*mut Node, usize>, Total>();
        assert_eq!(low_bit_widths::<Head>(), (0, 3), "no tag in the pointer's low bits");
        let empty = Head { top: None, version: 7 }.to_repr();
        assert_eq!(empty, DoubleWord { first: ptr::null_mut(), second: 7 }, "the counter's word");
        let top = node(20);
        laws(Head { top: Some(top), version: u64::MAX }, |head| head.top.map(value));
        let full = DoubleWord { first: top.as_ptr(), second: usize::MAX };
        assert_eq!(Head::from_repr(full), Some(Head { top: Some(top), version: u64::MAX }), "any");
        free(top);
    }

    #[test]
    fn a_counted_head_pops_and_pushes_through_its_pointer_across_threads() {
        assert_eq!(size_of::<Atomic<Head>>(), 16, "a pointer and a counter in one atomic");
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
                            assert!(
                                popped_value < 4,
                                "a node's own value, read through the pointer"
                            );
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

    /// Two pointers and a mark in the first one's low bits, as the derive lays out two pointers and
    /// a tag.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Marked {
        /// A node, whose pointer's low bits hold the mark.
        first: NonNull<Node>,
        /// Its partner.
        second: NonNull<Node>,
        /// Whether the pair is logically deleted.
        deleted: bool,
    }

    /// The pair of pointers `Marked` holds, the first taking its tag.
    type Pointers = (NonNull<Node>, NonNull<Node>);

    /// Where the mark lies: above the first pointer's own tags, none.
    const MARK: PackedField =
        PackedField::new(<bool as Atom>::REPRS, <Pointers as Atom>::TAG_WIDTH);

    /// How `Marked` lays out its fields.
    const MARKED: PointerWordLayout =
        PointerWordLayout::new(<Pointers as Atom>::TAG_WIDTH, PackedLayout::new(&[MARK]));

    const _: () = MARKED.assert_tags_fit::<Marked, Pointers>("tag field `deleted` needs");

    // SAFETY: the repr is the pair's, its first pointer offset by the mark, which `set_in` sets
    // only in a bit the pointer has clear, keeping its provenance, else the encode is refused;
    // neither pointer is null, so the repr is never zero, and zero never decodes; `from_repr`
    // splits the mark back off and decodes the pair, giving the value whose repr it is; and the
    // pointers may cross threads.
    const unsafe impl Atom for Marked {
        type Repr = <Pointers as Atom>::Repr;
        type Validity = ZeroNiche;
        const REPRS: ReprRange<Self::Repr> = MARKED.range(<Pointers as Atom>::REPRS);
        const TAG_WIDTH: u32 = MARKED.tag_width();
        const POINTEE_ALIGNMENT: PointeeAlignment = <Pointers as Atom>::POINTEE_ALIGNMENT;
        #[inline]
        fn to_repr(self) -> Self::Repr {
            let (repr, misaligned) = self.to_tagged_repr(Tags::EMPTY);
            assert_aligned::<Self>(misaligned);
            repr
        }
        #[inline]
        fn to_tagged_repr(self, tags: Tags) -> (Self::Repr, usize) {
            let bits = MARK.pack(to_bits(self.deleted));
            to_tagged_repr((self.first, self.second), MARKED.pointer_tags(bits, tags))
        }
        #[inline]
        fn from_repr(repr: Self::Repr) -> Option<Self> {
            let (pointers, bits) = MARKED.split(repr);
            match (from_repr::<Pointers>(pointers), from_bits(MARK.unpack(bits))) {
                (Some((first, second)), Some(deleted)) => Some(Self { first, second, deleted }),
                _ => None,
            }
        }
    }

    #[test]
    fn a_mark_in_the_first_pointers_low_bits_leaves_both_pointers_whole() {
        repr_and_validity_are::<Marked, DoubleWord<*mut Node, *mut Node>, ZeroNiche>();
        assert_eq!(low_bit_widths::<Marked>(), (1, 3), "the mark in the first pointer's bit 0");
        let (a, b) = (node(10), node(11));
        let marked = Marked { first: a, second: b, deleted: true };
        assert_eq!(marked.to_repr().first.addr(), a.addr().get() | 1, "the mark");
        assert_eq!(marked.to_repr().second, b.as_ptr(), "the second pointer, whole");
        laws(marked, |seen| (value(seen.first), value(seen.second), seen.deleted));
        let link = Atomic::new(Marked { first: a, second: b, deleted: false });
        assert_eq!(link.load_rmw(Acquire).first, a, "the first pointer, unmarked");
        let before = link.update(AcqRel, Acquire, |seen| Marked { deleted: true, ..seen });
        assert!(!before.deleted, "unmarked before");
        let seen = link.load_rmw(Acquire);
        assert!(seen.deleted, "marked after");
        assert_eq!((value(seen.first), value(seen.second)), (10, 11), "both read through");
        free(a);
        free(b);
    }

    #[test]
    fn a_slice_pointer_keeps_its_length_and_provenance() {
        let first = NonNull::from(Box::leak(vec![1_u8, 2, 3].into_boxed_slice()));
        let second = NonNull::from(Box::leak(vec![4_u8, 5].into_boxed_slice()));
        let slice = Atomic::new(first);
        assert_eq!(slice.compare_exchange(first, second, AcqRel, Acquire), Ok(first), "exchanged");
        let seen = slice.load_rmw(Acquire);
        // SAFETY: `second` is live, and nothing writes it.
        assert_eq!(unsafe { seen.as_ref() }, [4, 5], "the new slice, read through");
        let maybe: Atomic<Option<NonNull<[u8]>>> = Atomic::new(None);
        assert_eq!(size_of::<Atomic<Option<NonNull<[u8]>>>>(), 16, "`None` costs nothing");
        maybe.store_rmw(Some(first), Release);
        let length = maybe.load_rmw(Acquire).map(|seen| {
            // SAFETY: `first` is live, and nothing writes it.
            unsafe { seen.as_ref() }.len()
        });
        assert_eq!(length, Some(3), "its length");
        free(first);
        free(second);
    }

    #[test]
    fn a_str_pointer_keeps_its_length_and_provenance() {
        let text = NonNull::from(Box::leak(String::from("double").into_boxed_str()));
        let word = Atomic::new(text.as_ptr().cast_const());
        let seen = word.update(AcqRel, Acquire, |seen| seen);
        // SAFETY: `text` is live, and nothing writes it.
        assert_eq!(unsafe { &*seen }, "double", "read through");
        free(text);
    }

    /// How to measure a shape, which a `dyn` pointer calls through its vtable.
    trait Shape: Debug {
        /// The shape's area.
        fn area(&self) -> u64;
    }

    /// A square of a side.
    #[derive(Debug)]
    struct Square(u64);

    impl Shape for Square {
        fn area(&self) -> u64 {
            self.0.wrapping_mul(self.0)
        }
    }

    /// A rectangle of two sides.
    #[derive(Debug)]
    struct Rectangle(u64, u64);

    impl Shape for Rectangle {
        fn area(&self) -> u64 {
            self.0.wrapping_mul(self.1)
        }
    }

    /// A boxed shape, leaked until `free`.
    fn shape<S: Shape + 'static>(shape: S) -> NonNull<dyn Shape> {
        let boxed: Box<dyn Shape> = Box::new(shape);
        NonNull::from(Box::leak(boxed))
    }

    /// The area of the shape `pointer` points to, called through its vtable.
    fn area(pointer: NonNull<dyn Shape>) -> u64 {
        // SAFETY: every shape a test calls through is live, and nothing writes it.
        unsafe { pointer.as_ref() }.area()
    }

    #[test]
    fn a_trait_objects_pointer_keeps_its_vtables_provenance_for_a_call() {
        let (square, rectangle) = (shape(Square(3)), shape(Rectangle(2, 5)));
        let shared = Atomic::new(square);
        let found = shared.compare_exchange(square, rectangle, AcqRel, Acquire);
        assert_eq!(found.map(area), Ok(9), "the square before");
        assert_eq!(area(shared.load_rmw(Acquire)), 10, "called through the vtable read");
        let maybe: Atomic<Option<NonNull<dyn Shape>>> = Atomic::new(None);
        assert_eq!(size_of::<Atomic<Option<NonNull<dyn Shape>>>>(), 16, "`None` costs nothing");
        maybe.store_rmw(Some(square), Release);
        assert_eq!(maybe.load_rmw(Acquire).map(area), Some(9), "`Some`, called through");
        free(square);
        free(rectangle);
    }

    #[test]
    fn a_trait_objects_exchange_compares_its_vtable_so_current_comes_from_a_load() {
        let (square, rectangle) = (shape(Square(3)), shape(Rectangle(2, 5)));
        let shared = Atomic::new(square);
        // The square's data pointer beside the rectangle's vtable: the same address, another
        // vtable.
        let other_vtable =
            NonNull::from_raw_parts(square.cast::<()>(), ptr::metadata(rectangle.as_ptr()));
        let found = shared.compare_exchange(other_vtable, rectangle, AcqRel, Acquire);
        assert_eq!(found.map_err(area), Err(9), "the square, whose vtable differs");
        let current = shared.load_rmw(Acquire);
        let found = shared.compare_exchange(current, rectangle, AcqRel, Acquire);
        assert_eq!(found.map(area), Ok(9), "exchanged against the pointer loaded");
        let tried = shared.try_update(AcqRel, Acquire, |seen| (area(seen) == 10).then_some(square));
        assert_eq!(tried.map(area), Ok(10), "the rectangle, replaced by `try_update`");
        let declined = shared.try_update(AcqRel, Acquire, |_| None);
        assert_eq!(declined.map_err(area), Err(9), "declined, the square kept");
        free(square);
        free(rectangle);
    }

    #[test]
    fn a_wide_pointer_is_its_data_pointer_and_its_metadata() {
        repr_and_validity_are::<NonNull<[u64]>, DoubleWord<*mut (), usize>, ZeroNiche>();
        repr_and_validity_are::<*mut [u64], DoubleWord<*mut (), usize>, Total>();
        repr_and_validity_are::<*const str, DoubleWord<*mut (), usize>, Total>();
        repr_and_validity_are::<NonNull<dyn Shape>, DoubleWord<*mut (), _>, ZeroNiche>();
        repr_and_validity_are::<*mut dyn Shape, DoubleWord<*mut (), _>, ZeroNiche>();
        assert_eq!(<*mut dyn Shape as Atom>::REPRS, ReprRange::NONZERO, "its vtable never null");
        assert_eq!(<*mut [u64] as Atom>::REPRS, ReprRange::FULL, "a slice's, any repr");
        assert_eq!(low_bit_widths::<NonNull<[u64]>>(), (0, 3), "a slice's elements' alignment");
        assert_eq!(low_bit_widths::<NonNull<str>>(), (0, 0), "a `str`'s bytes'");
        assert_eq!(low_bit_widths::<NonNull<dyn Shape>>(), (0, 0), "and none known of a trait's");
    }

    /// Checks every law for `value`, whose pointers `read` reads through, and the zero repr's.
    fn laws<T, R, F>(value: T, read: F)
    where
        T: Atom + PartialEq + Debug,
        T::Repr: PartialEq + Debug,
        T::Validity: Promise,
        R: PartialEq + Debug,
        F: Fn(T) -> R,
    {
        assert_holds(round_trips(value));
        assert_holds(keeps_provenance(value, read));
        assert_holds(decodes_as_promised::<T::Validity, T>(value.to_repr()));
        assert_holds(decodes_as_promised::<T::Validity, T>(<T::Repr as Primitive>::from_bits(0)));
    }

    #[test]
    fn pairs_and_wide_pointers_obey_the_laws() {
        let (a, b) = (node(12), node(13));
        laws((a, b), |(first, second)| (value(first), value(second)));
        laws((a.as_ptr(), Some(b)), |(first, second)| {
            (value(NonNull::new(first).expect("the first is `a`")), second.map(value))
        });
        let bytes = NonNull::from(Box::leak(vec![1_u8, 2].into_boxed_slice()));
        // SAFETY: `bytes` is live, and nothing writes it.
        laws(bytes, |seen| unsafe { seen.as_ref() }.to_vec());
        // SAFETY: as above.
        laws(bytes.as_ptr(), |seen| unsafe { (*seen).to_vec() });
        let text = NonNull::from(Box::leak(String::from("word").into_boxed_str()));
        // SAFETY: `text` is live, and nothing writes it.
        laws(text.as_ptr().cast_const(), |seen| unsafe { &*seen }.to_owned());
        let square = shape(Square(4));
        laws(square, area);
        laws(Some(square), |seen| seen.map(area));
        // SAFETY: `square` is live, and nothing writes it.
        let raw_area = |seen: *mut dyn Shape| unsafe { &*seen }.area();
        laws(square.as_ptr(), raw_area);
        laws(Some(square.as_ptr()), |seen| seen.map(raw_area));
        let forged = <<*mut dyn Shape as Atom>::Repr as Primitive>::from_bits(u128::MAX);
        assert_eq!(<*mut dyn Shape as Atom>::from_repr(forged), None, "bits make no vtable");
        let bits = square.to_repr().packed_bits();
        let moved = square.to_repr().with_packed_bits(bits ^ 8 << 64);
        assert_eq!(<NonNull<dyn Shape> as Atom>::from_repr(moved), None, "nor a vtable moved");
        free(a);
        free(b);
        free(bytes);
        free(text);
        free(square);
    }

    #[test]
    #[should_panic(expected = "`double_words::tests::Marked`: the pointer's address has a bit set")]
    fn a_mark_over_a_pointer_with_its_bit_set_is_refused() {
        let pointer = ptr::without_provenance_mut::<Node>(0x1001);
        let first = NonNull::new(pointer).expect("an address not null");
        let _ = Marked { first, second: first, deleted: true }.to_repr();
    }
}
