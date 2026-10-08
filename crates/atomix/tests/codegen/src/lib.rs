//! One operation on a derived value per function, unmangled, for `tests/compiled.rs` to read the
//! assembly of: the load of a packed struct, a fieldless enum and a niche-filling enum, each field
//! operation through a packed struct's projection, a pointer word's tag operations, the load of a
//! word through its pointer's place, a word's store and exchange, the store of a word over a word
//! or a pointer enum, the load and match of each pointer enum and the store of each of its kinds of
//! variant, an update of a pointer enum that keeps its pointer, and of a word over one; and, where
//! the target has a 16-byte atomic, a struct of two words' load, exchange and counter's update,
//! and a pair of pointers' store and exchange. The `x86-64-refused` feature adds each shape of two
//! words, for the test that `x86_64` without `cmpxchg16b` refuses each once.
//!
//! The empty `[workspace]` in its manifest makes it a workspace of its own: the repository's does
//! not list it, and the test builds it alone.

#![no_std]

#[cfg(feature = "x86-64-refused")]
pub mod without_cmpxchg16b;

use core::ptr::NonNull;

use atomix::ordering::{AcqRel, Acquire, Release};
use atomix::{Atom, AtomBitwise, Atomic, RangedU8, RangedU32};

/// The side of the book an order rests on: one bit, its discriminant.
#[derive(Clone, Copy, Atom)]
pub enum Side {
    /// A buy.
    Bid,
    /// A sell.
    Ask,
}

/// Which way a price moved: two bits, signed.
#[derive(Clone, Copy, Atom)]
#[repr(i8)]
pub enum Sign {
    /// Down.
    Minus = -1,
    /// Neither.
    Flat,
    /// Up.
    Plus,
}

/// A resting quote: 32 bits of quantity, then a bit of side, then one of whether it is live.
#[derive(Clone, Copy, Atom)]
pub struct Quote {
    /// How many.
    pub quantity: u32,
    /// Which side.
    pub side: Side,
    /// Whether it may fill.
    pub live: bool,
}

/// A reading of a sign, missing or stale: the two units take the reprs beside the sign's range.
#[derive(Clone, Copy, Atom)]
pub enum Reading {
    /// No reading yet.
    Missing,
    /// A reading too old to use.
    Stale,
    /// The sign read.
    Present(Sign),
}

/// Flags of an order, each pattern of a byte a value.
#[derive(Clone, Copy, Atom, AtomBitwise)]
pub struct Flags(pub u8);

/// An order: 32 bits of quantity, a bit of side, a bit of whether it is live (bit 33), then flags.
#[derive(Clone, Copy, Atom)]
pub struct Order {
    /// How many.
    pub quantity: u32,
    /// Which side.
    pub side: Side,
    /// Whether it may fill.
    pub live: bool,
    /// Its flags, at bits 34 to 41.
    pub flags: Flags,
}

/// A count in the top half of the word, flags and a bit below it.
#[derive(Clone, Copy, Atom)]
pub struct Counted {
    /// Its flags.
    pub flags: Flags,
    /// Whether it is ready.
    pub ready: bool,
    /// Bits no one uses.
    pub spare: RangedU32<0, 0x7F_FFFF>,
    /// The count, at bits 32 to 63.
    pub count: u32,
}

/// A packed struct a field of another holds.
#[derive(Clone, Copy, Atom)]
pub struct Inner {
    /// Whether it is ready, at bit 8 of `Outer`.
    pub ready: bool,
    /// Its flags.
    pub flags: Flags,
}

/// A byte, the inner struct at bit 8, then a bit.
#[derive(Clone, Copy, Atom)]
pub struct Outer {
    /// A byte.
    pub low: u8,
    /// The inner struct.
    pub inner: Inner,
    /// Whether it is done.
    pub done: bool,
}

