//! No tag can share the low bits of a pointer to a trait object, whose alignment is known only at
//! run time: a tag beside one is refused at the tag.

use core::ptr::NonNull;

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
struct Callback {
    call: NonNull<dyn Fn()>,
    armed: bool,
}

fn main() {}
