//! A load publishes nothing, so a `Release` one would promise an order it cannot give, and
//! `core` panics on one.

use atomiks_core::AtomicU64;
use atomiks_core::ordering::Release;

fn main() {
    let _ = AtomicU64::new(0).load(Release);
}
