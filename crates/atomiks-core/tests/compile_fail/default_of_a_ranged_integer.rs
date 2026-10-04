//! Zero may lie outside a ranged integer's range, so it has no `Default`.

use atomiks_core::RangedU64;

fn main() {
    let _ = RangedU64::<3>::default();
}
