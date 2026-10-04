//! `Atom` derives for a newtype, a zero-width struct and a fieldless enum so far; the stub written
//! in place of each other impl raises no error of its own.

use atomiks::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
struct Quote {
    qty: u32,
    live: bool,
}

#[derive(Clone, Copy, Atom)]
enum Slot {
    Empty,
    Full(u32),
}

static SLOT: Atomic<Slot> = Atomic::new(Slot::Empty);

fn main() {
    let _ = (&SLOT, Quote { qty: 1, live: true }, Slot::Full(1));
}
