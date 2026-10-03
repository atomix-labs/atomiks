//! The repr a generic newtype states is its field's: an instance whose field has another is refused
//! where it is used.

#![feature(const_trait_impl)]

use atomiks::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
#[atom(repr = u32)]
struct Word<T>(T);

static WORD: Atomic<Word<u64>> = Atomic::new(Word(7));

fn main() {
    let _ = &WORD;
}
