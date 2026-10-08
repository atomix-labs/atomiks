//! An optional ranged integer over every `u8`, 0 to 255, leaves `None` no integer, so its reprs,
//! and with them an atomic of one, are refused.

use atomix_core::{Atom, ReprRange};
use deranged::OptionRangedU8;

/// The reprs of an optional ranged integer over every `u8`.
const REPRS: ReprRange<u8> = <OptionRangedU8<0, 255> as Atom>::REPRS;

fn main() {
    let _ = REPRS;
}
