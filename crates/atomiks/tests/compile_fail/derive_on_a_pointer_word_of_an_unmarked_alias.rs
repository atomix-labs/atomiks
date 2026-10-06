//! The derive reads a pointer from its type's syntax: an alias of one shows none, so a struct of it
//! is packed as bits, which hold no provenance, and refused; the note names `#[atom(ptr)]`.

use core::ptr::NonNull;

use atomiks::Atom;

type NodePointer = NonNull<u64>;

#[derive(Clone, Copy, Atom)]
struct Link {
    next: NodePointer,
    deleted: bool,
}

fn main() {}
