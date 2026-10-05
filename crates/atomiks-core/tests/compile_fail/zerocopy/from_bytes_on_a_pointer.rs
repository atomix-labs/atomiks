//! A pointer built from bytes has no provenance, so its atomic reads only from zeros, a null
//! pointer, as core's `AtomicPtr` does.

use atomiks_core::Atomic;
use zerocopy::FromBytes;

fn main() {
    let _ = Atomic::<*mut u8>::read_from_bytes(&[0; 8]);
}
