//! An or of two integers in a range can leave it (`8 | 3` is 11, past 10), while the repr's or
//! would store it: a ranged integer has no `AtomBitwise`.

use atomiks_core::ordering::Relaxed;
use atomiks_core::{Atomic, RangedU8};

fn main() {
    let depth = RangedU8::<1, 10>::new(8).expect("8 is from 1 to 10");
    Atomic::new(depth).or(RangedU8::new(3).expect("as is 3"), Relaxed);
}
