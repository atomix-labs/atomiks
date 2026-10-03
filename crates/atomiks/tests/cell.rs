//! The fences, the spin hint and the cell, as production code reaches them.

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    use std::thread;

    use atomiks::cell::UnsafeCell;
    use atomiks::ordering::{Acquire, Relaxed, Release, SeqCst, StoreStore};
    use atomiks::{AtomicBool, AtomicU64, fence, hint};

    /// Rounds per fence test: one round under Miri catches a missing fence about half the time.
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

    #[test]
    fn the_windows_and_guards_reach_the_contents() {
        let cell = UnsafeCell::new(1_u64);
        // SAFETY: the closure's pointer is the only access to the cell while it runs.
        #[expect(unsafe_code, reason = "writing through the pointer the cell lends")]
        cell.with_mut(|value| unsafe { *value = 2 });
        // SAFETY: as above.
        #[expect(unsafe_code, reason = "reading through the pointer the cell lends")]
        let read = cell.with(|value| unsafe { *value });
        assert_eq!(read, 2, "the shared window sees the exclusive one's write");
        {
            let writer = cell.get_mut();
            // SAFETY: the cell lives in place, and the guard is its only access while it lives.
            #[expect(unsafe_code, reason = "writing through the guard the cell lends")]
            unsafe {
                *writer.deref() = 3;
            }
        }
        let seen = {
            let reader = cell.get();
            // SAFETY: the cell lives in place, and the guard is its only access while it lives.
            #[expect(unsafe_code, reason = "reading through the guard the cell lends")]
            unsafe {
                *reader.deref()
            }
        };
        assert_eq!(seen, 3, "the shared guard sees the exclusive one's write");
        assert_eq!(cell.into_inner(), 3, "and so does unwrapping the cell");
    }

    #[test]
    fn the_pointers_reach_the_contents() {
        let cell = UnsafeCell::new(1_u64);
        // SAFETY: the pointer is the live cell's own, and nothing else reaches the cell.
        #[expect(unsafe_code, reason = "writing through the address the cell gives")]
        unsafe {
            *cell.as_ptr() = 2;
        }
        let contents = UnsafeCell::raw_get(&raw const cell);
        assert_eq!(contents, cell.as_ptr(), "both name the contents");
        // SAFETY: as above.
        #[expect(unsafe_code, reason = "reading through the address the cell gives")]
        let read = unsafe { *contents };
        assert_eq!(read, 2, "the write through `as_ptr`");
    }
}
