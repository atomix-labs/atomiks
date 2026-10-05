//! Or-ing into a field whose value has some patterns that are no value, as an owner's id that is
//! never zero, could leave one: a field's bitwise operations need `FieldBitwise`.

#![feature(const_trait_impl)]

#[path = "../testing/packed.rs"]
mod packed;

use core::num::NonZero;

use atomiks_core::Atomic;
use atomiks_core::ordering::Relaxed;

packed::packed! {
    /// A quantity, and its owner's id.
    struct Quote in u64, projected as QuoteFields { 0 => quantity: u32, 1 => owner: NonZero<u8> }
}

fn main() {
    let owner = NonZero::new(7).expect("7 is not zero");
    let quote = Atomic::new(Quote { quantity: 1, owner });
    quote.fields().quantity.or(2, Relaxed);
    quote.fields().owner.or(owner, Relaxed);
}
