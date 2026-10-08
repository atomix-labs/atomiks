//! A 128-bit word has no and, or or xor that is one instruction: a field of one has `load`,
//! `update` and `try_update` alone.

#![feature(const_trait_impl)]

#[path = "../testing/packed.rs"]
mod packed;

use atomix_core::Atomic;
use atomix_core::ordering::{AcqRel, Relaxed};

packed::packed! {
    /// A sequence number, and the value written at it.
    struct Versioned in u128, projected as VersionedFields { 0 => value: u64, 1 => seq: u64 }
}

fn main() {
    let versioned = Atomic::new(Versioned { value: 1, seq: 1 });
    versioned.fields().seq.update(AcqRel, Relaxed, |seq| seq + 1);
    versioned.fields().value.or(2, Relaxed);
}
