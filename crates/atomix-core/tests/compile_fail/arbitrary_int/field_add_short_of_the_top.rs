//! A last field that stops short of the repr's top bit would carry into the clear bits above it:
//! a `u61` after a `bool` ends at bit 62 of 64, and only a `u63`, which fills the word, adds in
//! place.

#![feature(const_trait_impl)]

#[path = "../../testing/packed.rs"]
mod packed;

use arbitrary_int::{u61, u63};
use atomix_core::Atomic;
use atomix_core::ordering::Relaxed;

packed::packed! {
    /// Whether it is live, then a count that fills the word.
    struct Filled in u64, projected as FilledFields { 0 => live: bool, 1 => count: u63 }
}

packed::packed! {
    /// Whether it is live, then a count two bits short of the top.
    struct Short in u64, projected as ShortFields { 0 => live: bool, 1 => count: u61 }
}

fn main() {
    Atomic::new(Filled { live: true, count: u63::new(1) }).fields().count.fetch_add(1, Relaxed);
    Atomic::new(Short { live: true, count: u61::new(1) }).fields().count.fetch_add(1, Relaxed);
}