#[unsafe(no_mangle)]
pub fn packed_struct_load(atomic: &Atomic<Quote>) -> Quote {
    atomic.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn fieldless_enum_load(atomic: &Atomic<Side>) -> Side {
    atomic.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn niche_filling_enum_load(atomic: &Atomic<Reading>) -> Reading {
    atomic.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn field_set(atomic: &Atomic<Order>) {
    atomic.fields().live.set(Release);
}

#[unsafe(no_mangle)]
pub fn field_clear(atomic: &Atomic<Order>) {
    atomic.fields().live.clear(Release);
}

#[unsafe(no_mangle)]
pub fn field_test_and_set(atomic: &Atomic<Order>) -> bool {
    atomic.fields().live.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn field_load(atomic: &Atomic<Order>) -> bool {
    atomic.fields().live.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn flags_or(atomic: &Atomic<Order>, flags: Flags) {
    atomic.fields().flags.or(flags, Release);
}

#[unsafe(no_mangle)]
pub fn nested_clear(atomic: &Atomic<Outer>) {
    atomic.fields().inner.fields().ready.clear(Release);
}

#[unsafe(no_mangle)]
pub fn top_fetch_add(atomic: &Atomic<Counted>, delta: u32) -> u32 {
    atomic.fields().count.fetch_add(delta, AcqRel).count
}

/// A node, aligned to 8: three low bits clear.
#[repr(align(8))]
pub struct Node {
    /// What it holds.
    pub value: u64,
}

/// A branch, aligned to 16.
#[repr(align(16))]
pub struct Branch {
    /// What its left side holds.
    pub left: u64,
    /// What its right side holds.
    pub right: u64,
}

/// A Treiber stack's head: the top node or none, a version and a mark, at bit 2.
#[derive(Clone, Copy, Atom)]
pub struct Head {
    /// The top node.
    pub top: Option<NonNull<Node>>,
    /// The version, which tells a top popped and pushed again from the one read.
    pub version: RangedU8<0, 3>,
    /// Whether a pop is under way.
    pub marked: bool,
}

/// The next node, or the end: null, with no tag.
#[derive(Clone, Copy, Atom)]
pub enum Next {
    /// The end.
    End,
    /// A node.
    Node(NonNull<Node>),
}

/// A node or a branch, by one tag bit.
#[derive(Clone, Copy, Atom)]
pub enum Child {
    /// A node, tag 0.
    Leaf(NonNull<Node>),
    /// A branch, tag 1.
    Branch(NonNull<Branch>),
}

/// Empty, a value held inline above the alignment, or a node: two tag bits.
#[derive(Clone, Copy, Atom)]
pub enum Slot {
    /// Tag 0, null.
    Empty,
    /// Tag 1.
    Inline(u32),
    /// Tag 2.
    Node(NonNull<Node>),
}

/// The next vacant index, or a node: one tag bit.
#[derive(Clone, Copy, Atom)]
pub enum Entry {
    /// Tag 0.
    Vacant(u32),
    /// Tag 1.
    Occupied(NonNull<Node>),
}

/// A slot and a lock above its two tag bits.
#[derive(Clone, Copy, Atom)]
pub struct LockedSlot {
    /// The slot.
    #[atom(ptr)]
    pub slot: Slot,
    /// The lock, at bit 2.
    pub locked: bool,
}

/// A child and a lock above its tag bit: a word over an enum of two pointers.
#[derive(Clone, Copy, Atom)]
pub struct LockedChild {
    /// The child.
    #[atom(ptr)]
    pub child: Child,
    /// The lock, at bit 1.
    pub locked: bool,
}

/// A node and a mark at bit 0.
#[derive(Clone, Copy, Atom)]
pub struct Marked {
    /// The node.
    pub node: NonNull<Node>,
    /// The mark.
    pub marked: bool,
}

/// A marked node and a lock at bit 1.
#[derive(Clone, Copy, Atom)]
pub struct Guarded {
    /// The marked node.
    #[atom(ptr)]
    pub inner: Marked,
    /// The lock.
    pub locked: bool,
}

#[unsafe(no_mangle)]
pub fn head_test_and_set_marked(atomic: &Atomic<Head>) -> bool {
    atomic.fields().marked.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn head_load_top(atomic: &Atomic<Head>) -> Option<NonNull<Node>> {
    atomic.fields().top.load(Acquire)
}

/// The value of the node `atomic` holds, or 7 at the end.
///
/// # Safety
/// The node `atomic` holds, if any, is live.
#[unsafe(no_mangle)]
pub unsafe fn next_load_match(atomic: &Atomic<Next>) -> u64 {
    match atomic.load(Acquire) {
        Next::End => 7,
        // SAFETY: the caller's node is live.
        Next::Node(node) => unsafe { node.as_ref().value },
    }
}

/// The value of the leaf, or the right side of the branch, `atomic` holds.
///
/// # Safety
/// The leaf or the branch `atomic` holds is live.
#[unsafe(no_mangle)]
pub unsafe fn child_load_match(atomic: &Atomic<Child>) -> u64 {
    match atomic.load(Acquire) {
        // SAFETY: the caller's leaf is live.
        Child::Leaf(leaf) => unsafe { leaf.as_ref().value },
        // SAFETY: the caller's branch is live.
        Child::Branch(branch) => unsafe { branch.as_ref().right },
    }
}

/// The value of the leaf `atomic` holds, or 0 for a branch.
///
/// # Safety
/// The leaf `atomic` holds, if any, is live.
#[unsafe(no_mangle)]
pub unsafe fn child_load_leaf(atomic: &Atomic<Child>) -> u64 {
    match atomic.load(Acquire) {
        // SAFETY: the caller's leaf is live.
        Child::Leaf(leaf) => unsafe { leaf.as_ref().value },
        Child::Branch(_) => 0,
    }
}

/// The right side of the branch `atomic` holds, or 0 for a leaf.
///
/// # Safety
/// The branch `atomic` holds, if any, is live.
#[unsafe(no_mangle)]
pub unsafe fn child_load_branch(atomic: &Atomic<Child>) -> u64 {
    match atomic.load(Acquire) {
        Child::Leaf(_) => 0,
        // SAFETY: the caller's branch is live.
        Child::Branch(branch) => unsafe { branch.as_ref().right },
    }
}

/// The value `atomic` holds inline or in its node, or 7 where it is empty.
///
/// # Safety
/// The node `atomic` holds, if any, is live.
#[unsafe(no_mangle)]
pub unsafe fn slot_load_match(atomic: &Atomic<Slot>) -> u64 {
    match atomic.load(Acquire) {
        Slot::Empty => 7,
        Slot::Inline(value) => u64::from(value),
        // SAFETY: the caller's node is live.
        Slot::Node(node) => unsafe { node.as_ref().value },
    }
}

/// The next vacant index `atomic` holds, or the value of its node.
///
/// # Safety
/// The node `atomic` holds, if any, is live.
#[unsafe(no_mangle)]
pub unsafe fn entry_load_match(atomic: &Atomic<Entry>) -> u64 {
    match atomic.load(Acquire) {
        Entry::Vacant(next) => u64::from(next),
        // SAFETY: the caller's node is live.
        Entry::Occupied(node) => unsafe { node.as_ref().value },
    }
}

#[unsafe(no_mangle)]
pub fn slot_store_inline(atomic: &Atomic<Slot>, value: u32) {
    atomic.store(Slot::Inline(value), Release);
}

#[unsafe(no_mangle)]
pub fn slot_store_node(atomic: &Atomic<Slot>, node: NonNull<Node>) {
    atomic.store(Slot::Node(node), Release);
}

#[unsafe(no_mangle)]
pub fn next_store_node(atomic: &Atomic<Next>, node: NonNull<Node>) {
    atomic.store(Next::Node(node), Release);
}

#[unsafe(no_mangle)]
pub fn marked_store(atomic: &Atomic<Marked>, node: NonNull<Node>) {
    atomic.store(Marked { node, marked: true }, Release);
}

#[unsafe(no_mangle)]
pub fn guarded_store(atomic: &Atomic<Guarded>, node: NonNull<Node>) {
    atomic.store(Guarded { inner: Marked { node, marked: true }, locked: true }, Release);
}

#[unsafe(no_mangle)]
pub fn locked_slot_store_node(atomic: &Atomic<LockedSlot>, node: NonNull<Node>) {
    atomic.store(LockedSlot { slot: Slot::Node(node), locked: true }, Release);
}

#[unsafe(no_mangle)]
pub fn head_compare_exchange(atomic: &Atomic<Head>, current: Head, new: Head) -> bool {
    atomic.compare_exchange(current, new, AcqRel, Acquire).is_ok()
}

#[unsafe(no_mangle)]
pub fn slot_update(atomic: &Atomic<Slot>) -> Slot {
    atomic.update(AcqRel, Acquire, |slot| match slot {
        Slot::Node(node) => Slot::Node(node),
        Slot::Empty => Slot::Inline(0),
        Slot::Inline(value) => Slot::Inline(value.wrapping_add(1)),
    })
}

#[unsafe(no_mangle)]
pub fn entry_update(atomic: &Atomic<Entry>) -> Entry {
    atomic.update(AcqRel, Acquire, |entry| match entry {
        Entry::Occupied(node) => Entry::Occupied(node),
        Entry::Vacant(next) => Entry::Vacant(next.wrapping_add(1)),
    })
}

#[unsafe(no_mangle)]
pub fn child_update(atomic: &Atomic<Child>) -> Child {
    atomic.update(AcqRel, Acquire, |child| child)
}

#[unsafe(no_mangle)]
pub fn locked_child_update(atomic: &Atomic<LockedChild>) -> LockedChild {
    atomic.update(AcqRel, Acquire, |locked_child| LockedChild {
        locked: !locked_child.locked,
        ..locked_child
    })
}

#[unsafe(no_mangle)]
pub fn locked_slot_set_locked(atomic: &Atomic<LockedSlot>) {
    atomic.fields().locked.set(Release);
}

#[unsafe(no_mangle)]
pub fn locked_slot_test_and_set_locked(atomic: &Atomic<LockedSlot>) -> bool {
    atomic.fields().locked.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn guarded_set_locked(atomic: &Atomic<Guarded>) {
    atomic.fields().locked.set(Release);
}

#[unsafe(no_mangle)]
pub fn guarded_test_and_set_marked(atomic: &Atomic<Guarded>) -> bool {
    atomic.fields().inner.fields().marked.test_and_set(AcqRel)
}

/// The value of the node `atomic`'s inner word holds.
///
/// # Safety
/// The node is live.
#[unsafe(no_mangle)]
pub unsafe fn guarded_load_inner(atomic: &Atomic<Guarded>) -> u64 {
    // SAFETY: the caller's node is live.
    unsafe { atomic.fields().inner.load(Acquire).node.as_ref().value }
}

/// A Treiber stack's head: the top node, or none, beside a counter no alignment holds, in a word
/// of its own.
#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec", target_feature = "cmpxchg16b"))]
#[derive(Clone, Copy, Atom)]
pub struct CountedHead {
    /// The top node.
    pub top: Option<NonNull<Node>>,
    /// How many times the head changed.
    pub version: u64,
}

/// Two nodes, and a mark in the first one's low bits.
#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec", target_feature = "cmpxchg16b"))]
#[derive(Clone, Copy, Atom)]
pub struct MarkedPair {
    /// A node.
    pub first: NonNull<Node>,
    /// Its partner.
    pub second: NonNull<Node>,
    /// Whether the pair is deleted.
    pub marked: bool,
}

#[cfg(any(target_feature = "avx", target_feature = "lse2"))]
#[unsafe(no_mangle)]
pub fn counted_head_load(atomic: &Atomic<CountedHead>) -> CountedHead {
    atomic.load(Acquire)
}

#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec", target_feature = "cmpxchg16b"))]
#[unsafe(no_mangle)]
pub fn counted_head_load_rmw(atomic: &Atomic<CountedHead>) -> CountedHead {
    atomic.load_rmw(Acquire)
}

#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec", target_feature = "cmpxchg16b"))]
#[unsafe(no_mangle)]
pub fn counted_head_compare_exchange(
    atomic: &Atomic<CountedHead>, current: CountedHead, new: CountedHead,
) -> Result<CountedHead, CountedHead> {
    atomic.compare_exchange(current, new, AcqRel, Acquire)
}

#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec", target_feature = "cmpxchg16b"))]
#[unsafe(no_mangle)]
pub fn counted_head_update_version(atomic: &Atomic<CountedHead>) -> CountedHead {
    atomic.fields().version.update(AcqRel, Acquire, |version| version.wrapping_add(1))
}

#[cfg(any(target_feature = "avx", target_feature = "lse2"))]
#[unsafe(no_mangle)]
pub fn marked_pair_store(atomic: &Atomic<MarkedPair>, value: MarkedPair) {
    atomic.store(value, Release);
}

#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec", target_feature = "cmpxchg16b"))]
#[unsafe(no_mangle)]
pub fn marked_pair_compare_exchange(
    atomic: &Atomic<MarkedPair>, current: MarkedPair, new: MarkedPair,
) -> Result<MarkedPair, MarkedPair> {
    atomic.compare_exchange(current, new, AcqRel, Acquire)
}
