//! A `Relaxed` fence orders nothing, and `core` panics on one: here it does not compile.

use atomix_core::fence;
use atomix_core::ordering::Relaxed;

fn main() {
    fence(Relaxed);
}
