//! A store reads nothing, so an `Acquire` one would promise an order it cannot give, and
//! `core` panics on one.

use atomiks_core::AtomicU64;
use atomiks_core::ordering::Acquire;

fn main() {
    AtomicU64::new(0).store(1, Acquire);
}
