use atomiks::AtomicU64;
use atomiks::ordering::Release;

fn main() {
    let _ = AtomicU64::new(0).load(Release);
}
