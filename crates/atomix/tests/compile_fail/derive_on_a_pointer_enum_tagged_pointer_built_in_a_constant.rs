//! No constant reads the address of a pointer with provenance, so none can check that a tagged
//! variant's pointer leaves its tag's bits clear: the static is refused. A static of a niche's
//! pointer, which no tag shares, builds.

use core::ptr::NonNull;

use atomix::{Atom, Atomic};

#[repr(align(8))]
struct Leaf(u64);

#[repr(align(8))]
struct Branch(u64);

#[derive(Clone, Copy, Atom)]
enum Child {
    Leaf(NonNull<Leaf>),
    Branch(NonNull<Branch>),
}

#[derive(Clone, Copy, Atom)]
enum Next {
    End,
    Node(NonNull<Leaf>),
}

static LEAF: Leaf = Leaf(1);

static FIRST: Atomic<Next> = Atomic::new(Next::Node(NonNull::from_ref(&LEAF)));

static ROOT: Atomic<Child> = Atomic::new(Child::Leaf(NonNull::from_ref(&LEAF)));

fn main() {
    let _ = (&FIRST, &ROOT, Child::Branch(NonNull::from_ref(&Branch(2))), Next::End);
}
