//! A sum of two integers in a range can leave it, while the repr's add would store it: a ranged
//! integer has no `AtomAdd`.

use atomix_core::ordering::Relaxed;
use atomix_core::{Atomic, RangedU8};

fn main() {
    Atomic::new(RangedU8::<1, 10>::MAX).fetch_add(1, Relaxed);
}
