//! `Atom` derives only for a newtype so far; the stub written in place of each impl raises no error
//! of its own.

use atomiks::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
struct Marker;

#[derive(Clone, Copy, Atom)]
struct Quote {
    qty: u32,
    live: bool,
}

#[derive(Clone, Copy, Atom)]
enum Side {
    Bid,
    Ask,
}

#[derive(Clone, Copy, Atom)]
enum Slot {
    Empty,
    Full(u32),
}

static SIDE: Atomic<Side> = Atomic::new(Side::Bid);

fn main() {
    let _ = (&SIDE, Marker, Quote { qty: 1, live: true }, Slot::Empty, Slot::Full(1));
}
