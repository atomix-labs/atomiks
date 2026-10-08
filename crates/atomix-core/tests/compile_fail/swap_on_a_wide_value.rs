//! A 128-bit `swap` would hide a compare-exchange loop behind a name that promises none: atomix
//! uses no 128-bit exchange instruction.

use atomix_core::AtomicU128;
use atomix_core::ordering::AcqRel;

fn main() {
    let _ = AtomicU128::new(0).swap(1, AcqRel);
}
