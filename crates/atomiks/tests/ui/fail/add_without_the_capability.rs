use atomiks::AtomicBool;
use atomiks::ordering::Relaxed;

fn main() {
    let _ = AtomicBool::new(false).fetch_add(true, Relaxed);
}
