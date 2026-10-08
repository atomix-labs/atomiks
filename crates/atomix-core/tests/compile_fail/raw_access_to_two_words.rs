//! Two words' cell holds their pointers' addresses, their provenance exposed: a pointer read
//! through its place would have none, and one written there unexposed none for a load to take
//! back, so an atomic of two words lends no place, neither its address nor `&mut`.

use core::ptr::{self, NonNull};
use core::slice;

use atomix_core::Atomic;

/// Two nodes of a list.
type Pair = (NonNull<u64>, NonNull<u64>);

fn main() {
    let mut node = 7_u64;
    let node = NonNull::from_mut(&mut node);
    let mut pair: Atomic<Pair> = Atomic::new((node, node));
    let _ = pair.as_ptr();
    let _ = pair.get_mut();
    let _ = Atomic::get_mut_slice(slice::from_mut(&mut pair));
    // SAFETY: never runs; the build fails above.
    let _ = unsafe { Atomic::<NonNull<[u8]>>::from_ptr(ptr::null_mut()) };
}
