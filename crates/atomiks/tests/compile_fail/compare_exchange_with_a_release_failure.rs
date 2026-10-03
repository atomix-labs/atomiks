//! A failed compare-exchange is a load, so a `Release` failure ordering would promise an order it
//! cannot give, and `core` panics on one.

use atomiks::AtomicU64;
use atomiks::ordering::{AcqRel, Release};

fn main() {
    let _ = AtomicU64::new(0).compare_exchange(0, 1, AcqRel, Release);
}
