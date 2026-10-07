//! A pointer repr marks `None` with null alone: an enum whose first variant is a unit holds null,
//! so its `Option` is refused where it is built.

use core::ptr::NonNull;

use atomiks::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
enum Slot {
    Empty,
    Inline(u32),
    Node(NonNull<u64>),
}

static SLOT: Atomic<Option<Slot>> = Atomic::new(None);

fn main() {
    let _ = &SLOT;
}
