//! Other crates' traits for atomiks' types, each module behind the feature named for its crate.
//!
//! zerocopy's are derived on the types themselves, since zerocopy allows no other impl.

#[cfg(feature = "bytemuck")]
mod bytemuck;
#[cfg(feature = "serde")]
mod serde;
