//! A signed ranged integer whose `MIN` is above `MAX` would hold no integer. rustc compares the
//! bounds as signed, so it refuses `5..=-5`, though its bits, `0x05` to `0xFB`, ascend as unsigned.

use atomix_core::RangedI8;

fn main() {
    let _ = RangedI8::<5, -5>::new(0);
}
