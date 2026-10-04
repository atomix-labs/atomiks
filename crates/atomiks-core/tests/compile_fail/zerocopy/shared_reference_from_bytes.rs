//! A shared reference to an atomic writes, so none comes from shared bytes: an atomic is never
//! `Immutable`, which `ref_from_bytes` needs. `mut_from_bytes` takes the bytes exclusively, and a
//! reborrow shares them.

use atomiks_core::AtomicU64;
use zerocopy::Immutable;

/// Compiles only where a shared `T` never writes.
const fn is_immutable<T: Immutable>() {}

fn main() {
    is_immutable::<AtomicU64>();
}
