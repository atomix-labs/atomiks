//! A `bool` spends both its reprs, so its `Option` has no repr for `None`: the refusal names the
//! enum of three states to derive.

use atomix_core::Atomic;

static VOTE: Atomic<Option<bool>> = Atomic::new(None);

fn main() {}
