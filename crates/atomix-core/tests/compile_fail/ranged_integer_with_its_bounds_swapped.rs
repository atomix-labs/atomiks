//! A ranged integer whose `MIN` is above `MAX` would hold no integer, so rustc refuses its pattern
//! type wherever the type is used.

use atomix_core::RangedU64;

fn main() {
    let _ = RangedU64::<5, 3>::new(4);
}
