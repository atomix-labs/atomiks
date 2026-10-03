use atomiks::AtomicU128;
use atomiks::ordering::{Acquire, Release};

fn main() {
    let wide = AtomicU128::new(0);
    wide.store(1, Release);
    let _ = wide.load(Acquire);
}
