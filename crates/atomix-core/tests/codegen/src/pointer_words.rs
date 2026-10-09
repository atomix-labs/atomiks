//! One pointer word operation per function, over words `tests/testing/pointer_word.rs` writes out
//! as the derive writes them: a load and its pointer, an `Option`'s load, a store, a word's over a
//! word, an exchange, an update and a Treiber stack's pop, and each tag operation, a bit's at bit
//! 0, 1 and 2, a counter's load and update, a field of flags' bitwise operations, and the load of a
//! word through the place of its pointer, itself a word.

use core::ptr::NonNull;

use arbitrary_int::u4;
use atomix_core::ordering::{AcqRel, Acquire, Release};
use atomix_core::{Atomic, RangedU8};

use crate::pointer_word::pointer_word;

/// A node, aligned to 8, so a pointer to one leaves three low bits clear.
#[repr(align(8))]
pub struct Node {
    /// What it holds.
    pub value: u64,
    /// The node below it on a stack.
    pub next: Option<NonNull<Node>>,
}

/// A block, aligned to 64, so a pointer to one leaves six low bits clear.
#[repr(align(64))]
pub struct Block {
    /// What it holds.
    pub value: u64,
}

pointer_word! {
    /// A Treiber stack's head: the top node, a counter of 0 to 3 and a mark, in three bits.
    pub struct Head, projected as HeadFields {
        0 => top: Option<NonNull<Node>>,
        1 => version: RangedU8<0, 3>,
        2 => marked: bool,
    }
}

pointer_word! {
    /// A link never null, and its mark in bit 0.
    pub struct Link, projected as LinkFields {
        0 => next: NonNull<Node>,
        1 => deleted: bool,
    }
}

pointer_word! {
    /// A bit at bit 0, 1 and 2 of a node's pointer.
    pub struct Bits, projected as BitsFields {
        0 => node: NonNull<Node>,
        1 => low: bool,
        2 => middle: bool,
        3 => high: bool,
    }
}

pointer_word! {
    /// A bit, then a field of flags in bits 1 to 4, below a block's pointer.
    pub struct Flagged, projected as FlaggedFields {
        0 => block: NonNull<Block>,
        1 => ready: bool,
        2 => flags: u4,
    }
}

pointer_word! {
    /// A link, its mark at bit 0, and a lock at bit 1, above it.
    pub struct Guarded, projected as GuardedFields {
        0 => link: Link,
        1 => locked: bool,
    }
}

