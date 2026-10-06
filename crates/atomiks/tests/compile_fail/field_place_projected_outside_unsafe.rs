//! The one way to a field's place but its module's projection is the hidden `project_field`,
//! which is `unsafe`: a module that projects a field it does not see breaks that field's privacy,
//! which only an `unsafe` block answers for.

use atomiks::{Atom, Atomic};

mod book {
    use atomiks::Atom;

    /// A resting quote, whose fill only this module changes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    pub struct Quote {
        /// How many.
        pub quantity: u32,
        /// How many of four lots filled: at most 4.
        filled: u8,
    }

    impl Quote {
        /// A quote of `quantity`, none of it filled.
        pub const fn new(quantity: u32) -> Self {
            Self { quantity, filled: 0 }
        }
    }
}

/// A book of one quote, whose field is a place of a `Quote`.
#[derive(Clone, Copy, Atom)]
struct Book {
    quote: book::Quote,
    open: bool,
}

fn main() {
    let book = Atomic::new(Book { quote: book::Quote::new(5), open: true });
    let quote = book.fields().quote;
    let _ = atomiks::__private::project_field::<_, u8, 1>(quote);
}
