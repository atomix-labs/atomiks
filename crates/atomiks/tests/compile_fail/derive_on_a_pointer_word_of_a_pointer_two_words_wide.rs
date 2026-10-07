//! A pointer to a trait object holds its vtable beside its address, two words: a pointer word of
//! one is refused at the pointee.

use core::ptr::NonNull;

use atomiks::Atom;

#[derive(Clone, Copy, Atom)]
struct Callback {
    call: NonNull<dyn Fn()>,
    armed: bool,
}

fn main() {}
