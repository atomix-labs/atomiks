//! A pointer's bytes would expose its address and drop its provenance, so its atomic has none, as
//! core's `AtomicPtr` has none.

use atomiks_core::Atomic;
use zerocopy::IntoBytes;

/// Compiles only where `T`'s bytes may be read.
const fn has_into_bytes<T: IntoBytes>() {}

fn main() {
    has_into_bytes::<Atomic<*mut u8>>();
}
