//! A 128-bit `swap` would hide a compare-exchange loop behind a name that promises none: atomiks
//! uses no 128-bit exchange instruction.

use atomiks::AtomicU128;
use atomiks::ordering::AcqRel;

fn main() {
    let _ = AtomicU128::new(0).swap(1, AcqRel);
}
