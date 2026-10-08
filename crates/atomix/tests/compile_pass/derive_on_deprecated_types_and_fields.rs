//! A crate that denies warnings derives `Atom` for a deprecated packed struct, generic packed
//! struct, enum with fields, one whose discriminant names `Self`, fieldless enum, zero-width
//! struct, pointer word and pointer enum, `Atom` and `AtomAdd` for a deprecated newtype, and `Atom`
//! for structs with a deprecated field, and for a deprecated packed struct, newtype and pointer word
//! a `macro_rules!` declares, as std's derives do: the derive's own uses of a deprecated type or
//! field, and of the deprecated place it writes for one, warn nothing.

#![deny(warnings)]
// The generic `Couple` converts its fields through `Atom`'s methods.
#![feature(const_trait_impl)]

use core::ptr::NonNull;

use atomix::{Atom, AtomAdd};

/// A quote: a packed struct, and its projection.
#[deprecated = "use the quote of two prices"]
#[derive(Clone, Copy, Atom)]
pub struct Quote {
    /// The quantity.
    pub quantity: u32,
    /// Whether it may fill.
    pub live: bool,
}

/// A slot, empty or full: an enum with fields.
#[deprecated]
#[derive(Clone, Copy, Atom)]
pub enum Slot {
    /// Nothing.
    Empty,
    /// A value.
    Full(u32),
}

/// A side of the book: a fieldless enum.
#[deprecated]
#[derive(Clone, Copy, Atom)]
pub enum Side {
    /// A bid.
    Bid,
    /// An ask.
    Ask,
}

/// A sequence number: a newtype, and its capability.
#[deprecated]
#[derive(Clone, Copy, Atom, AtomAdd)]
pub struct Seq(pub u64);

/// A stack's head: a pointer word.
#[deprecated]
#[derive(Clone, Copy, Atom)]
pub struct Head {
    /// The top node.
    pub top: Option<NonNull<u64>>,
    /// Whether it is closed.
    pub closed: bool,
}

/// Two values: a generic packed struct.
#[deprecated]
#[derive(Clone, Copy, Atom)]
#[atom(repr = u64)]
pub struct Couple<A, B> {
    /// The first.
    pub first: A,
    /// The second.
    pub second: B,
}

/// A level whose discriminant names `Self`: an enum with fields.
#[deprecated]
#[derive(Clone, Copy, Atom)]
#[repr(u8)]
pub enum Level {
    /// The base.
    Base = Self::BASE,
    /// Above it, by a byte.
    Above(u8),
}

#[expect(deprecated, reason = "the constant the discriminant names is the deprecated enum's own")]
impl Level {
    /// The base's discriminant.
    const BASE: u8 = 4;
}

/// A closed flag: a zero-width struct.
#[deprecated]
#[derive(Clone, Copy, Atom)]
pub struct Closed;

/// The next node, or none: a pointer enum.
#[deprecated]
#[derive(Clone, Copy, Atom)]
pub enum Next {
    /// The end.
    End,
    /// A node.
    Node(NonNull<u64>),
}

/// An order, whose deprecated field's place is deprecated too.
#[derive(Clone, Copy, Atom)]
pub struct Order {
    /// Whether it may fill.
    #[deprecated = "read `state`"]
    pub live: bool,
    /// Its state.
    pub state: u8,
}

/// A pair of a flag and a count, whose flag is deprecated: a tuple struct.
#[derive(Clone, Copy, Atom)]
pub struct Pair(#[deprecated] pub bool, pub u8);

/// Declares a deprecated quote, sequence number and stack's head, whose field names the derive
/// reads from the macro's expansion.
macro_rules! deprecated_types {
    () => {
        /// A quote a macro declares: a packed struct.
        #[deprecated]
        #[derive(Clone, Copy, Atom)]
        pub struct DeclaredQuote {
            /// The quantity.
            pub quantity: u8,
            /// Whether it may fill.
            pub live: bool,
        }

        /// A sequence number a macro declares: a newtype.
        #[deprecated]
        #[derive(Clone, Copy, Atom)]
        pub struct DeclaredSeq(pub u64);

        /// A stack's head a macro declares: a pointer word.
        #[deprecated]
        #[derive(Clone, Copy, Atom)]
        pub struct DeclaredHead {
            /// The top node.
            pub top: Option<NonNull<u64>>,
            /// Whether it is closed.
            pub closed: bool,
        }
    };
}

deprecated_types!();

fn main() {}
