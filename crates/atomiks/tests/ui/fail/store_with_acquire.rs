use atomiks::AtomicU64;
use atomiks::ordering::Acquire;

fn main() {
    AtomicU64::new(0).store(1, Acquire);
}
