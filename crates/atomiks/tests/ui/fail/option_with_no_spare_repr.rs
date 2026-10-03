use atomiks::Atomic;

static FULL: Atomic<Option<u64>> = Atomic::new(None);

fn main() {
    let _ = &FULL;
}
