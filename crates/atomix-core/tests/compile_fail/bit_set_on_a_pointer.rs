//! A pointer's bits are its address, and setting one by its position would move the pointer off
//! its pointee: `bit_set` needs `AtomBitwise`, which a pointer does not have.

use core::ptr;

use atomix_core::Atomic;
use atomix_core::ordering::AcqRel;

fn main() {
    let node = Atomic::new(ptr::null_mut::<u64>());
    let _ = node.bit_set(0, AcqRel);
}
