//! serde on an atomic, as the value a load reads, and on each ranged integer, as its integer:
//! refused outside the range in serde's words, at `serde_json`'s line and column.

#![cfg(feature = "serde")]

// Under loom the impls stay on, since none reads an atomic's memory as bytes; `check-loom` builds
// this.
#[cfg(loom)]
const _: () = {
    use atomix_core::{AtomicU64, RangedU8};
    use serde_core::{Deserialize, Serialize};

    /// Compiles only where `T` serializes and deserializes.
    const fn has_serde_traits<T: Serialize + for<'de> Deserialize<'de>>() {}

    has_serde_traits::<RangedU8<1, 10>>();
    has_serde_traits::<AtomicU64>();
};

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#[cfg(not(loom))]
#[cfg(test)]
mod tests {
    use core::num::{NonZero, Wrapping};

    #[cfg(wide)]
    use atomix_core::AtomicU128;
    use atomix_core::ordering::Acquire;
    #[cfg(wide)]
    use atomix_core::ordering::Relaxed;
    use atomix_core::{
        Atom, Atomic, Load, RangedI8, RangedI16, RangedI32, RangedI64, RangedI128, RangedIsize,
        RangedU8, RangedU16, RangedU32, RangedU64, RangedU128, RangedUsize,
    };
    use serde_core::Serialize;
    use serde_core::de::value::Error;
    use serde_core::de::{DeserializeOwned, IntoDeserializer};

    /// How many levels of an order book a feed sends.
    type Depth = RangedU8<1, 10>;

    /// `value` as JSON, and the value read back from it.
    fn round_trip<T: Serialize + DeserializeOwned>(value: &T) -> (String, T) {
        let text = serde_json::to_string(value).expect("serde_json writes any scalar");
        let back = serde_json::from_str(&text).expect("the text is what `value` wrote");
        (text, back)
    }

    /// `value` in an atomic as JSON, and the value an atomic read back from it holds.
    fn atomic_round_trip<T>(value: T) -> (String, T)
    where
        T: Atom + Serialize + DeserializeOwned,
        T::Repr: Load,
    {
        let (text, back) = round_trip(&Atomic::from(value));
        (text, back.load(Acquire))
    }

    /// `serde_json`'s message for `text` as a `T`.
    fn refusal<T: DeserializeOwned>(text: &str) -> String {
        serde_json::from_str::<T>(text).err().expect("the text is refused").to_string()
    }

