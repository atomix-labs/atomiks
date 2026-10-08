//! Most `u32`s are no `char`, so its atomic's cell is opaque, and reads from no bytes.

use atomix_core::Atomic;
use zerocopy::FromBytes;

fn main() {
    let _ = Atomic::<char>::read_from_bytes(&[0; 4]);
}
