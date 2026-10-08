//! A crate with no feature gate, under denied warnings, derives `Atom` for packed structs whose
//! parameters, a type, a constant and a lifetime, reach `PhantomData` markers alone, or no field:
//! each instance lays out alike, so none states a repr, and each projects its fields, the top one
//! adding in place.

#![deny(warnings, missing_docs)]

use core::marker::PhantomData;

use atomix::ordering::{AcqRel, Acquire, Release};
use atomix::{Atom, Atomic};

/// An order, which a handle names.
#[derive(Clone, Copy)]
pub struct Order;

/// A handle to a `T` in a slab: its slot, and the slot's generation, which counts its reuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct Handle<T> {
    /// The slot.
    pub index: u32,
    /// The slot's generation, at the top.
    pub generation: u32,
    /// What the slot holds.
    pub kind: PhantomData<fn() -> T>,
}

/// A batch of `N` lanes: its round, and how many lanes are filled, at the top.
#[derive(Clone, Copy, Atom)]
pub struct Batch<const N: usize> {
    /// The round it belongs to.
    pub round: u16,
    /// How many lanes are filled.
    pub filled: u16,
}

/// A cursor into a buffer that lives for `'a`: its offset, and whether it reached the end.
#[derive(Clone, Copy, Atom)]
pub struct Cursor<'a> {
    /// The offset.
    pub offset: u32,
    /// Whether it reached the end.
    pub done: bool,
    /// The buffer's lifetime.
    pub buffer: PhantomData<&'a [u8]>,
}

/// A fill on venue `V`, pending or of a quantity: an enum with fields, whose repr is stated.
#[derive(Clone, Copy, Atom)]
#[atom(repr = u16)]
pub enum Fill<V> {
    /// Not yet filled.
    Pending,
    /// Filled.
    Filled {
        /// How many.
        quantity: u8,
        /// The venue.
        venue: PhantomData<fn() -> V>,
    },
}

/// The handle of the order last placed.
pub static LAST_ORDER: Atomic<Handle<Order>> =
    Atomic::new(Handle { index: 3, generation: 0, kind: PhantomData });
/// The batch being filled.
pub static BATCH: Atomic<Batch<4>> = Atomic::new(Batch { round: 0, filled: 0 });
/// The cursor into a static buffer.
pub static CURSOR: Atomic<Cursor<'static>> =
    Atomic::new(Cursor { offset: 0, done: false, buffer: PhantomData });
/// The last order's fill.
pub static FILL: Atomic<Fill<Order>> = Atomic::new(Fill::Pending);

fn main() {
    // Each field changes through its place, the top ones adding in place.
    let _ = LAST_ORDER.fields().generation.fetch_add(1, AcqRel);
    let _ = BATCH.fields().filled.fetch_add(1, AcqRel);
    CURSOR.fields().done.set(Release);
    let _ = (CURSOR.fields().offset.load(Acquire), FILL.load(Acquire));
}
