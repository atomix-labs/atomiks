//! Each instance of an enum with fields and parameters lays its fields out by them, so no constant
//! knows its width to select a repr from: the enum states one, which each instance is checked
//! against. The stub written in place of its impl raises no error of its own.

use atomix::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
enum Lock<O> {
    Free,
    Owned(O),
}

static LOCK: Atomic<Lock<u8>> = Atomic::new(Lock::Free);

fn main() {
    let _ = (&LOCK, Lock::Owned(1_u8));
}
