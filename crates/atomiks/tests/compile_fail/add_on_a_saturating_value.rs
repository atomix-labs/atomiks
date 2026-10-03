//! A `Saturating` value's add stops at its bound, but its repr's wraps: an atomic add would wrap.

use core::num::Saturating;

use atomiks::Atomic;
use atomiks::ordering::Relaxed;

fn main() {
    Atomic::new(Saturating(u8::MAX)).add(1, Relaxed);
}
