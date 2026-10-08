//! The base's wrapping add can leave the width (a `u3`'s 7 and 1 make 8), while the repr's add
//! would store it: an arbitrary-int integer has no `AtomAdd`.

use arbitrary_int::u3;
use atomix_core::Atomic;
use atomix_core::ordering::Relaxed;

fn main() {
    Atomic::new(u3::new(7)).fetch_add(1, Relaxed);
}
