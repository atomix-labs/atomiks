//! Adding to a top field whose value is never zero could wrap it to zero: a field's add needs
//! `FieldAdd`, which only values whose every pattern decodes have.

#![feature(const_trait_impl)]

#[path = "../testing/packed.rs"]
mod packed;

use core::num::NonZero;

use atomix_core::Atomic;
use atomix_core::ordering::Relaxed;

packed::packed! {
    /// A count, then an owner's id at the top.
    struct Owned in u32, projected as OwnedFields { 0 => count: u16, 1 => owner: NonZero<u16> }
}

fn main() {
    let owner = NonZero::new(7).expect("7 is not zero");
    Atomic::new(Owned { count: 1, owner }).fields().owner.fetch_add(1, Relaxed);
}
