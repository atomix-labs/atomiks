//! A path is a type alone, of which no code outside atomix builds a place: a field private to its
//! module, which may carry an invariant the module's unsafe code relies on, is reached only through
//! that module's projection.

use atomix::ordering::Relaxed;
use atomix::{Atomic, Field};

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

    /// A resting quote, whose side and fill only this module changes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    pub struct Quote {
        /// How many.
        pub quantity: u32,
        /// Which side.
        side: Side,
        /// How many of four lots filled: at most 4.
        filled: u8,
    }

    impl Quote {
        /// A bid of `quantity`, none of it filled.
        pub const fn bid(quantity: u32) -> Self {
            Self { quantity, side: Side::Bid, filled: 0 }
        }
    }
}

fn main() {
    let quote = Atomic::new(book::Quote::bid(5));
    let side = quote.field(Field::<book::Quote, 1, book::Side>::new());
    side.update(Relaxed, Relaxed, |_| book::Side::Ask);
    let _ = Field::<book::Quote, 2, u8>::default();
}
