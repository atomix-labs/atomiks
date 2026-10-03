use atomiks::Atomic;

fn main() {
    let _ = Atomic::<Option<u64>>::from(None);
}
