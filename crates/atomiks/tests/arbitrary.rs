//! arbitrary on an atomic of a derived value, as a fuzz target builds one: it holds the value the
//! same bytes give alone.

#![cfg(all(feature = "derive", feature = "arbitrary"))]
// Loom's cells exist only inside a model.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    use arbitrary::{Arbitrary, Result, Unstructured};
    use atomiks::{Atom, Atomic, RangedU32};

    /// A resting quote, packed into a `u64`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Quote {
        /// The price, in ticks.
        price: RangedU32<1, 100_000>,
        /// The quantity, in lots.
        quantity: u16,
    }

    /// Each field in turn.
    impl<'a> Arbitrary<'a> for Quote {
        fn arbitrary(input: &mut Unstructured<'a>) -> Result<Self> {
            Ok(Self { price: input.arbitrary()?, quantity: input.arbitrary()? })
        }
    }

    #[test]
    fn an_atomic_of_a_derived_value_holds_the_value_its_bytes_give() {
        for byte in 0..=u8::MAX {
            let bytes = [byte; 8];
            let actual =
                Atomic::<Quote>::arbitrary(&mut Unstructured::new(&bytes)).map(Atomic::into_inner);
            let expected = Quote::arbitrary(&mut Unstructured::new(&bytes));
            assert_eq!(actual.ok(), expected.ok(), "{byte:#04x} gives the quote it gives alone");
        }
        let quote = Atomic::<Quote>::arbitrary(&mut Unstructured::new(&[])).map(Atomic::into_inner);
        assert_eq!(
            quote.ok(),
            Some(Quote { price: RangedU32::MIN, quantity: 0 }),
            "and the smallest"
        );
    }
}
