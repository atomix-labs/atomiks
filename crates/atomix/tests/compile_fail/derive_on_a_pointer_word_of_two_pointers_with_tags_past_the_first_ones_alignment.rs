//! Two pointers keep their tags in the first one's low bits: a `u16`, aligned to 2, leaves one
//! bit, too few for two tags. The struct is two words already, so the refusal, at the struct,
//! offers the tags no word of their own.

use core::ptr::NonNull;

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
struct Pair {
    first: NonNull<u16>,
    second: NonNull<u64>,
    marked: bool,
    sealed: bool,
}

fn main() {}
