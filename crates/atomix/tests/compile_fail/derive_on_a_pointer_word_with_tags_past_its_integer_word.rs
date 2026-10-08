//! Tags no alignment holds take a word of their own beside the pointer, and no third word holds
//! more: tags past that word's 64 bits are refused at the struct.

use core::ptr::NonNull;

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
struct Head {
    top: NonNull<u64>,
    version: u64,
    marked: bool,
}

fn main() {}
