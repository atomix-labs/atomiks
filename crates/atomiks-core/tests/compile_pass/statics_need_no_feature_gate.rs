//! Statics and consts of atomics, `get_mut` and `into_inner` among them, need no `#![feature]`.

use core::num::NonZero;

use atomiks_core::{Atomic, AtomicBool, AtomicU64, AtomicU128};

static COUNT: AtomicU64 = AtomicU64::new(0);
static READY: AtomicBool = AtomicBool::new(false);
static OWNER: Atomic<Option<NonZero<u32>>> = Atomic::new(None);
static WIDE: AtomicU128 = AtomicU128::new(u128::MAX);
static START: AtomicU64 = {
    let mut start = AtomicU64::new(1);
    *start.get_mut() += 1;
    start
};
const ONE: u64 = AtomicU64::new(1).into_inner();

fn main() {
    let _ = (&COUNT, &READY, &OWNER, &WIDE, &START, ONE);
}
