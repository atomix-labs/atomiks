//! A word keeps its tags in the low bits its pointee's alignment leaves clear: a `u32`, aligned to
//! 4, leaves two, too few for three bits of tags, and the build is refused.

#![feature(const_trait_impl)]

#[path = "../testing/pointer_word.rs"]
mod pointer_word;

use core::ptr::NonNull;

use atomix_core::RangedU8;

pointer_word::pointer_word! {
    /// A pointer to a `u32`, a version of 0 to 3 and a mark: three bits.
    struct Head, projected as HeadFields {
        0 => top: NonNull<u32>,
        1 => version: RangedU8<0, 3>,
        2 => marked: bool,
    }
}

fn main() {}
