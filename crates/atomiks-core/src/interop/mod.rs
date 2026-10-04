//! Other crates' traits for atomiks' types, and `Atom` for other crates' values, each module behind
//! its crate's feature.
//!
//! zerocopy's are derived on the types themselves, since zerocopy allows no other impl.

#[cfg(feature = "arbitrary")]
mod arbitrary;
#[cfg(feature = "arbitrary-int")]
mod arbitrary_int;
#[cfg(feature = "bytemuck")]
mod bytemuck;
#[cfg(feature = "deranged-05")]
mod deranged;
#[cfg(feature = "serde")]
mod serde;
