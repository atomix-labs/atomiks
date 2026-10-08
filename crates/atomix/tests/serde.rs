//! serde on an atomic of a type that derives both `Atom` and serde's traits: the value as the
//! type's own derive writes it, and a ranged field refused outside its range.

#![cfg(all(feature = "derive", feature = "serde"))]
// Loom's cells exist only inside a model.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    use atomix::ordering::Acquire;
    use atomix::{Atom, Atomic, RangedU64};
    use serde::{Deserialize, Serialize};

    /// The side of the book an order rests on.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom, Serialize, Deserialize)]
    enum Side {
        /// A buy.
        Bid,
        /// A sell.
        Ask,
    }

    /// A resting quote, packed into a `u64`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom, Serialize, Deserialize)]
    struct Quote {
        /// The price, in ticks.
        price: u32,
        /// The quantity, in lots.
        quantity: u16,
        /// The side it rests on.
        side: Side,
    }

    /// An owner's id, from 3, which leaves 0 to 2 spare.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom, Serialize, Deserialize)]
    struct OwnerId(RangedU64<3>);

    #[test]
    fn an_atomic_of_a_derived_type_is_the_value_its_derive_writes() {
        let best_bid = Atomic::new(Some(Quote { price: 10_050, quantity: 300, side: Side::Bid }));
        let text = serde_json::to_string(&best_bid).expect("serde_json writes any quote");
        assert_eq!(text, r#"{"price":10050,"quantity":300,"side":"Bid"}"#, "the quote, loaded");
        let back: Atomic<Option<Quote>> = serde_json::from_str(&text).expect("the text is a quote");
        assert_eq!(back.load(Acquire), best_bid.load(Acquire), "and back");
        assert_eq!(
            serde_json::to_string(&Atomic::new(None::<Quote>)).ok().as_deref(),
            Some("null"),
            "and no quote, as `null`"
        );
    }

    #[test]
    fn a_derived_newtype_of_a_ranged_integer_refuses_an_integer_outside_its_range() {
        let refused = serde_json::from_str::<Atomic<OwnerId>>("2").expect_err("2 is below 3");
        assert_eq!(
            refused.to_string(),
            "invalid value: integer `2`, expected an integer in 3..=18446744073709551615 at line 1 \
             column 1",
            "the range's message, through the newtype and the atomic"
        );
    }
}
