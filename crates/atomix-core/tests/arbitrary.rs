//! arbitrary on a ranged integer, which lands inside its range from any input, reaches each integer
//! of it, and reads the bytes its size hint says; and on an atomic, as its value.

#![cfg(feature = "arbitrary")]

// Under loom the impls stay on, since none reads an atomic's memory as bytes; `check-loom` builds
// this.
#[cfg(loom)]
const _: () = {
    use arbitrary::Arbitrary;
    use atomix_core::{AtomicU64, RangedU8};

    /// Compiles only where arbitrary generates a `T`.
    const fn generates<T: for<'a> Arbitrary<'a>>() {}

    generates::<RangedU8<1, 10>>();
    generates::<AtomicU64>();
};

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#[cfg(not(loom))]
#[cfg(test)]
mod tests {
    use core::any::type_name;

    use arbitrary::{Arbitrary, Unstructured};
    use atomix_core::{
        Atomic, AtomicU64, RangedI8, RangedI16, RangedI32, RangedI64, RangedI128, RangedIsize,
        RangedU8, RangedU16, RangedU32, RangedU64, RangedU128, RangedUsize,
    };

    /// How many levels of an order book a feed sends.
    type Depth = RangedU8<1, 10>;

    /// The value `bytes` give a `T`, and how many of them it read.
    fn read<'a, T: Arbitrary<'a>>(bytes: &'a [u8]) -> (T, usize) {
        let mut input = Unstructured::new(bytes);
        let value = T::arbitrary(&mut input).expect("every type this file reads takes any input");
        (value, bytes.len().strict_sub(input.len()))
    }

    /// The integers `get` reads of the `T` each byte alone gives, in order, each once.
    fn reached_by_a_byte<T, I, F>(get: F) -> Vec<I>
    where
        T: for<'a> Arbitrary<'a>,
        I: Ord,
        F: Fn(T) -> I,
    {
        let mut reached: Vec<I> = (0..=u8::MAX).map(|byte| get(read::<T>(&[byte]).0)).collect();
        reached.sort_unstable();
        reached.dedup();
        reached
    }

    /// Checks that `T` hints exactly `count` bytes, and reads that many of a longer input.
    fn hint_is_the_bytes_read<T: for<'a> Arbitrary<'a>>(count: usize) {
        let name = type_name::<T>();
        assert_eq!(T::size_hint(0), (count, Some(count)), "{name} hints {count} bytes");
        assert_eq!(T::try_size_hint(0).ok(), Some((count, Some(count))), "{name} tries the same");
        assert_eq!(read::<T>(&[0xA5; 32]).1, count, "{name} reads {count} bytes");
    }

    #[test]
    fn every_byte_lands_inside_the_range_and_every_integer_is_reached() {
        assert_eq!(reached_by_a_byte(Depth::get), Vec::from_iter(1..=10), "each of 1..=10 alone");
        let moves = reached_by_a_byte(RangedI8::<-5, 5>::get);
        assert_eq!(moves, Vec::from_iter(-5..=5), "each of -5..=5, through zero");
        let every = reached_by_a_byte(RangedU8::<0>::get);
        assert_eq!(every, Vec::from_iter(0..=u8::MAX), "every `u8` of a full range");
        let every = reached_by_a_byte(RangedI8::<{ i8::MIN }>::get);
        assert_eq!(every, Vec::from_iter(i8::MIN..=i8::MAX), "every `i8` too");
    }

    #[test]
    fn a_range_wider_than_a_byte_reaches_both_bounds() {
        type PastAByte = RangedU64<0, 256>;
        type Full = RangedI64<{ i64::MIN }>;
        assert_eq!(read::<PastAByte>(&[0, 0]).0, PastAByte::MIN, "two zero bytes give 0");
        assert_eq!(read::<PastAByte>(&[1, 0]).0, PastAByte::MAX, "and 0x0100 gives 256");
        assert_eq!(read::<RangedU64<0>>(&[0xFF; 8]).0.get(), u64::MAX, "a full range's largest");
        assert_eq!(read::<Full>(&[0; 8]).0, Full::MIN, "a full signed range's smallest");
        assert_eq!(read::<Full>(&[0xFF; 8]).0, Full::MAX, "and its largest");
    }

    #[test]
    fn an_empty_input_gives_the_smallest() {
        type Full = RangedIsize<{ isize::MIN }>;
        assert_eq!(read::<Depth>(&[]), (Depth::MIN, 0), "`MIN`, reading nothing");
        assert_eq!(read::<RangedI64<-5, 5>>(&[]).0, RangedI64::MIN, "a signed range's too");
        assert_eq!(read::<RangedU128<7>>(&[]).0, RangedU128::MIN, "a 128-bit range's too");
        assert_eq!(read::<Full>(&[]).0, Full::MIN, "and a full signed range's");
    }

    #[test]
    fn the_size_hint_is_the_bytes_read() {
        hint_is_the_bytes_read::<RangedU8<7, 7>>(0);
        hint_is_the_bytes_read::<Depth>(1);
        hint_is_the_bytes_read::<RangedU8<0>>(1);
        hint_is_the_bytes_read::<RangedU16<0, 255>>(1);
        hint_is_the_bytes_read::<RangedU16<0, 256>>(2);
        hint_is_the_bytes_read::<RangedU32<1, 0x1_0001>>(3);
        hint_is_the_bytes_read::<RangedU64<0>>(8);
        hint_is_the_bytes_read::<RangedU128<0>>(16);
        hint_is_the_bytes_read::<RangedUsize<0, 1>>(1);
        hint_is_the_bytes_read::<RangedI8<-5, 5>>(1);
        hint_is_the_bytes_read::<RangedI16<-200, 200>>(2);
        hint_is_the_bytes_read::<RangedI32<-1, 0>>(1);
        hint_is_the_bytes_read::<RangedI64<{ i64::MIN }>>(8);
        hint_is_the_bytes_read::<RangedI128<-1, { i128::MAX }>>(16);
        hint_is_the_bytes_read::<RangedIsize<0, 0>>(0);
    }

    #[test]
    fn an_atomic_is_its_value() {
        let bytes = [7, 0, 0, 0, 0, 0, 0, 0, 3];
        let (seq, bytes_read) = read::<AtomicU64>(&bytes);
        assert_eq!((seq.into_inner(), bytes_read), read::<u64>(&bytes), "as a `u64`");
        let rest =
            AtomicU64::arbitrary_take_rest(Unstructured::new(&bytes)).map(Atomic::into_inner);
        assert_eq!(rest, u64::arbitrary_take_rest(Unstructured::new(&bytes)), "the rest too");
        assert_eq!(read::<Atomic<Depth>>(&[3]).0.into_inner(), read::<Depth>(&[3]).0, "a depth");
        assert_eq!(AtomicU64::size_hint(0), u64::size_hint(0), "its size hint");
        let tried = Atomic::<Depth>::try_size_hint(0).ok();
        assert_eq!(tried, Depth::try_size_hint(0).ok(), "and the one it tries");
    }
}
