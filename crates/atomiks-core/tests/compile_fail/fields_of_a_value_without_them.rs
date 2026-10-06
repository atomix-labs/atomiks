//! An integer has no fields to project onto: `fields` needs a packed struct, whose derive writes
//! its projection.

use atomiks_core::Atomic;

fn main() {
    let _ = Atomic::new(5_u32).fields();
}
