//! An `Option` with no spare repr is refused when its atomic is built at run time too.

use atomiks::Atomic;

fn main() {
    let _ = Atomic::<Option<u64>>::from(None);
}
