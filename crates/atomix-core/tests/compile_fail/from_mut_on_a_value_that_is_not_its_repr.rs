//! `from_mut` takes a value's place as its atomic's cell, so the value must be its own repr: a
//! `()` has no byte for its cell's `u8`.

use atomix_core::Atomic;

fn main() {
    let _ = Atomic::from_mut(&mut ());
}
