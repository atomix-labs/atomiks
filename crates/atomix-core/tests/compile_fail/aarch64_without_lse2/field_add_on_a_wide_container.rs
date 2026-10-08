//! A 128-bit word has no add that is one instruction: its top field adds with `update`.

#![feature(const_trait_impl)]

#[path = "../../testing/packed.rs"]
mod packed;

use atomix_core::Atomic;
use atomix_core::ordering::{AcqRel, Acquire};

packed::packed! {
    /// A value, and the sequence number it was written at, in the top half.
    struct Versioned in u128, projected as VersionedFields { 0 => value: u64, 1 => seq: u64 }
}

fn main() {
    let versioned = Atomic::new(Versioned { value: 1, seq: 1 });
    versioned.fields().seq.update(AcqRel, Acquire, |seq| seq + 1);
    versioned.fields().seq.fetch_add(1, AcqRel);
}
