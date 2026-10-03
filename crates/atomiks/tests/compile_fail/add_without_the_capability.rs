//! Adding to a `bool` means nothing: `fetch_add` needs `AtomAdd`, which marks the values whose
//! repr's add is their own.

use atomiks::AtomicBool;
use atomiks::ordering::Relaxed;

fn main() {
    let _ = AtomicBool::new(false).fetch_add(true, Relaxed);
}
