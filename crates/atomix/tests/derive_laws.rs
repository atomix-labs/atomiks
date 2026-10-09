//! The `Atom` laws over derived values, one of each shape and layout, generic instances and
//! `Option`s of them: a value's repr lies in its range, which may wrap, and decodes back to it; a
//! repr that decodes re-encodes to itself, unchecked too; each validity's promise holds; `None`
//! takes a spare repr: zero where zero is the niche, else one outside its value's range; and a
//! pointer word's or a pointer enum's pointers, decoded, read the nodes they point to, those of
//! each shape held in an atomic inside its own pointee too.
//!
//! The repr laws run on every repr of 16 bits or fewer; on a wider one, on the edges of each width,
//! on random bits, and beside each value's repr. A pointer repr made of bits has no provenance,
//! and is compared by its address; a value's own keeps its pointer's, which Miri checks as the
//! node is read through it.

// The derives on the generic `Wrap`, `Pair`, `Lock`, `LockWord`, `GenericLink`, `GenericPair`,
// `GenericHead` and `GenericChunk` need it.
#![feature(const_trait_impl)]
#![cfg(feature = "derive")]
// Loom's `Atomic::new` is not `const`, so no static of a node that holds one builds under it.
#![cfg(not(loom))]

// atomix-core's, by its path; its `//!` says why.
#[cfg(test)]
#[path = "../../atomix-core/tests/testing/mod.rs"]
mod testing;

#[cfg(test)]
mod tests {
    use core::any::type_name;
    use core::fmt::Debug;
    use core::marker::PhantomData;
    use core::num::NonZero;
    use core::ptr::NonNull;

    use atomix::validity::{Partial, Total, TotalZeroNiche, ZeroNiche, ZeroValid};
    use atomix::{Atom, Atomic, ExactBits, Primitive, RangedI8, RangedU8, RangedU64, ReprRange};
    use proptest::prelude::{Just, Strategy, any, prop_oneof};
    use proptest::sample::select;
    use proptest::test_runner::TestCaseError;
    use proptest::{option, prop_assert_eq, proptest};

    use crate::testing::atom::{repr_and_validity_are, with_every_byte};
    use crate::testing::law::{
        Promise, assert_holds, decodes_as_promised, edge_or_random_bits, keeps_provenance,
        none_laws, round_trips,
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

    /// A quote of some venue `V`, which a marker alone names: laid out as `Quote` is, in constants,
    /// with no repr stated and no gate.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct VenueQuote<V> {
        /// How many.
        quantity: u32,
        /// Which side.
        side: Side,
        /// Whether it may fill.
        live: bool,
        /// The venue.
        venue: PhantomData<fn() -> V>,
    }

    /// Two halves of a word on some venue `V`, which a marker alone names: every pattern of their
    /// bits decodes and they fill the repr, so laid out once, it is `Total`, as one laid out per
    /// instance could not promise.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct VenueHalves<V> {
        /// The low half.
        low: u16,
        /// The high half.
        high: u16,
        /// The venue.
        venue: PhantomData<fn() -> V>,
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

    /// A node a pointer shape points to, aligned to 16, so its pointer leaves four low bits clear.
    #[derive(Debug)]
    #[repr(align(16))]
    struct Node {
        /// What it holds.
        value: u64,
    }

    /// The nodes the pointer shapes point to, which live as long as the tests.
    static NODES: [Node; 4] =
        [Node { value: 10 }, Node { value: 11 }, Node { value: 12 }, Node { value: 13 }];

