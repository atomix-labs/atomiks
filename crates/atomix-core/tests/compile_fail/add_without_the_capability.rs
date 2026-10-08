//! Adding to a `bool` means nothing: `fetch_add` needs `AtomAdd`, which marks the values whose
//! repr's add is their own.

use atomix_core::AtomicBool;
use atomix_core::ordering::Relaxed;

fn main() {
    AtomicBool::new(false).fetch_add(true, Relaxed);
}
