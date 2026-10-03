//! The fences, as production code reaches them.

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    use std::thread;

    use atomiks_core::ordering::{Acquire, Relaxed, Release, SeqCst, StoreStore};
    use atomiks_core::{AtomicBool, AtomicU64, fence, hint};

    /// Rounds per test: one round under Miri catches a missing fence about half the time.
    const ROUNDS: usize = 32;

    #[test]
    fn release_and_acquire_fences_publish_a_relaxed_store() {
        for _ in 0..ROUNDS {
            let (value, ready) = (AtomicU64::new(0), AtomicBool::new(false));
            thread::scope(|scope| {
                scope.spawn(|| {
                    value.store(7, Relaxed);
                    fence(Release);
                    ready.store(true, Relaxed);
                });
                while !ready.load(Relaxed) {
                    hint::spin_loop();
                }
                fence(Acquire);
                assert_eq!(value.load(Relaxed), 7, "the fences publish the value");
            });
        }
    }

    #[test]
    fn a_store_store_fence_keeps_a_relaxed_store_first() {
        for _ in 0..ROUNDS {
            let (value, ready) = (AtomicU64::new(0), AtomicBool::new(false));
            thread::scope(|scope| {
                scope.spawn(|| {
                    value.store(7, Relaxed);
                    fence(StoreStore);
                    ready.store(true, Relaxed);
                });
                while !ready.load(Acquire) {
                    hint::spin_loop();
                }
                assert_eq!(value.load(Relaxed), 7, "the fence keeps the value's store first");
            });
        }
    }

    #[test]
    fn seq_cst_fences_keep_two_threads_from_both_missing_the_other() {
        for _ in 0..ROUNDS {
            let (left, right) = (AtomicBool::new(false), AtomicBool::new(false));
            let (saw_left, saw_right) = thread::scope(|scope| {
                let other = scope.spawn(|| {
                    right.store(true, Relaxed);
                    fence(SeqCst);
                    left.load(Relaxed)
                });
                left.store(true, Relaxed);
                fence(SeqCst);
                let saw_right = right.load(Relaxed);
                (other.join().expect("the other thread does not panic"), saw_right)
            });
            assert!(saw_left || saw_right, "one fence comes first, and its thread's store is seen");
        }
    }
}
