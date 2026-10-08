//! A tag shares a pointer's low bits: a value stored as an integer is no `PtrAtom`, and generic
//! code that asks for one refuses it.

use atomix_core::PtrAtom;

/// Compiles only where `T` is stored as a pointer.
fn stored_as_a_pointer<T: PtrAtom>() {}

fn main() {
    stored_as_a_pointer::<*mut u64>();
    stored_as_a_pointer::<u64>();
}