    /// A Treiber stack's head: the top node or none, a sign and a mark, in three of the four bits
    /// a node's alignment leaves clear.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Head {
        /// The top node.
        top: Option<NonNull<Node>>,
        /// Which way the price last moved.
        sign: Sign,
        /// Whether a pop is under way.
        marked: bool,
    }

    /// A link never null, and its mark.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Link {
        /// The next node.
        next: NonNull<Node>,
        /// Whether the node that holds the link is deleted.
        marked: bool,
    }

    /// A link and a lock above its mark: a word over a word.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Guarded {
        /// The link.
        #[atom(ptr)]
        link: Link,
        /// Whether the link is locked.
        locked: bool,
    }

    /// The next node, or the end, which takes null: one unit fills the niche of a pointer never
    /// null.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Next {
        /// The end.
        End,
        /// A node.
        Node(NonNull<Node>),
    }

    /// A bucket of a table, tagged: a node beside tag fields, the index and generation of the next
    /// vacant bucket above the four clear bits, or closed.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Bucket {
        /// Holds a node.
        Occupied {
            /// The node.
            node: NonNull<Node>,
            /// Whether it is deleted.
            marked: bool,
            /// The side it rests on.
            side: Side,
        },
        /// Free, beside the next free one.
        Vacant {
            /// The next free bucket's index.
            next: u32,
            /// How many times it was freed.
            generation: u16,
        },
        /// Closed.
        Closed,
    }

    /// A pointer of any kind and a flag.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Flagged<P> {
        /// The pointer.
        #[atom(ptr)]
        pointer: P,
        /// The flag.
        flag: bool,
    }

    /// One of two pointers of any kind, by one tag bit.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Either<A, B> {
        /// The left pointer.
        Left(#[atom(ptr)] A),
        /// The right pointer.
        Right(#[atom(ptr)] B),
    }

    /// A node of a list, a stack, a chain and a table at once, which holds an atomic link of each
    /// shape to a node like it, as a lock-free structure's nodes hold theirs: no shape's layout
    /// reads the node's alignment, which the node's own layout, holding the links, decides.
    #[expect(dead_code, reason = "the links are there for their types; the laws read the value")]
    struct Linked {
        /// What it holds.
        value: u64,
        /// The next node in a list, and whether this one is deleted.
        next: Atomic<ListLink>,
        /// The head of a stack below it.
        below: Atomic<StackHead>,
        /// The next node in a chain, or its end.
        chain: Atomic<ChainLink>,
        /// A table's slot.
        slot: Atomic<TableSlot>,
        /// A locked link to a node, or none.
        guarded: Atomic<Option<GuardedLink>>,
    }

    /// A Harris list's link: the next node or none, and whether the node that holds it is deleted.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct ListLink {
        /// The next node.
        next: Option<NonNull<Linked>>,
        /// Whether the node that holds this link is deleted.
        deleted: bool,
    }

    /// A Treiber stack's head, which its nodes hold too: the top node or none, and a version.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct StackHead {
        /// The top node.
        top: Option<NonNull<Linked>>,
        /// How many times the head changed, wrapping.
        version: RangedU8<0, 3>,
    }

    /// The next node of a chain, or its end, which fills the pointer's niche.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum ChainLink {
        /// The end.
        End,
        /// The next node.
        Node(NonNull<Linked>),
    }

    /// A table's slot, tagged: empty, a value inline above the node's alignment, or a node.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum TableSlot {
        /// Nothing.
        Empty,
        /// A value, held inline.
        Inline(u32),
        /// A node.
        Node(NonNull<Linked>),
    }

    /// A link to a node never null, and its mark.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct MarkedLink {
        /// The next node.
        next: NonNull<Linked>,
        /// Whether the node that holds this link is deleted.
        deleted: bool,
    }

    /// A marked link and a lock above its mark: a word over a word.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct GuardedLink {
        /// The marked link.
        #[atom(ptr)]
        link: MarkedLink,
        /// Whether the link is locked.
        locked: bool,
    }

    /// A list's node of any value, which holds the link to the next.
    #[expect(dead_code, reason = "the link is there for its type; the laws read the value")]
    struct GenericNode<T> {
        /// What it holds.
        value: T,
        /// The next node, and whether this one is deleted.
        next: Atomic<GenericLink<T>>,
    }

    /// The link a node of any value holds: an instance of a generic word, in its own pointee.
    #[derive(Debug, PartialEq, Eq, Atom)]
    struct GenericLink<T> {
        /// The next node.
        next: Option<NonNull<GenericNode<T>>>,
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

    /// A node holding `value`, each of its links to no node.
    const fn linked(value: u64) -> Linked {
        let version = RangedU8::new(0).expect("0 lies in 0 to 3");
        Linked {
            value,
            next: Atomic::new(ListLink { next: None, deleted: false }),
            below: Atomic::new(StackHead { top: None, version }),
            chain: Atomic::new(ChainLink::End),
            slot: Atomic::new(TableSlot::Empty),
            guarded: Atomic::new(None),
        }
    }

    /// A node of the generic list, holding `value`, linked to no node.
    const fn generic_node(value: u64) -> GenericNode<u64> {
        GenericNode { value, next: Atomic::new(GenericLink { next: None, deleted: false }) }
    }

    /// The nodes the shapes in their own pointee point to, which live as long as the tests.
    static LINKED: [Linked; 4] = [linked(20), linked(21), linked(22), linked(23)];

    /// The generic nodes a generic link points to, which live as long as the tests.
    static GENERIC_NODES: [GenericNode<u64>; 4] =
        [generic_node(30), generic_node(31), generic_node(32), generic_node(33)];

    /// A pointer to each node, with the static's provenance.
    fn nodes() -> impl Strategy<Value = NonNull<Node>> {
        (0..NODES.len()).prop_map(|index| NonNull::from_ref(&NODES[index]))
    }

    /// The value of the node `node` points to, read through it: Miri refuses the read where the
    /// pointer lost its provenance.
    fn read(node: NonNull<Node>) -> u64 {
        // SAFETY: each node is a static's, which lives as long as the tests.
        #[expect(unsafe_code, reason = "reads through the pointer, so Miri checks its provenance")]
        let node = unsafe { node.as_ref() };
        node.value
    }

    /// A pointer to each node a link of each shape holds, with the static's provenance.
    fn linked_nodes() -> impl Strategy<Value = NonNull<Linked>> {
        (0..LINKED.len()).prop_map(|index| NonNull::from_ref(&LINKED[index]))
    }

    /// A pointer to each generic node, with the static's provenance.
    fn generic_nodes() -> impl Strategy<Value = NonNull<GenericNode<u64>>> {
        (0..GENERIC_NODES.len()).prop_map(|index| NonNull::from_ref(&GENERIC_NODES[index]))
    }

    /// The value of the node `node` points to, read through it as `read` reads a node.
    fn read_linked(node: NonNull<Linked>) -> u64 {
        // SAFETY: each node is a static's, which lives as long as the tests.
        #[expect(unsafe_code, reason = "reads through the pointer, so Miri checks its provenance")]
        let node = unsafe { node.as_ref() };
        node.value
    }

    /// The value of the generic node `node` points to, read through it as `read` reads a node.
    fn read_generic(node: NonNull<GenericNode<u64>>) -> u64 {
        // SAFETY: each node is a static's, which lives as long as the tests.
        #[expect(unsafe_code, reason = "reads through the pointer, so Miri checks its provenance")]
        let node = unsafe { node.as_ref() };
        node.value
    }

    /// List links to every node or none, marked or not.
    fn list_links() -> impl Strategy<Value = ListLink> {
        (option::of(linked_nodes()), any::<bool>())
            .prop_map(|(next, deleted)| ListLink { next, deleted })
    }

    /// Stack heads of every node or none, and every version.
    fn stack_heads() -> impl Strategy<Value = StackHead> {
        let versions = (0_u8..=3)
            .prop_map(|count| RangedU8::new(count).expect("a count of 0 to 3 lies in 0 to 3"));
        (option::of(linked_nodes()), versions).prop_map(|(top, version)| StackHead { top, version })
    }

    /// Each kind of chain link.
    fn chain_links() -> impl Strategy<Value = ChainLink> {
        prop_oneof![Just(ChainLink::End), linked_nodes().prop_map(ChainLink::Node)]
    }

    /// Each kind of table slot, of every value and node.
    fn table_slots() -> impl Strategy<Value = TableSlot> {
        prop_oneof![
            Just(TableSlot::Empty),
            any::<u32>().prop_map(TableSlot::Inline),
            linked_nodes().prop_map(TableSlot::Node),
        ]
    }

    /// Locked links of every node, marked or not, locked or not.
    fn guarded_links_to_linked() -> impl Strategy<Value = GuardedLink> {
        (linked_nodes(), any::<bool>(), any::<bool>()).prop_map(|(next, deleted, locked)| {
            GuardedLink { link: MarkedLink { next, deleted }, locked }
        })
    }

    /// Generic links of every generic node or none, marked or not.
    fn generic_links() -> impl Strategy<Value = GenericLink<u64>> {
        (option::of(generic_nodes()), any::<bool>())
            .prop_map(|(next, deleted)| GenericLink { next, deleted })
    }

    /// What the node a locked link points to holds, read through its pointer.
    fn read_guarded(guarded: GuardedLink) -> u64 {
        read_linked(guarded.link.next)
    }

    /// What the node a table slot holds reads, read through its pointer.
    fn read_slot(slot: TableSlot) -> Option<u64> {
        match slot {
            TableSlot::Node(node) => Some(read_linked(node)),
            TableSlot::Empty | TableSlot::Inline(_) => None,
        }
    }

    /// Links to every node, marked or not.
    fn links() -> impl Strategy<Value = Link> {
        (nodes(), any::<bool>()).prop_map(|(next, marked)| Link { next, marked })
    }

    /// Guarded links of every link, locked or not.
    fn guarded_links() -> impl Strategy<Value = Guarded> {
        (links(), any::<bool>()).prop_map(|(link, locked)| Guarded { link, locked })
    }

    /// Each kind of bucket: of every node and tag, every index, and closed.
    fn buckets() -> impl Strategy<Value = Bucket> {
        prop_oneof![
            (nodes(), any::<bool>(), sides()).prop_map(|(node, marked, side)| Bucket::Occupied {
                node,
                marked,
                side
            }),
            (any::<u32>(), any::<u16>())
                .prop_map(|(next, generation)| Bucket::Vacant { next, generation }),
            Just(Bucket::Closed),
        ]
    }

    /// Either pointer, to every node.
    fn eithers() -> impl Strategy<Value = Either<NonNull<Node>, *mut Node>> {
        prop_oneof![
            nodes().prop_map(Either::Left),
            nodes().prop_map(|node| Either::Right(node.as_ptr())),
        ]
    }

    /// What the node an occupied bucket holds reads, read through its pointer.
    fn read_bucket(bucket: Bucket) -> Option<u64> {
        match bucket {
            Bucket::Occupied { node, .. } => Some(read(node)),
            Bucket::Vacant { .. } | Bucket::Closed => None,
        }
    }

    /// What the node either pointer points to holds, read through it.
    fn read_either(either: Either<NonNull<Node>, *mut Node>) -> u64 {
        match either {
            Either::Left(node) => read(node),
            Either::Right(node) => read(NonNull::new(node).expect("each right node is a static's")),
        }
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
        T::Repr: PartialEq + Debug,
    {
        decodes_as_promised::<_, T>(repr)?;
        T::from_repr(repr).map_or(Ok(()), round_trips)
    }

    /// The laws for `value`, and for its repr's neighbours and the low bits of `bits` as reprs: a
    /// pointer's neighbours keep its provenance, and its bits have none.
    fn value_obeys_the_laws<T>(value: T, bits: u128) -> Result<(), TestCaseError>
    where
        T: Atom + PartialEq + Debug,
        T::Validity: Promise,
        T::Repr: PartialEq + Debug,
    {
        round_trips(value)?;
        let repr = value.to_repr();
        let address = repr.packed_bits();
        [
            repr.with_packed_bits(address.wrapping_sub(1)),
            repr.with_packed_bits(address.wrapping_add(1)),
            Primitive::from_bits(bits),
        ]
        .into_iter()
        .try_for_each(repr_obeys_the_laws::<T>)
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
        repr_and_validity_are::<ListLink, *mut Linked, Total>();
        repr_and_validity_are::<StackHead, *mut Linked, Partial>();
        repr_and_validity_are::<ChainLink, *mut (), ZeroValid>();
        repr_and_validity_are::<TableSlot, *mut (), ZeroValid>();
        repr_and_validity_are::<GuardedLink, *mut Linked, ZeroNiche>();
        repr_and_validity_are::<GenericLink<u64>, *mut GenericNode<u64>, ZeroValid>();
        repr_and_validity_are::<VenueHalves<u8>, u32, Total>();
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
        none_laws!(VenueQuote<u8>, VenueQuote<()>);
        none_laws!(OwnerId, PriceMove, Lock<OwnerId>);
        none_laws!(NonZeroByte, Third, Turn, Entry, Slot, Reading, Fill, Offer, Shift, Signal);
        none_laws!(Wrap<Sign>, Pair<u32, bool>, Pair<NonZero<u8>, Sign>, Lock<u8>, Lock<Sign>);
        none_laws!(LockWord<Sign>, LockWord<NonZero<u8>>);
        none_laws!(Wrap<Swing>, Lock<Swing>, LockWord<Swing>);
        none_laws!(Option<Sign>, Option<Step>, Option<Reading>, Option<Order>, Option<Slot>);
        // A nullable head's and a niche's null decodes, so neither has an `Option`.
        none_laws!(Link, Guarded, Bucket, Flagged<NonNull<Node>>, Either<NonNull<Node>, *mut Node>);
        none_laws!(MarkedLink, GuardedLink);
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
            let Quote { quantity, side, live } = quote;
            let venue_quote = VenueQuote::<u8> { quantity, side, live, venue: PhantomData };
            value_obeys_the_laws(venue_quote, bits)?;
            prop_assert_eq!(
                venue_quote.to_repr(),
                quote.to_repr(),
                "a marker's parameter moves no field"
            );
            value_obeys_the_laws(halves, bits)?;
            let Halves { low, high } = halves;
            value_obeys_the_laws(VenueHalves::<u8> { low, high, venue: PhantomData }, bits)?;
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
        fn pointer_words_obey_the_laws(
            head in (option::of(nodes()), signs(), any::<bool>())
                .prop_map(|(top, sign, marked)| Head { top, sign, marked }),
            link in links(),
            guarded in guarded_links(),
            flagged in (nodes(), any::<bool>()).prop_map(|(pointer, flag)| Flagged { pointer, flag }),
            bits in edge_or_random_bits(&[0xF, 0x10]),
        ) {
            value_obeys_the_laws(head, bits)?;
            keeps_provenance(head, |head| head.top.map(read))?;
            value_obeys_the_laws(link, bits)?;
            keeps_provenance(link, |link| read(link.next))?;
            value_obeys_the_laws(guarded, bits)?;
            keeps_provenance(guarded, |guarded| read(guarded.link.next))?;
            value_obeys_the_laws(flagged, bits)?;
            keeps_provenance(flagged, |flagged| read(flagged.pointer))?;
        }

        #[test]
        fn pointer_enums_obey_the_laws(
            next in prop_oneof![Just(Next::End), nodes().prop_map(Next::Node)],
            bucket in buckets(),
            either in eithers(),
            bits in edge_or_random_bits(&[0xF, 0x10]),
        ) {
            value_obeys_the_laws(next, bits)?;
            keeps_provenance(next, |next| match next {
                Next::End => None,
                Next::Node(node) => Some(read(node)),
            })?;
            value_obeys_the_laws(bucket, bits)?;
            keeps_provenance(bucket, read_bucket)?;
            value_obeys_the_laws(either, bits)?;
            keeps_provenance(either, read_either)?;
        }

        #[test]
        fn pointer_shapes_in_their_own_pointee_obey_the_laws(
            list in list_links(),
            stack in stack_heads(),
            chain in chain_links(),
            slot in table_slots(),
            guarded in option::of(guarded_links_to_linked()),
            generic in generic_links(),
            bits in edge_or_random_bits(&[0x7, 0x8]),
        ) {
            value_obeys_the_laws(list, bits)?;
            keeps_provenance(list, |list| list.next.map(read_linked))?;
            value_obeys_the_laws(stack, bits)?;
            keeps_provenance(stack, |stack| stack.top.map(read_linked))?;
            value_obeys_the_laws(chain, bits)?;
            keeps_provenance(chain, |chain| match chain {
                ChainLink::End => None,
                ChainLink::Node(node) => Some(read_linked(node)),
            })?;
            value_obeys_the_laws(slot, bits)?;
            keeps_provenance(slot, read_slot)?;
            value_obeys_the_laws(guarded, bits)?;
            keeps_provenance(guarded, |guarded| guarded.map(read_guarded))?;
            value_obeys_the_laws(generic, bits)?;
            keeps_provenance(generic, |generic| generic.next.map(read_generic))?;
        }

        #[test]
        fn options_of_pointer_shapes_obey_the_laws(
            link in option::of(links()),
            guarded in option::of(guarded_links()),
            bucket in option::of(buckets()),
            either in option::of(eithers()),
            bits in edge_or_random_bits(&[0xF, 0x10]),
        ) {
            value_obeys_the_laws(link, bits)?;
            keeps_provenance(link, |link| link.map(|link| read(link.next)))?;
            value_obeys_the_laws(guarded, bits)?;
            keeps_provenance(guarded, |guarded| guarded.map(|guarded| read(guarded.link.next)))?;
            value_obeys_the_laws(bucket, bits)?;
            keeps_provenance(bucket, |bucket| bucket.and_then(read_bucket))?;
            value_obeys_the_laws(either, bits)?;
            keeps_provenance(either, |either| either.map(read_either))?;
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

    /// Derived values two words wide, which hold pointers, beside nodes of their own: a pointer
    /// beside tags no alignment holds, or beside the integer word `repr = u128` states, two
    /// pointers, a slice's pointer, and each shape in its own pointee.
    #[cfg(any(
        target_arch = "aarch64",
        target_arch = "arm64ec",
        all(target_arch = "x86_64", target_feature = "cmpxchg16b")
    ))]
    mod double_words {
        use core::ptr::NonNull;

        use atomix::validity::{Partial, Total, ZeroNiche, ZeroValid};
        use atomix::{Atom, Atomic, DoubleWord};
        use proptest::prelude::{Strategy, any};
        use proptest::{option, proptest};

        use super::{NODES, Node, Sign, read, signs, value_obeys_the_laws};
        use crate::testing::atom::repr_and_validity_are;
        use crate::testing::law::{edge_or_random_bits, keeps_provenance, none_laws};

        /// A Treiber stack's head: the top node or none, and a counter no alignment holds.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
        struct CountedHead {
            /// The top node.
            top: Option<NonNull<Node>>,
            /// How many times the head changed, wrapping.
            version: u64,
        }

        /// A link and a sign beside it, in the integer word `repr = u128` states: two bits that
        /// wrap through zero, so the word above them copies their sign.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
        #[atom(repr = u128)]
        struct StatedLink {
            /// The next node, or none.
            next: Option<NonNull<Node>>,
            /// Which way the link last moved.
            sign: Sign,
        }

        /// Two pointers, and a mark in the first one's low bits.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
        struct MarkedPair {
            /// A node.
            first: NonNull<Node>,
            /// Its partner, or none.
            second: Option<NonNull<Node>>,
            /// Whether the pair is logically deleted.
            marked: bool,
        }

        /// A slice of words, and a seal in its data pointer's low bits.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
        struct SealedChunk {
            /// The words.
            words: NonNull<[u64]>,
            /// Whether the chunk is sealed.
            sealed: bool,
        }

        /// Two pointers, and no tag.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
        struct BarePair {
            /// A node.
            first: NonNull<Node>,
            /// Its partner, or none.
            second: Option<NonNull<Node>>,
        }

        /// A newtype of a slice's pointer, which takes its repr whole.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
        struct Words(NonNull<[u64]>);

        /// Two pointers of any two types, and a mark in the first one's low bits.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
        struct GenericPair<A, B> {
            /// The first.
            first: NonNull<A>,
            /// The second, or none.
            second: Option<NonNull<B>>,
            /// The mark.
            marked: bool,
        }

        /// A pointer to any `T` and a count, in the integer word `repr = u128` states.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
        #[atom(repr = u128)]
        struct GenericHead<T> {
            /// The top, or none.
            top: Option<NonNull<T>>,
            /// The count.
            count: u16,
        }

        /// A slice of any element, and a seal in its data pointer's low bits.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
        struct GenericChunk<T> {
            /// The elements.
            elements: NonNull<[T]>,
            /// Whether the chunk is sealed.
            sealed: bool,
        }

        /// A node that holds an atomic of each two-word shape that points to a node like it.
        #[expect(
            dead_code,
            reason = "the links are there for their types; the laws read the value"
        )]
        struct Linked {
            /// What it holds.
            value: u64,
            /// The next node, and a counter.
            counted: Atomic<CountedLink>,
            /// Two nodes, or none.
            pair: Atomic<Option<LinkedPair>>,
        }

        /// A link to the next node, beside a counter no alignment holds.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
        struct CountedLink {
            /// The next node, or none.
            next: Option<NonNull<Linked>>,
            /// How many times the link changed.
            version: u64,
        }

        /// Two links to nodes, and a mark.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
        struct LinkedPair {
            /// A node.
            first: NonNull<Linked>,
            /// Another.
            second: NonNull<Linked>,
            /// Whether the pair is marked.
            marked: bool,
        }

        /// A node holding `value`, each of its links to no node.
        const fn linked(value: u64) -> Linked {
            Linked {
                value,
                counted: Atomic::new(CountedLink { next: None, version: 0 }),
                pair: Atomic::new(None),
            }
        }

        /// The nodes the two-word shapes in their own pointee point to.
        static LINKED: [Linked; 3] = [linked(40), linked(41), linked(42)];

        /// The words the slices and the generic shapes point into.
        static WORDS: [u64; 4] = [50, 51, 52, 53];

        /// A pointer to each node, with the static's provenance.
        fn nodes() -> impl Strategy<Value = NonNull<Node>> {
            (0..NODES.len()).prop_map(|index| NonNull::from_ref(&NODES[index]))
        }

        /// A pointer to each linked node, with the static's provenance.
        fn linked_nodes() -> impl Strategy<Value = NonNull<Linked>> {
            (0..LINKED.len()).prop_map(|index| NonNull::from_ref(&LINKED[index]))
        }

        /// A pointer to each word, with the static's provenance.
        fn words() -> impl Strategy<Value = NonNull<u64>> {
            (0..WORDS.len()).prop_map(|index| NonNull::from_ref(&WORDS[index]))
        }

        /// The word `word` points to, read through it.
        fn read_word(word: NonNull<u64>) -> u64 {
            // SAFETY: each word is a static's, which lives as long as the tests.
            #[expect(unsafe_code, reason = "reads through it, so Miri checks its provenance")]
            let word = unsafe { word.read() };
            word
        }

        /// The value of the linked node `node` points to, read through it.
        fn read_linked(node: NonNull<Linked>) -> u64 {
            // SAFETY: each node is a static's, which lives as long as the tests.
            #[expect(unsafe_code, reason = "reads through it, so Miri checks its provenance")]
            let node = unsafe { node.as_ref() };
            node.value
        }

        /// The words a chunk's slice holds, read through it.
        fn read_words(words: NonNull<[u64]>) -> Vec<u64> {
            // SAFETY: each slice is of a static's, which lives as long as the tests.
            #[expect(unsafe_code, reason = "reads through it, so Miri checks its provenance")]
            let words = unsafe { words.as_ref() };
            words.to_vec()
        }

        #[test]
        fn each_two_word_shape_has_the_validity_its_fields_promise() {
            repr_and_validity_are::<CountedHead, DoubleWord<*mut Node, usize>, Total>();
            repr_and_validity_are::<StatedLink, DoubleWord<*mut Node, usize>, ZeroValid>();
            repr_and_validity_are::<MarkedPair, DoubleWord<*mut Node, *mut Node>, ZeroNiche>();
            repr_and_validity_are::<SealedChunk, DoubleWord<*mut (), usize>, ZeroNiche>();
            repr_and_validity_are::<CountedLink, DoubleWord<*mut Linked, usize>, Total>();
            repr_and_validity_are::<LinkedPair, DoubleWord<*mut Linked, *mut Linked>, ZeroNiche>();
            repr_and_validity_are::<BarePair, DoubleWord<*mut Node, *mut Node>, ZeroNiche>();
            repr_and_validity_are::<Words, DoubleWord<*mut (), usize>, ZeroNiche>();
            repr_and_validity_are::<GenericPair<u64, u64>, DoubleWord<*mut u64, *mut u64>, Partial>(
            );
            repr_and_validity_are::<GenericHead<u64>, DoubleWord<*mut u64, usize>, ZeroValid>();
            repr_and_validity_are::<GenericChunk<u64>, DoubleWord<*mut (), usize>, Partial>();
            let down = StatedLink { next: None, sign: Sign::Minus }.to_repr();
            assert_eq!(down.second, usize::MAX, "a sign below zero, extended over the word");
            let unextended = DoubleWord { first: down.first, second: 0b11 };
            assert_eq!(StatedLink::from_repr(unextended), None, "its bits alone, refused");
            none_laws!(
                MarkedPair,
                SealedChunk,
                LinkedPair,
                BarePair,
                Words,
                GenericPair<u64, u64>,
                GenericChunk<u64>,
            );
        }

        proptest! {
            #[test]
            fn two_word_shapes_obey_the_laws(
                head in (option::of(nodes()), any::<u64>())
                    .prop_map(|(top, version)| CountedHead { top, version }),
                stated in (option::of(nodes()), signs())
                    .prop_map(|(next, sign)| StatedLink { next, sign }),
                pair in (nodes(), option::of(nodes()), any::<bool>())
                    .prop_map(|(first, second, marked)| MarkedPair { first, second, marked }),
                chunk in (0..=WORDS.len(), any::<bool>()).prop_map(|(length, sealed)| {
                    SealedChunk { words: NonNull::from_ref(&WORDS[..length]), sealed }
                }),
                counted in (option::of(linked_nodes()), any::<u64>())
                    .prop_map(|(next, version)| CountedLink { next, version }),
                linked in option::of((linked_nodes(), linked_nodes(), any::<bool>())
                    .prop_map(|(first, second, marked)| LinkedPair { first, second, marked })),
                bare in (nodes(), option::of(nodes()))
                    .prop_map(|(first, second)| BarePair { first, second }),
                words in (0..=WORDS.len()).prop_map(|length| Words(NonNull::from_ref(&WORDS[..length]))),
                generic_pair in (words(), option::of(words()), any::<bool>())
                    .prop_map(|(first, second, marked)| GenericPair { first, second, marked }),
                generic_head in (option::of(words()), any::<u16>())
                    .prop_map(|(top, count)| GenericHead { top, count }),
                generic_chunk in (0..=WORDS.len(), any::<bool>()).prop_map(|(length, sealed)| {
                    GenericChunk { elements: NonNull::from_ref(&WORDS[..length]), sealed }
                }),
                bits in edge_or_random_bits(&[1 << 64, 0xFF << 64, 0b11 << 64, u128::MAX << 64]),
            ) {
                value_obeys_the_laws(head, bits)?;
                keeps_provenance(head, |head| head.top.map(read))?;
                value_obeys_the_laws(stated, bits)?;
                keeps_provenance(stated, |stated| stated.next.map(read))?;
                value_obeys_the_laws(pair, bits)?;
                keeps_provenance(pair, |pair| (read(pair.first), pair.second.map(read)))?;
                value_obeys_the_laws(chunk, bits)?;
                keeps_provenance(chunk, |chunk| read_words(chunk.words))?;
                value_obeys_the_laws(counted, bits)?;
                keeps_provenance(counted, |counted| counted.next.map(read_linked))?;
                value_obeys_the_laws(linked, bits)?;
                keeps_provenance(linked, |linked| {
                    linked.map(|linked| (read_linked(linked.first), read_linked(linked.second)))
                })?;
                value_obeys_the_laws(bare, bits)?;
                keeps_provenance(bare, |bare| (read(bare.first), bare.second.map(read)))?;
                value_obeys_the_laws(words, bits)?;
                keeps_provenance(words, |words| read_words(words.0))?;
                value_obeys_the_laws(generic_pair, bits)?;
                keeps_provenance(generic_pair, |pair| (read_word(pair.first), pair.second.map(read_word)))?;
                value_obeys_the_laws(generic_head, bits)?;
                keeps_provenance(generic_head, |head| head.top.map(read_word))?;
                value_obeys_the_laws(generic_chunk, bits)?;
                keeps_provenance(generic_chunk, |chunk| read_words(chunk.elements))?;
            }
        }
    }
}
