//! Fences: core's, and the store-store fence core has no name for.

#[cfg(dmb_ishst)]
use core::arch::asm;
use core::sync::atomic::compiler_fence as core_compiler_fence;
#[cfg(not(loom))]
use core::sync::atomic::fence as atomic_fence;

#[cfg(loom)]
use loom::sync::atomic::fence as atomic_fence;

use crate::ordering::FenceOrdering;

/// Orders memory accesses across threads as `order` says.
///
/// [`StoreStore`](crate::ordering::StoreStore) orders this thread's earlier stores before its later
/// ones, and nothing else; a reader orders its own loads, with an `Acquire` load or fence. On
/// `aarch64` it is `dmb ishst`, which Rust's memory model has no name for: there it creates no
/// happens-before, so what it orders must be atomic (a plain write would race), and only the
/// hardware orders it. Everywhere else, and under loom and Miri, it is a `Release` fence, which on
/// `x86_64` emits no instruction; it also orders earlier loads and publishes plain writes, so
/// neither checker reports code that needs either from it. The other orderings are `core`'s fence.
#[inline]
pub fn fence<O: FenceOrdering>(order: O) {
    let _ = order;
    #[cfg(dmb_ishst)]
    if O::IS_STORE_STORE {
        store_store();
        return;
    }
    atomic_fence(O::CORE_FENCE);
}

/// Orders memory accesses within this thread only: no instruction, only the compiler's order, so
/// nothing between threads, and nothing loom models.
#[inline]
pub fn compiler_fence<O: FenceOrdering>(order: O) {
    let _ = order;
    core_compiler_fence(O::CORE_FENCE);
}

/// The store-store fence on `aarch64`.
#[cfg(dmb_ishst)]
#[expect(unsafe_code, reason = "`dmb ishst`, which no core function emits")]
#[inline]
fn store_store() {
    // SAFETY: a barrier: it reads and writes no memory, touches no stack and keeps the flags;
    // without `nomem` it is also a compiler barrier, which the fence must be.
    unsafe {
        asm!("dmb ishst", options(nostack, preserves_flags));
    }
}
