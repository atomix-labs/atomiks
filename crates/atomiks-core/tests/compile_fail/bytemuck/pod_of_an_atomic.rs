//! A shared reference to an atomic writes, so no cell is `Copy`, and no atomic is bytemuck's `Pod`:
//! none lends its bytes.

use atomiks_core::AtomicU64;
use bytemuck::NoUninit;

/// Compiles only where `T` lends its bytes, as `bytes_of` needs.
const fn lends_its_bytes<T: NoUninit>() {}

fn main() {
    lends_its_bytes::<AtomicU64>();
}
