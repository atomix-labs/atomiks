//! A `char`'s atomic reads from no checked bytes either: its opaque cell has no check.

use atomix_core::Atomic;
use zerocopy::TryFromBytes;

fn main() {
    let _ = Atomic::<char>::try_read_from_bytes(&[0; 4]);
}
