//! A newtype takes a capability only from a field that has it: `Saturating`'s add saturates where
//! its repr's wraps, so it has no `AtomAdd`, though its repr and validity would allow one.

use core::num::Saturating;

use atomix::{Atom, AtomAdd};

#[derive(Clone, Copy, Atom, AtomAdd)]
struct Volume(Saturating<u32>);

fn main() {}
