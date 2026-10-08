//! No constant reads the address of a pointer with provenance, so none can check that a pointer to
//! a static leaves its tags' bits clear: a static of a word of one is refused, at the static.

#![feature(const_trait_impl)]

#[path = "../testing/pointer_word.rs"]
mod pointer_word;

use core::ptr::NonNull;

use atomix_core::Atomic;

/// A node, aligned to 8.
#[repr(align(8))]
struct Node(u64);

pointer_word::pointer_word! {
    /// A link, never null, and its mark.
    struct Link, projected as LinkFields { 0 => next: NonNull<Node>, 1 => deleted: bool }
}

static SENTINEL: Node = Node(0);

static FIRST: Atomic<Link> =
    Atomic::new(Link { next: NonNull::from_ref(&SENTINEL), deleted: false });

fn main() {
    let _ = &FIRST;
}
