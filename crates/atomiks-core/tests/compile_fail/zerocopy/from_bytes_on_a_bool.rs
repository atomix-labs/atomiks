//! A byte of 2 is no `bool`, so its atomic reads only from checked bytes, as core's `AtomicBool`
//! does.

use atomiks_core::AtomicBool;
use zerocopy::FromBytes;

fn main() {
    let _ = AtomicBool::read_from_bytes(&[2]);
}
