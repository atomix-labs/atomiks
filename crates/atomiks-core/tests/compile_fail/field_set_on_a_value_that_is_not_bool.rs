//! `set` turns on the field's lowest bit, which is the field's whole value only where the field is a
//! `bool`: on a `u32` quantity it would make the quantity odd.

#![feature(const_trait_impl)]

#[path = "../testing/packed.rs"]
mod packed;

use atomiks_core::Atomic;
use atomiks_core::ordering::Relaxed;

packed::packed! {
    /// A quantity, and whether it is live.
    struct Quote in u64, projected as QuoteFields { 0 => quantity: u32, 1 => live: bool }
}

fn main() {
    let quote = Atomic::new(Quote { quantity: 1, live: false });
    quote.fields().live.set(Relaxed);
    quote.fields().quantity.set(Relaxed);
}
