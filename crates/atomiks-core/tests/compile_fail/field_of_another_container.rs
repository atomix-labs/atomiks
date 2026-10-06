//! A path into one packed value names bits that mean something else in another: generic code over
//! a field of a `Quote` takes no field of a `Head`.

#![feature(const_trait_impl)]

#[path = "../testing/packed.rs"]
mod packed;

use atomiks_core::ordering::Relaxed;
use atomiks_core::{Atomic, AtomicField, FieldPath};

packed::packed! {
    /// A quantity, and whether it is live.
    struct Quote in u64, projected as QuoteFields { 0 => quantity: u32, 1 => live: bool }
}

packed::packed! {
    /// A slot, and whether it is written.
    struct Head in u32, projected as HeadFields { 0 => slot: u16, 1 => written: bool }
}

/// Whether the quote is live, wherever its bit lies.
fn is_live<P: FieldPath<Container = Quote, Value = bool>>(live: &AtomicField<P>) -> bool {
    live.load(Relaxed)
}

fn main() {
    let quote = Atomic::new(Quote { quantity: 1, live: true });
    let head = Atomic::new(Head { slot: 1, written: false });
    let _ = is_live(quote.fields().live);
    let _ = is_live(head.fields().written);
}
