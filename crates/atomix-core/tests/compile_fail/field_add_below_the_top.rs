//! A carry out of a field below the top would reach the field above it: a field adds in place only
//! where it ends at the repr's top bit, its `TopField`.

#![feature(const_trait_impl)]

#[path = "../testing/packed.rs"]
mod packed;

use atomix_core::Atomic;
use atomix_core::ordering::Relaxed;

packed::packed! {
    /// A count, then a count above it at the top.
    struct Counts in u64, projected as CountsFields { 0 => low: u32, 1 => high: u32 }
}

fn main() {
    let counts = Atomic::new(Counts { low: 1, high: 1 });
    counts.fields().high.fetch_add(1, Relaxed);
    counts.fields().low.fetch_add(1, Relaxed);
}
