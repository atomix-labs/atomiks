//! A field private to its module stays private through the projection: its place takes the
//! field's own visibility, so no other module changes it.

use atomix::ordering::Relaxed;
use atomix::Atomic;

mod book {
    use atomix::Atom;

    /// The side of the book an order rests on.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    pub enum Side {
        /// A buy.
        Bid,
        /// A sell.
        Ask,
    }

    /// A resting quote, whose side only this module changes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    pub struct Quote {
        /// How many.
        pub quantity: u32,
        /// Which side.
        side: Side,
    }

    impl Quote {
        /// A bid of `quantity`.
        pub const fn bid(quantity: u32) -> Self {
            Self { quantity, side: Side::Bid }
        }
    }
}

fn main() {
    let quote = Atomic::new(book::Quote::bid(5));
    quote.fields().quantity.update(Relaxed, Relaxed, |quantity| quantity + 1);
    quote.fields().side.update(Relaxed, Relaxed, |_| book::Side::Ask);
}
