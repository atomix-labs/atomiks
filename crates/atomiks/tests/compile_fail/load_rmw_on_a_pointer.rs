//! `load_rmw` writes zero over zero, which over a pointer at address zero would drop the
//! pointer's provenance; a pointer has a pure-read `load` instead.

use core::ptr;

use atomiks::AtomicPtr;
use atomiks::ordering::Acquire;

fn main() {
    let _ = AtomicPtr::<u64>::new(ptr::null_mut()).load_rmw(Acquire);
}
