//! Typed atomics for any value that fits one atomic word.

#![no_std]
#![feature(impl_restriction)]

#[cfg(all(loom, not(feature = "loom")))]
compile_error!(concat!(
    "atomiks: `--cfg loom` needs the `loom` feature: pass `--features atomiks/loom`, or depend \
     with `[target.'cfg(loom)'.dependencies] atomiks = { version = \"",
    env!("CARGO_PKG_VERSION"),
    "\", features = [\"loom\"] }`"
));

pub mod ordering;
