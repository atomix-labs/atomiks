//! Other crates' traits for atomiks' types, and `Atom` for other crates' values, each module behind
//! the feature named for its crate.
//!
//! zerocopy's are derived on the types themselves, since zerocopy allows no other impl.

#[cfg(feature = "arbitrary")]
mod arbitrary;
#[cfg(feature = "arbitrary-int")]
mod arbitrary_int;
#[cfg(feature = "bytemuck")]
mod bytemuck;
#[cfg(feature = "serde")]
mod serde;
