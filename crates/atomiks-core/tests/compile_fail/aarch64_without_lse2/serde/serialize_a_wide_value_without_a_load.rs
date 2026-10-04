//! Without LSE2, a 128-bit value's only read is a compare-exchange, which writes: no `Serialize`,
//! as no `Debug`.

use atomiks_core::AtomicU128;
use serde_core::Serialize;

/// Compiles only where `T` serializes.
const fn serializes<T: Serialize>() {}

fn main() {
    serializes::<AtomicU128>();
}
