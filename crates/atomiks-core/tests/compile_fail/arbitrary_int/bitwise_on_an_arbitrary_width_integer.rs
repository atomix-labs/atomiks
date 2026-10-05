//! The base's `not` sets the bits above the width (a `u3`'s 0 becomes 255), while the repr's would
//! store them: an arbitrary-int integer has no `AtomBitwise`.

use arbitrary_int::u3;
use atomiks_core::Atomic;
use atomiks_core::ordering::Relaxed;

fn main() {
    Atomic::new(u3::new(0)).not(Relaxed);
}
