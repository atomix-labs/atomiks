//! A failed compare-exchange is a load, so a `Release` failure ordering would promise an order it
//! cannot give, and `core` panics on one.

use atomiks_core::AtomicU64;
use atomiks_core::ordering::{AcqRel, Release};

fn main() {
    let _ = AtomicU64::new(0).compare_exchange(0, 1, AcqRel, Release);
}
