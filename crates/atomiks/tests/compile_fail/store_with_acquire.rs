//! A store reads nothing, so an `Acquire` one would promise an order it cannot give, and
//! `core` panics on one.

use atomiks::AtomicU64;
use atomiks::ordering::Acquire;

fn main() {
    AtomicU64::new(0).store(1, Acquire);
}
