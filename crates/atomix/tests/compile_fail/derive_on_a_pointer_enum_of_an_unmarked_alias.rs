//! The derive reads a pointer from its type's syntax: an alias of one in a variant
//! shows none, so the enum is one with fields, whose bits hold no provenance, and is
//! refused; the note names `#[atom(ptr)]`.

use core::ptr::NonNull;

use atomix::Atom;

type NodePointer = NonNull<u64>;

#[derive(Clone, Copy, Atom)]
enum Next {
    End,
    Node(NodePointer),
}

fn main() {}
