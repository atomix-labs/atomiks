//! Typed atomics for any value that fits one atomic word.

#![no_std]
#![feature(const_convert, const_destruct, const_trait_impl, impl_restriction, integer_casts)]
#![cfg_attr(not(loom), feature(const_atomic))]

#[cfg(all(loom, not(feature = "loom")))]
compile_error!(concat!(
    "atomiks: `--cfg loom` needs the `loom` feature: pass `--features atomiks/loom`, or depend \
     with `[target.'cfg(loom)'.dependencies] atomiks = { version = \"",
    env!("CARGO_PKG_VERSION"),
    "\", features = [\"loom\"] }`"
));

pub mod ordering;

mod primitive;
pub mod validity;

#[doc(hidden)]
pub use crate::primitive::{Bitwise, CellAccess, CellOps, FetchAdd};
pub use crate::primitive::{FetchBitwise, Integer, Load, MinMax, Primitive, Store, Swap};
