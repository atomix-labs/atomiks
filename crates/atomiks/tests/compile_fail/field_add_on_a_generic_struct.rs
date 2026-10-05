//! No constant knows where a generic struct's last field ends, in each instance, so none adds in
//! place, even where an instance's fills the repr: its add is `update`.

#![feature(const_trait_impl)]

use atomiks::ordering::Relaxed;
use atomiks::{Atom, Atomic};

/// Flags of some width, then a count above them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
#[atom(repr = u64)]
struct Counted<F> {
    flags: F,
    count: u32,
}

fn main() {
    let counted = Atomic::new(Counted { flags: 0_u32, count: 1 });
    counted.fields().count.update(Relaxed, Relaxed, |count| count + 1);
    counted.fields().count.fetch_add(1, Relaxed);
}
