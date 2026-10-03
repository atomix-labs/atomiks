//! A `u64` fills every repr of its primitive, so `None` would share one with a value.

use atomiks_core::Atomic;

static FULL: Atomic<Option<u64>> = Atomic::new(None);

fn main() {
    let _ = &FULL;
}
