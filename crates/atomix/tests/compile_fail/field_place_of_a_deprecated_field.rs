//! A deprecated field's place, through the projection, is deprecated as the field is.

#![deny(deprecated)]

use atomix::ordering::Acquire;
use atomix::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
struct Order {
    #[deprecated = "read `state`"]
    live: bool,
    state: u8,
}

fn is_live(order: &Atomic<Order>) -> bool {
    order.fields().live.load(Acquire)
}

fn main() {}
