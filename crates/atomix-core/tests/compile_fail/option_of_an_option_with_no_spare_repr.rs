//! Each `Option` takes a spare repr, so a `NonZero`'s one serves only the inner `Option`: the
//! refusal names a type with a spare repr for each.

use core::num::NonZero;

use atomix_core::Atomic;

static OWNER: Atomic<Option<Option<NonZero<u64>>>> = Atomic::new(None);

fn main() {}
