//! A public struct may have a field of a less visible type, as Rust allows: deriving `Atom` keeps
//! the field's type out of every public interface, so a struct of a private and a `pub(crate)`
//! field type, and a public struct in a private module, each derive, and their modules change the
//! private fields through the projection.

#![deny(missing_docs, missing_debug_implementations, private_bounds, private_interfaces, unused)]

use atomix::ordering::{Acquire, Relaxed};
use atomix::{Atom, Atomic};

/// The side of the book an order rests on, which no other crate names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
enum Side {
    /// A buy.
    Bid,
    /// A sell.
    Ask,
}

/// How an order trades, which only this crate names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub(crate) enum Kind {
    /// At its price or better.
    Limit,
    /// At any price.
    Market,
}

/// An order, whose side and kind only this crate sees.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct Order {
    /// How many.
    pub quantity: u32,
    /// Which side.
    side: Side,
    /// How it trades.
    pub(crate) kind: Kind,
}

/// The book, whose quote's side is its own.
mod book {
    use atomix::ordering::Relaxed;
    use atomix::{Atom, Atomic};

    /// The side of the book a quote rests on, which no other module names.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    enum Side {
        /// A buy.
        Bid,
        /// A sell.
        Ask,
    }

    /// A resting quote, public in a private module.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    pub struct Quote {
        /// How many.
        pub quantity: u32,
        /// Which side.
        side: Side,
    }

    /// A bid of `quantity`.
    pub(crate) const fn bid(quantity: u32) -> Quote {
        Quote { quantity, side: Side::Bid }
    }

    /// Turns `quote` to the other side: this module changes the field no other module sees.
    pub(crate) fn turn(quote: &Atomic<Quote>) -> bool {
        quote.fields().side.update(Relaxed, Relaxed, |_| Side::Ask).side == Side::Bid
    }
}

fn main() {
    let order = Atomic::new(Order { quantity: 5, side: Side::Bid, kind: Kind::Limit });
    order.fields().side.update(Relaxed, Relaxed, |_| Side::Ask);
    order.fields().kind.update(Relaxed, Relaxed, |_| Kind::Market);
    order.fields().quantity.update(Relaxed, Relaxed, |quantity| quantity * 2);
    let last = Order { quantity: 10, side: Side::Ask, kind: Kind::Market };
    assert_eq!(order.load(Acquire), last, "each field changed through its place");
    let quote = Atomic::new(book::bid(3));
    assert!(book::turn(&quote), "a bid before the turn");
    assert_eq!(quote.fields().quantity.load(Acquire), 3, "and the public field read outside");
}
