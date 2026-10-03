use atomiks::Atomic;

struct Order;

fn main() {
    let _ = Atomic::new(Order);
}
