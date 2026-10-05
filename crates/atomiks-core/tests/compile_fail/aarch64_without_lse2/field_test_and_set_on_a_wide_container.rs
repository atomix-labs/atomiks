//! A 128-bit word has no bit test-and-set that is one instruction: a `bool` field of one changes
//! with `update`.

#![feature(const_trait_impl)]

#[path = "../../testing/packed.rs"]
mod packed;

use atomiks_core::Atomic;
use atomiks_core::ordering::{AcqRel, Acquire};

packed::packed! {
    /// A value, and whether it is written.
    struct Slot in u128, projected as SlotFields { 0 => value: u64, 1 => written: bool }
}

fn main() {
    let slot = Atomic::new(Slot { value: 1, written: false });
    slot.fields().written.update(AcqRel, Acquire, |_| true);
    let _ = slot.fields().written.test_and_set(AcqRel);
}
