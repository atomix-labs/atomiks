//! Nor does a `char`'s atomic build from zeros through zerocopy, though zero is `'\0'`: its opaque
//! cell is no `FromZeros`. bytemuck's `Zeroable` builds it.

use atomiks_core::Atomic;
use zerocopy::FromZeros;

fn main() {
    let _ = Atomic::<char>::new_zeroed();
}
