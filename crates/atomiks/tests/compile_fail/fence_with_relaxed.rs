//! A `Relaxed` fence orders nothing, and `core` panics on one: here it does not compile.

use atomiks::fence;
use atomiks::ordering::Relaxed;

fn main() {
    fence(Relaxed);
}
