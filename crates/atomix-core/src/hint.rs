//! The spin hint: core's, or under loom loom's, which yields to the model's scheduler so a spin
//! cannot stall the search.

#[cfg(not(loom))]
pub use core::hint::spin_loop;

#[cfg(loom)]
pub use loom::hint::spin_loop;
