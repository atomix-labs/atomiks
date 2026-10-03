use atomiks::fence;
use atomiks::ordering::Relaxed;

fn main() {
    fence(Relaxed);
}