#[unsafe(no_mangle)]
pub fn word_load(atomic: &Atomic<Head>) -> Head {
    atomic.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn word_load_top(atomic: &Atomic<Head>) -> Option<NonNull<Node>> {
    atomic.load(Acquire).top
}

#[unsafe(no_mangle)]
pub fn option_word_load(atomic: &Atomic<Option<Link>>) -> Option<Link> {
    atomic.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn tag_set(atomic: &Atomic<Link>) {
    atomic.fields().deleted.set(Release);
}

#[unsafe(no_mangle)]
pub fn tag_clear(atomic: &Atomic<Link>) {
    atomic.fields().deleted.clear(Release);
}

#[unsafe(no_mangle)]
pub fn tag_toggle(atomic: &Atomic<Link>) {
    atomic.fields().deleted.toggle(Release);
}

#[unsafe(no_mangle)]
pub fn tag_load(atomic: &Atomic<Head>) -> RangedU8<0, 3> {
    atomic.fields().version.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn tag_update(atomic: &Atomic<Head>) -> Head {
    atomic
        .fields()
        .version
        .update(AcqRel, Acquire, |version| RangedU8::new_saturating(version.get().wrapping_add(1)))
}

#[unsafe(no_mangle)]
pub fn bit0_test_and_set(atomic: &Atomic<Bits>) -> bool {
    atomic.fields().low.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn bit0_test_and_set_in_some(atomic: &Atomic<Bits>) -> Option<bool> {
    Some(atomic.fields().low.test_and_set(AcqRel))
}

#[unsafe(no_mangle)]
pub fn bit0_test_and_clear(atomic: &Atomic<Bits>) -> bool {
    atomic.fields().low.test_and_clear(AcqRel)
}

#[unsafe(no_mangle)]
pub fn bit0_test_and_toggle(atomic: &Atomic<Bits>) -> bool {
    atomic.fields().low.test_and_toggle(AcqRel)
}

#[unsafe(no_mangle)]
pub fn bit1_test_and_set(atomic: &Atomic<Bits>) -> bool {
    atomic.fields().middle.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn bit1_test_and_clear(atomic: &Atomic<Bits>) -> bool {
    atomic.fields().middle.test_and_clear(AcqRel)
}

#[unsafe(no_mangle)]
pub fn bit1_test_and_toggle(atomic: &Atomic<Bits>) -> bool {
    atomic.fields().middle.test_and_toggle(AcqRel)
}

#[unsafe(no_mangle)]
pub fn bit2_test_and_set(atomic: &Atomic<Bits>) -> bool {
    atomic.fields().high.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn bit2_test_and_clear(atomic: &Atomic<Bits>) -> bool {
    atomic.fields().high.test_and_clear(AcqRel)
}

#[unsafe(no_mangle)]
pub fn bit2_test_and_toggle(atomic: &Atomic<Bits>) -> bool {
    atomic.fields().high.test_and_toggle(AcqRel)
}

#[unsafe(no_mangle)]
pub fn tag_flags_or(atomic: &Atomic<Flagged>, flags: u4) {
    atomic.fields().flags.or(flags, Release);
}

#[unsafe(no_mangle)]
pub fn tag_flags_and(atomic: &Atomic<Flagged>, flags: u4) {
    atomic.fields().flags.and(flags, Release);
}

#[unsafe(no_mangle)]
pub fn tag_flags_xor(atomic: &Atomic<Flagged>, flags: u4) {
    atomic.fields().flags.xor(flags, Release);
}

#[unsafe(no_mangle)]
pub fn tag_flags_not(atomic: &Atomic<Flagged>) {
    atomic.fields().flags.not(Release);
}

#[unsafe(no_mangle)]
pub fn nested_tag_set(atomic: &Atomic<Guarded>) {
    atomic.fields().link.fields().deleted.set(Release);
}

#[unsafe(no_mangle)]
pub fn pointer_place_load(atomic: &Atomic<Guarded>) -> Link {
    atomic.fields().link.load(Acquire)
}

#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn tag_fetch_or(atomic: &Atomic<Link>) -> Link {
    atomic.fields().deleted.fetch_or(true, AcqRel)
}

#[unsafe(no_mangle)]
pub fn word_store(atomic: &Atomic<Link>, next: NonNull<Node>) {
    atomic.store(Link { next, deleted: true }, Release);
}

#[unsafe(no_mangle)]
pub fn nested_store(atomic: &Atomic<Guarded>, next: NonNull<Node>) {
    atomic.store(Guarded { link: Link { next, deleted: true }, locked: true }, Release);
}

#[unsafe(no_mangle)]
pub fn word_compare_exchange(atomic: &Atomic<Head>, current: Head, new: Head) -> bool {
    atomic.compare_exchange(current, new, AcqRel, Acquire).is_ok()
}

#[unsafe(no_mangle)]
pub fn nested_compare_exchange(atomic: &Atomic<Guarded>, current: Guarded, new: Guarded) -> bool {
    atomic.compare_exchange(current, new, AcqRel, Acquire).is_ok()
}

#[unsafe(no_mangle)]
pub fn word_update(atomic: &Atomic<Head>) -> Head {
    atomic.update(AcqRel, Acquire, |head| Head { marked: !head.marked, ..head })
}

/// Pops a Treiber stack's top node: its `next`, read through the pointer the head decodes to,
/// becomes the head, a version on.
///
/// # Safety
/// No node the stack has held is freed or written while this pop runs.
#[unsafe(no_mangle)]
pub unsafe fn word_pop(stack: &Atomic<Head>) -> Option<NonNull<Node>> {
    // ORDERING: Acquire, pairing with the Release exchange of the push that linked the top, so its
    // `next` reads as that push wrote it.
    let mut head = stack.load(Acquire);
    loop {
        let top = head.top?;
        // SAFETY: the stack held `top` when `head` was read, and the caller frees and writes no
        // node the stack has held while this runs, so `top` names a whole `Node` even once another
        // pop has taken it.
        let next = unsafe { top.as_ref() }.next;
        let version = RangedU8::new_saturating(head.version.get().wrapping_add(1) & 3);
        let popped = Head { top: next, version, marked: head.marked };
        // ORDERING: AcqRel on success, as a push's exchange; Acquire on failure, as the load above,
        // since the head found is read through on the next turn.
        match stack.compare_exchange_weak(head, popped, AcqRel, Acquire) {
            Ok(_) => return Some(top),
            Err(found) => head = found,
        }
    }
}
