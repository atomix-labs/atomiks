//! Without LSE2, a 128-bit load is a compare-exchange, which writes its line: a field of a 128-bit
//! value reads with `load_rmw`, by name.

#![feature(const_trait_impl)]

#[path = "../../testing/packed.rs"]
mod packed;

use atomix_core::Atomic;
use atomix_core::ordering::Acquire;

packed::packed! {
    /// A value, and the sequence number it was written at.
    struct Versioned in u128, projected as VersionedFields { 0 => value: u64, 1 => seq: u64 }
}

fn main() {
    let versioned = Atomic::new(Versioned { value: 1, seq: 1 });
    let _ = versioned.fields().seq.load_rmw(Acquire);
    let _ = versioned.fields().seq.load(Acquire);
}
