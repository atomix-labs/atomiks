use atomiks::AtomicU128;
use atomiks::ordering::AcqRel;

fn main() {
    let _ = AtomicU128::new(0).swap(1, AcqRel);
}
