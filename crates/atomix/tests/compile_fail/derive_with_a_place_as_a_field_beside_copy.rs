//! A derived atom whose field is a place, deriving `Copy` beside `Atom`, is refused once by the
//! derive and by std's `Copy` and `Clone`, and its uses as an atom raise no error of their own.

use atomix::{Atom, Atomic, AtomicU64};

#[derive(Clone, Copy, Atom)]
struct Pair {
    seq: AtomicU64,
    live: bool,
}

static PAIR: Atomic<Pair> = Atomic::new(Pair { seq: AtomicU64::new(0), live: false });

fn main() {}
