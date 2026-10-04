//! A ranged integer over every `u8`, 0 to 255, fills every repr, so `None` would share one with a
//! value.

use atomiks_core::{Atomic, RangedU8};

static FULL: Atomic<Option<RangedU8<0, 255>>> = Atomic::new(None);

fn main() {
    let _ = &FULL;
}
