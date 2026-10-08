//! Or-ing two `char`s can make a surrogate, which is no `char`: the bitwise operations need
//! `AtomBitwise`, which only values whose every repr decodes have.

use atomix_core::Atomic;
use atomix_core::ordering::Relaxed;

fn main() {
    Atomic::new('a').or('b', Relaxed);
}
