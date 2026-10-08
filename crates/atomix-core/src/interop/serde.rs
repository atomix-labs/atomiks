//! serde's `Serialize` and `Deserialize`: an atomic as its value, and a ranged integer as its
//! integer, checked against the range on the way in.

use core::fmt::{self, Formatter};
use core::marker::PhantomData;

use serde_core::de::{Error, Unexpected, Visitor};
use serde_core::{Deserialize, Deserializer, Serialize, Serializer};

use crate::atom::Atom;
use crate::atomic::Atomic;
use crate::ordering::Relaxed;
use crate::primitive::Load;
use crate::ranged::{self, each_ranged_integer};

/// Serializes the value a `Relaxed` load reads, as serde does core's atomics.
///
/// A 128-bit value has it only where its repr has [`Load`], as with `Debug`: elsewhere its only
/// read is a compare-exchange, which writes.
impl<T: Atom + Serialize> Serialize for Atomic<T>
where
    T::Repr: Load,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // ORDERING: Relaxed, as serde's impls for core's atomics: a snapshot publishes nothing and
        // pairs with no store.
        self.load(Relaxed).serialize(serializer)
    }
}

/// Deserializes a `T`, and holds it in a new atomic.
impl<'de, T: Atom + Deserialize<'de>> Deserialize<'de> for Atomic<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self::from)
    }
}

/// An integer, read at its width: by the `Deserializer` method serde's own impl for it calls.
trait DeserializeAtWidth {
    /// Reads an integer from `deserializer` through `visitor`, by serde's method for it, so a
    /// format that is not self-describing reads the bytes the integer's own `Serialize` wrote.
    fn deserialize_at_width<'de, D: Deserializer<'de>, V: Visitor<'de>>(
        deserializer: D, visitor: V,
    ) -> Result<V::Value, D::Error>;
}

/// Implements `DeserializeAtWidth` for each integer, `$int`, read by `$method`.
macro_rules! integer {
    ($($int:ident by $method:ident),+ $(,)?) => {$(
        impl DeserializeAtWidth for $int {
            #[inline]
            fn deserialize_at_width<'de, D: Deserializer<'de>, V: Visitor<'de>>(
                deserializer: D, visitor: V,
            ) -> Result<V::Value, D::Error> {
                deserializer.$method(visitor)
            }
        }
    )+};
}

// serde reads `usize` and `isize` as 64-bit integers, on every target.
integer!(
    u8 by deserialize_u8,
    u16 by deserialize_u16,
    u32 by deserialize_u32,
    u64 by deserialize_u64,
    u128 by deserialize_u128,
    usize by deserialize_u64,
    i8 by deserialize_i8,
    i16 by deserialize_i16,
    i32 by deserialize_i32,
    i64 by deserialize_i64,
    i128 by deserialize_i128,
    isize by deserialize_i64,
);

/// Reads a ranged integer, `R`, from any integer serde reads, refusing one outside the range in
/// serde's words: "invalid value: integer `11`, expected an integer in 1..=10".
struct RangedVisitor<R>(
    /// The ranged integer read.
    PhantomData<R>,
);

/// Implements `Serialize` and `Deserialize` for a ranged integer, `$name`, over `$int`.
macro_rules! ranged {
    ($name:ident($int:ident)) => {
        #[doc = concat!("Serializes the integer, as a `", stringify!($int), "`.")]
        impl<const MIN: $int, const MAX: $int> Serialize for ranged::$name<MIN, MAX> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.get().serialize(serializer)
            }
        }

        /// Deserializes an integer from `MIN` to `MAX`.
        ///
        /// It refuses any other by the range, even one past the integer's type: a `RangedU8<1, 10>`
        /// refuses 300 with "invalid value: integer `300`, expected an integer in 1..=10".
        impl<'de, const MIN: $int, const MAX: $int> Deserialize<'de> for ranged::$name<MIN, MAX> {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                $int::deserialize_at_width(deserializer, RangedVisitor::<Self>(PhantomData))
            }
        }

        // serde passes every integer narrower than 64 bits on as a 64-bit one, and each 64-bit
        // one goes on here as a 128-bit one.
        impl<const MIN: $int, const MAX: $int> Visitor<'_>
            for RangedVisitor<ranged::$name<MIN, MAX>>
        {
            type Value = ranged::$name<MIN, MAX>;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                write!(formatter, "an integer in {MIN}..={MAX}")
            }

            fn visit_u64<E: Error>(self, integer: u64) -> Result<Self::Value, E> {
                self.visit_u128(u128::from(integer))
            }

            fn visit_i64<E: Error>(self, integer: i64) -> Result<Self::Value, E> {
                self.visit_i128(i128::from(integer))
            }

            // `Unexpected` holds no integer past 64 bits: serde's own 128-bit integers name the
            // type instead.
            fn visit_u128<E: Error>(self, integer: u128) -> Result<Self::Value, E> {
                $int::try_from(integer).ok().and_then(Self::Value::new).ok_or_else(|| {
                    let unexpected = u64::try_from(integer)
                        .map_or(Unexpected::Other("u128"), Unexpected::Unsigned);
                    E::invalid_value(unexpected, &self)
                })
            }

            fn visit_i128<E: Error>(self, integer: i128) -> Result<Self::Value, E> {
                $int::try_from(integer).ok().and_then(Self::Value::new).ok_or_else(|| {
                    let unexpected = i64::try_from(integer)
                        .map_or(Unexpected::Other("i128"), Unexpected::Signed);
                    E::invalid_value(unexpected, &self)
                })
            }
        }
    };
}

each_ranged_integer!(ranged);
