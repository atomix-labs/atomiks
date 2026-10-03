use core::ptr;

use atomiks::AtomicPtr;
use atomiks::ordering::Acquire;

fn main() {
    let _ = AtomicPtr::<u64>::new(ptr::null_mut()).load_rmw(Acquire);
}
