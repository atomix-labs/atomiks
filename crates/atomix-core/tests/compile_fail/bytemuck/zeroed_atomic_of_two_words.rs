//! Two words' cell holds their pointers' addresses, not the words, so their atomic has no
//! `Zeroable`, though zero is null pointers: bytemuck's traits promise memory that is the value.

use core::ptr::NonNull;

use atomix_core::Atomic;
use bytemuck::Zeroable;

fn main() {
    let _ = Atomic::<Option<NonNull<[u8]>>>::zeroed();
    let _ = Atomic::<*mut [u8]>::zeroed();
}
