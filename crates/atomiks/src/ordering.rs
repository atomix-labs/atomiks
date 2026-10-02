//! Memory orderings as types: each operation names the orderings it accepts, so a wrong one is a
//! compile error rather than a panic.
//!
//! Only the types here implement the traits.

use core::sync::atomic::Ordering as CoreOrdering;

/// Orders nothing beyond the operation's own atomicity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Relaxed;
/// Sees every write published by the [`Release`] it reads from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Acquire;
/// Publishes every earlier write to the [`Acquire`] that reads it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Release;
/// [`Acquire`] and [`Release`] at once, for a read-modify-write or a fence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct AcqRel;
/// [`Acquire`], [`Release`] or both, as the operation allows, in one order of every `SeqCst`
/// operation and fence that all threads agree on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SeqCst;
/// A fence that orders earlier stores before later ones, and nothing else.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct StoreStore;

/// An ordering a read-modify-write may take.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an ordering",
    label = "expected `Relaxed`, `Acquire`, `Release`, `AcqRel` or `SeqCst` from `atomiks::ordering`"
)]
pub impl(crate) trait Ordering: Copy {
    /// The ordering as `core` (and loom) spell it.
    #[doc(hidden)]
    const CORE: CoreOrdering;
}

/// An ordering a load may take; also a compare-exchange's failure ordering.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a load ordering",
    label = "expected `Relaxed`, `Acquire` or `SeqCst` from `atomiks::ordering`",
    note = "`Release` and `AcqRel` order a store, and a load has none"
)]
pub impl(crate) trait LoadOrdering: Ordering {}

/// An ordering a store may take.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a store ordering",
    label = "expected `Relaxed`, `Release` or `SeqCst` from `atomiks::ordering`",
    note = "`Acquire` and `AcqRel` order a load, and a store has none"
)]
pub impl(crate) trait StoreOrdering: Ordering {}

/// An ordering a fence may take.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a fence ordering",
    label = "expected `Acquire`, `Release`, `AcqRel`, `SeqCst` or `StoreStore` from `atomiks::ordering`",
    note = "a `Relaxed` fence orders nothing"
)]
pub impl(crate) trait FenceOrdering: Copy {
    /// The `core` fence's ordering that stands for this one: [`StoreStore`]'s is `Release`, the
    /// narrowest that covers it.
    #[doc(hidden)]
    const CORE_FENCE: CoreOrdering;
    /// Whether `fence` emits the store-store barrier rather than `CORE_FENCE`'s.
    #[doc(hidden)]
    const IS_STORE_STORE: bool = false;
}

/// Implements `$trait` for each ordering, with `$core` the `core` ordering of the same name.
macro_rules! core_orderings {
    ($trait:ident::$core:ident: $($ordering:ident),+) => {$(
        impl $trait for $ordering {
            const $core: CoreOrdering = CoreOrdering::$ordering;
        }
    )+};
}

core_orderings!(Ordering::CORE: Relaxed, Acquire, Release, AcqRel, SeqCst);
core_orderings!(FenceOrdering::CORE_FENCE: Acquire, Release, AcqRel, SeqCst);

impl FenceOrdering for StoreStore {
    const CORE_FENCE: CoreOrdering = CoreOrdering::Release;
    const IS_STORE_STORE: bool = true;
}

// Written out, not generated: a wrong ordering's error lists these impls, and would show a macro.
impl LoadOrdering for Relaxed {}
impl LoadOrdering for Acquire {}
impl LoadOrdering for SeqCst {}

impl StoreOrdering for Relaxed {}
impl StoreOrdering for Release {}
impl StoreOrdering for SeqCst {}

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicU8, compiler_fence, fence};

    use super::{
        AcqRel, Acquire, CoreOrdering, FenceOrdering, LoadOrdering, Relaxed, Release, SeqCst,
        StoreOrdering, StoreStore,
    };

    /// Loads with `O`, and fails a compare-exchange with it, on `core`'s atomic, which panics on an
    /// ordering a load refuses.
    fn load_on_core<O: LoadOrdering>() {
        let atomic = AtomicU8::new(1);
        assert_eq!(atomic.load(O::CORE), 1, "the load reads the value");
        assert_eq!(
            atomic.compare_exchange(0, 2, CoreOrdering::SeqCst, O::CORE),
            Err(1),
            "the compare-exchange fails and reads the value"
        );
    }

    /// Stores with `O` on `core`'s atomic, which panics on an ordering a store refuses.
    fn store_on_core<O: StoreOrdering>() {
        let atomic = AtomicU8::new(0);
        atomic.store(1, O::CORE);
        assert_eq!(atomic.into_inner(), 1, "the store writes the value");
    }

    /// Fences with `O`'s `core` ordering, which `core` panics on if a fence refuses it.
    fn fence_on_core<O: FenceOrdering>() {
        fence(O::CORE_FENCE);
        compiler_fence(O::CORE_FENCE);
    }

    #[test]
    fn core_accepts_each_ordering_an_operation_admits() {
        load_on_core::<Relaxed>();
        load_on_core::<Acquire>();
        load_on_core::<SeqCst>();
        store_on_core::<Relaxed>();
        store_on_core::<Release>();
        store_on_core::<SeqCst>();
        fence_on_core::<Acquire>();
        fence_on_core::<Release>();
        fence_on_core::<AcqRel>();
        fence_on_core::<SeqCst>();
        fence_on_core::<StoreStore>();
    }
}
