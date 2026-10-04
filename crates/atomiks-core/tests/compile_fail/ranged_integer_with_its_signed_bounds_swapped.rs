//! A signed ranged integer whose `MIN` is above `MAX` would hold no integer, though a range through
//! zero, `-5..=5`, wraps in its bits, so rustc compares the bounds as signed and refuses it too.

use atomiks_core::RangedI8;

fn main() {
    let _ = RangedI8::<5, -5>::new(0);
}
