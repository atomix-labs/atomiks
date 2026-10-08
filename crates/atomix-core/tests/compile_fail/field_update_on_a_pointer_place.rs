//! A word's pointer field is no bits of the word: its place loads the pointer through the word, and
//! an update, which would write a pointer's bits without its provenance, is refused.

#![feature(const_trait_impl)]

#[path = "../testing/pointer_word.rs"]
mod pointer_word;

use core::ptr::NonNull;

use atomix_core::Atomic;
use atomix_core::ordering::{AcqRel, Acquire};

/// A node, aligned to 8.
#[repr(align(8))]
struct Node(u64);

pointer_word::pointer_word! {
    /// A link, never null, and its mark.
    struct Link, projected as LinkFields { 0 => next: NonNull<Node>, 1 => deleted: bool }
}

fn main() {
    let link = Atomic::new(Link { next: NonNull::dangling(), deleted: false });
    let _ = link.fields().next.load(Acquire);
    link.fields().next.update(AcqRel, Acquire, |next| next);
}
