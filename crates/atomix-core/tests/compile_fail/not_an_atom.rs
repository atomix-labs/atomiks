//! Without an `Atom` impl, a value has no repr an atomic could hold and decode.

use atomix_core::Atomic;

struct Order;

fn main() {
    let _ = Atomic::new(Order);
}
