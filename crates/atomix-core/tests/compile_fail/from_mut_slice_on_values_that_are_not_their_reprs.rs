//! `from_mut_slice` takes each value's place as an atomic's cell, so each value must be its own
//! repr: a slice of `()` has no bytes for its cells' `u8`s.

use atomix_core::Atomic;

fn main() {
    let _ = Atomic::from_mut_slice(&mut [(); 2]);
}
