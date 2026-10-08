//! A `#[repr(C)]` enum's discriminants are C's `int`: one past it, which rustc only warns of, is
//! refused, naming the bits the discriminants need.

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
#[repr(C)]
enum Big {
    Small = 0,
    Large = 1 << 32,
}

fn main() {}