    /// `integer` as a `T`, read as a format that is not self-describing reads it: through the
    /// visit method of the integer's own type. A refusal is its message.
    fn read_at_its_width<T: DeserializeOwned, I: IntoDeserializer<'static, Error>>(
        integer: I,
    ) -> Result<T, String> {
        T::deserialize(integer.into_deserializer()).map_err(|refusal| refusal.to_string())
    }

    #[test]
    fn an_atomic_is_the_value_it_holds() {
        assert_eq!(atomic_round_trip(42_u64), ("42".to_owned(), 42), "an integer");
        assert_eq!(atomic_round_trip(-7_i8), ("-7".to_owned(), -7), "a signed one");
        assert_eq!(atomic_round_trip(true), ("true".to_owned(), true), "a `bool`");
        assert_eq!(atomic_round_trip('€'), ("\"€\"".to_owned(), '€'), "a `char`, as text");
        assert_eq!(
            atomic_round_trip(NonZero::new(7_u64)),
            ("7".to_owned(), NonZero::new(7)),
            "an `Option` of a `NonZero`"
        );
        assert_eq!(
            atomic_round_trip(None::<NonZero<u64>>),
            ("null".to_owned(), None),
            "and its `None`, as `null`"
        );
        assert_eq!(
            atomic_round_trip(Wrapping(u32::MAX)),
            ("4294967295".to_owned(), Wrapping(u32::MAX)),
            "a `Wrapping`, as its integer"
        );
        assert_eq!(
            atomic_round_trip(Depth::MAX),
            ("10".to_owned(), Depth::MAX),
            "and a ranged integer"
        );
    }

    #[cfg(wide)]
    #[test]
    fn a_wide_atomic_deserializes_on_every_target() {
        let wide: AtomicU128 = serde_json::from_str("340282366920938463463374607431768211455")
            .expect("the text is `u128::MAX`");
        assert_eq!(wide.load_rmw(Relaxed), u128::MAX, "read with a compare-exchange");
    }

    #[cfg(wide_load_store)]
    #[test]
    fn a_wide_atomic_serializes_where_it_loads() {
        assert_eq!(
            serde_json::to_string(&AtomicU128::new(u128::MAX)).ok().as_deref(),
            Some("340282366920938463463374607431768211455"),
            "written with a load"
        );
    }

    #[test]
    fn each_ranged_integer_is_its_integer_at_either_end() {
        assert_eq!(round_trip(&Depth::MIN), ("1".to_owned(), Depth::MIN), "`u8`");
        assert_eq!(
            round_trip(&RangedU16::<1_000>::MAX),
            ("65535".to_owned(), RangedU16::MAX),
            "`u16`"
        );
        assert_eq!(
            round_trip(&RangedU32::<0, 86_399>::MAX),
            ("86399".to_owned(), RangedU32::MAX),
            "`u32`"
        );
        assert_eq!(
            round_trip(&RangedU64::<3>::MAX),
            ("18446744073709551615".to_owned(), RangedU64::MAX),
            "`u64`"
        );
        assert_eq!(
            round_trip(&RangedU128::<1>::MAX),
            ("340282366920938463463374607431768211455".to_owned(), RangedU128::MAX),
            "`u128`, past 64 bits"
        );
        assert_eq!(
            round_trip(&RangedUsize::<1, 10>::MAX),
            ("10".to_owned(), RangedUsize::MAX),
            "`usize`"
        );
        assert_eq!(round_trip(&RangedI8::<-5, 5>::MIN), ("-5".to_owned(), RangedI8::MIN), "`i8`");
        assert_eq!(
            round_trip(&RangedI16::<{ i16::MIN }, -1>::MIN),
            ("-32768".to_owned(), RangedI16::MIN),
            "`i16`"
        );
        assert_eq!(
            round_trip(&RangedI32::<-100, 100>::MAX),
            ("100".to_owned(), RangedI32::MAX),
            "`i32`"
        );
        assert_eq!(
            round_trip(&RangedI64::<{ i64::MIN }, 0>::MIN),
            ("-9223372036854775808".to_owned(), RangedI64::MIN),
            "`i64`"
        );
        assert_eq!(
            round_trip(&RangedI128::<{ i128::MIN }, 0>::MIN),
            ("-170141183460469231731687303715884105728".to_owned(), RangedI128::MIN),
            "`i128`, past 64 bits"
        );
        assert_eq!(
            round_trip(&RangedIsize::<-10, 10>::MIN),
            ("-10".to_owned(), RangedIsize::MIN),
            "and `isize`"
        );
    }

    #[test]
    fn a_ranged_integer_refuses_an_integer_outside_its_range() {
        assert_eq!(
            refusal::<Depth>("0"),
            "invalid value: integer `0`, expected an integer in 1..=10 at line 1 column 1",
            "one below"
        );
        assert_eq!(
            refusal::<Depth>("11"),
            "invalid value: integer `11`, expected an integer in 1..=10 at line 1 column 2",
            "one above"
        );
        assert_eq!(
            refusal::<RangedI8<-5, 5>>("-6"),
            "invalid value: integer `-6`, expected an integer in -5..=5 at line 1 column 2",
            "a signed one below"
        );
        assert_eq!(
            refusal::<RangedU64<3>>("2"),
            "invalid value: integer `2`, expected an integer in 3..=18446744073709551615 at line 1 \
             column 1",
            "`MAX` by default"
        );
        assert_eq!(
            refusal::<RangedUsize<1, 10>>("11"),
            "invalid value: integer `11`, expected an integer in 1..=10 at line 1 column 2",
            "a `usize`"
        );
        assert_eq!(
            refusal::<Atomic<Depth>>("11"),
            "invalid value: integer `11`, expected an integer in 1..=10 at line 1 column 2",
            "and inside an atomic"
        );
    }

    #[test]
    fn an_integer_outside_the_integers_type_is_refused_by_the_range() {
        assert_eq!(
            refusal::<Depth>("300"),
            "invalid value: integer `300`, expected an integer in 1..=10 at line 1 column 3",
            "past `u8`"
        );
        assert_eq!(
            refusal::<Depth>("-1"),
            "invalid value: integer `-1`, expected an integer in 1..=10 at line 1 column 2",
            "a negative one for an unsigned integer"
        );
        assert_eq!(
            refusal::<RangedI8<-5, 5>>("-200"),
            "invalid value: integer `-200`, expected an integer in -5..=5 at line 1 column 4",
            "below `i8`"
        );
    }

    #[test]
    fn a_ranged_integer_reads_an_integer_narrower_than_64_bits() {
        assert_eq!(read_at_its_width(7_u8).ok(), Depth::new(7), "a `u8`");
        assert_eq!(read_at_its_width(10_u16).ok(), Some(Depth::MAX), "a `u16`");
        assert_eq!(read_at_its_width(1_u32).ok(), Some(Depth::MIN), "a `u32`");
        assert_eq!(read_at_its_width(-5_i8).ok(), Some(RangedI8::<-5, 5>::MIN), "an `i8`");
        assert_eq!(read_at_its_width(5_i16).ok(), Some(RangedI8::<-5, 5>::MAX), "an `i16`");
        assert_eq!(
            read_at_its_width(3_i32).ok(),
            Some(RangedU64::<3>::MIN),
            "an `i32`, for an unsigned integer"
        );
    }

    #[test]
    fn a_ranged_integer_refuses_a_narrow_integer_outside_its_range() {
        assert_eq!(
            read_at_its_width::<Depth, _>(11_u8).err().as_deref(),
            Some("invalid value: integer `11`, expected an integer in 1..=10"),
            "a `u8` above"
        );
        assert_eq!(
            read_at_its_width::<Depth, _>(300_u16).err().as_deref(),
            Some("invalid value: integer `300`, expected an integer in 1..=10"),
            "a `u16` past `u8`"
        );
        assert_eq!(
            read_at_its_width::<Depth, _>(-1_i32).err().as_deref(),
            Some("invalid value: integer `-1`, expected an integer in 1..=10"),
            "a negative `i32` for an unsigned integer"
        );
        assert_eq!(
            read_at_its_width::<RangedI8<-5, 5>, _>(-6_i16).err().as_deref(),
            Some("invalid value: integer `-6`, expected an integer in -5..=5"),
            "an `i16` below"
        );
    }

    #[test]
    fn a_wide_integer_past_64_bits_is_named_by_its_type() {
        assert_eq!(
            refusal::<RangedU128<1, 10>>("11"),
            "invalid value: integer `11`, expected an integer in 1..=10 at line 1 column 2",
            "one within 64 bits by its value"
        );
        assert_eq!(
            refusal::<RangedI128<-5, 5>>("-6"),
            "invalid value: integer `-6`, expected an integer in -5..=5 at line 1 column 2",
            "a signed one too"
        );
        assert_eq!(
            refusal::<RangedU128<1, 10>>("18446744073709551616"),
            "invalid value: u128, expected an integer in 1..=10 at line 1 column 20",
            "one past them by its type, as serde names it"
        );
        assert_eq!(
            refusal::<RangedI128<-5, 5>>("-9223372036854775809"),
            "invalid value: i128, expected an integer in -5..=5 at line 1 column 20",
            "and one below them"
        );
    }

    #[test]
    fn a_refusal_names_the_line_and_column_of_the_integer() {
        assert_eq!(
            refusal::<Vec<Depth>>("[4,\n 11]"),
            "invalid value: integer `11`, expected an integer in 1..=10 at line 2 column 3",
            "the second line's integer"
        );
    }

    #[test]
    fn a_ranged_integer_refuses_what_is_not_an_integer_with_its_range() {
        assert_eq!(
            refusal::<Depth>("\"4\""),
            "invalid type: string \"4\", expected an integer in 1..=10 at line 1 column 3",
            "text"
        );
    }
}
