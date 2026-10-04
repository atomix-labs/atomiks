//! `Atom` derives for every shape but an enum with fields so far; the stub written in place of its
//! impl raises no error of its own.

use atomiks::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
enum Slot {
    Empty,
    Full(u32),
}

static SLOT: Atomic<Slot> = Atomic::new(Slot::Empty);

fn main() {
    let _ = (&SLOT, Slot::Full(1));
}
