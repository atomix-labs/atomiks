//! `StoreStore` orders a fence and nothing else, so a swap cannot take it.

use atomiks_core::AtomicU64;
use atomiks_core::ordering::StoreStore;

fn main() {
    let _ = AtomicU64::new(0).swap(1, StoreStore);
}
