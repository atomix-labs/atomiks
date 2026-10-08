//! Without LSE2, a 128-bit load or store is a compare-exchange: the load would write its line, and
//! the store would loop.

use atomix_core::AtomicU128;
use atomix_core::ordering::{Acquire, Release};

fn main() {
    let wide = AtomicU128::new(0);
    wide.store(1, Release);
    let _ = wide.load(Acquire);
}
