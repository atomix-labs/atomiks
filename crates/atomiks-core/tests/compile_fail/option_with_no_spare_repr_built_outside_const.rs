//! An `Option` with no spare repr is refused when its atomic is built at run time too.

use atomiks_core::Atomic;

fn main() {
    let _ = Atomic::<Option<u64>>::from(None);
}
