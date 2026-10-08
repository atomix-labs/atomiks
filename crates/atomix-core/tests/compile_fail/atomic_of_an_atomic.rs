//! An atomic is a place, not a value: one inside another is refused with a message of its own,
//! not `Atom`'s advice to derive it.

use atomix_core::{Atomic, AtomicU64};

/// The atomic as a type, inside an atomic.
fn read(_: &Atomic<AtomicU64>) {}

/// And inside an `Option` inside an atomic.
static SLOT: Atomic<Option<AtomicU64>> = Atomic::new(None);

fn main() {}
