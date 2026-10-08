//! A `Saturating` value's add stops at its bound, but its repr's wraps: an atomic add would wrap.

use core::num::Saturating;

use atomix_core::Atomic;
use atomix_core::ordering::Relaxed;

fn main() {
    Atomic::new(Saturating(u8::MAX)).fetch_add(1, Relaxed);
}
