//! Zero is no `NonZero`, so its atomic has no `Zeroable`; its `Option`'s, whose `None` takes zero,
//! has.

use core::num::NonZero;

use atomiks_core::Atomic;
use bytemuck::Zeroable;

fn main() {
    let _ = Atomic::<NonZero<u64>>::zeroed();
}
