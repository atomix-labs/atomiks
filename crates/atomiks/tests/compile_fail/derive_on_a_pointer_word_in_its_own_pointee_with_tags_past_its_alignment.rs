//! A word held in its own pointee is laid out by its pointer's tag width alone, then checked
//! against the pointee's alignment: a node of one pointer, aligned to 8, leaves three low bits, too
//! few for a byte of tags, and the build is refused at the word, not by a cycle.

use core::ptr::NonNull;

use atomiks::{Atom, Atomic};

struct Node {
    next: Atomic<Link>,
}

#[derive(Clone, Copy, Atom)]
struct Link {
    next: Option<NonNull<Node>>,
    tag: u8,
}

fn main() {}
