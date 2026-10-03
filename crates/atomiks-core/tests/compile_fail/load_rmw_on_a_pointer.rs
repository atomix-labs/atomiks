//! `load_rmw` writes zero over zero, which over a pointer at address zero would drop the
//! pointer's provenance; a pointer has a pure-read `load` instead.

use core::ptr;

use atomiks_core::AtomicPtr;
use atomiks_core::ordering::Acquire;

fn main() {
    let _ = AtomicPtr::<u64>::new(ptr::null_mut()).load_rmw(Acquire);
}
