//! A `Relaxed` fence orders nothing, and `core` panics on one: here it does not compile.

use atomiks_core::fence;
use atomiks_core::ordering::Relaxed;

fn main() {
    fence(Relaxed);
}
