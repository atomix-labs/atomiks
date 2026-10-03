use atomiks::Atomic;
use atomiks::ordering::Relaxed;

fn main() {
    Atomic::new('a').or('b', Relaxed);
}
