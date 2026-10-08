//! Models through the seam, where loom sees atomix's atomics, fences and cells.
//!
//! Each `should_panic` model is a bug loom must report; the rest hold under every interleaving.

#![feature(const_trait_impl)]

// Loom's types exist only under `--cfg loom`; a crate-level `cfg` would empty the crate, and with
// it the attributes `just check-rust-lints` injects.
#[cfg(loom)]
#[cfg(test)]
mod testing;

#[cfg(loom)]
#[cfg(test)]
mod tests {
    use core::ptr;

    use atomix_core::cell::UnsafeCell;
    use atomix_core::model::{Arc, check, thread};
    use atomix_core::ordering::{AcqRel, Acquire, Relaxed, Release, StoreOrdering, StoreStore};
    use atomix_core::{Atomic, AtomicBool, AtomicPtr, AtomicU64, fence, hint};

    use crate::testing::pointer_word::pointer_word;

    /// A value published behind a flag the writer stores with `order`.
    fn publish<O: StoreOrdering + Send + Sync + 'static>(order: O) {
        check(move || {
            let shared = Arc::new((AtomicU64::new(0), AtomicBool::new(false)));
            let writer = {
                let shared = Arc::clone(&shared);
                thread::spawn(move || {
                    shared.0.store(7, Relaxed);
                    shared.1.store(true, order);
                })
            };
            if shared.1.load(Acquire) {
                assert_eq!(shared.0.load(Relaxed), 7, "the flag publishes the value");
            }
            writer.join().expect("the writer does not panic");
        });
    }

    #[test]
    fn a_release_store_publishes() {
        publish(Release);
    }

    #[test]
    #[should_panic(expected = "the flag publishes the value")]
    fn a_relaxed_store_does_not() {
        publish(Relaxed);
    }

    /// A value published behind a flag `raise` sets with a compare-exchange loop.
    ///
    /// The reader uses `load_rmw`: loom 0.7.2 wrongly synchronizes a plain load with a Relaxed
    /// write that follows its writer's own load of the same atomic, as every loop's write does.
    fn publish_through_a_loop<W: Fn(&AtomicBool) + Copy + Send + Sync + 'static>(raise: W) {
        check(move || {
            let shared = Arc::new((AtomicU64::new(0), AtomicBool::new(false)));
            let writer = {
                let shared = Arc::clone(&shared);
                thread::spawn(move || {
                    shared.0.store(7, Relaxed);
                    raise(&shared.1);
                })
            };
            if shared.1.load_rmw(Acquire) {
                assert_eq!(shared.0.load(Relaxed), 7, "the flag publishes the value");
            }
            writer.join().expect("the writer does not panic");
        });
    }

    #[test]
    fn a_release_update_publishes() {
        publish_through_a_loop(|flag| {
            assert!(!flag.update(Release, Relaxed, |_| true), "nothing else raises the flag");
        });
    }

    #[test]
    fn a_release_try_update_publishes() {
        publish_through_a_loop(|flag| {
            let raised = flag.try_update(Release, Relaxed, |_| Some(true));
            assert_eq!(raised, Ok(false), "nothing else raises the flag");
        });
    }

    #[test]
    fn a_release_store_rmw_publishes() {
        publish_through_a_loop(|flag| flag.store_rmw(true, Release));
    }

    #[test]
    fn concurrent_updates_never_lose_one() {
        check(|| {
            let count = Arc::new(AtomicU64::new(0));
            let other = {
                let count = Arc::clone(&count);
                thread::spawn(move || count.update(AcqRel, Acquire, |seen| seen + 1))
            };
            let _ = count.update(AcqRel, Acquire, |seen| seen + 1);
            other.join().expect("the other updater does not panic");
            assert_eq!(count.load(Acquire), 2, "both increments landed");
        });
    }

    #[test]
    fn try_update_lets_exactly_one_of_two_claims_land() {
        check(|| {
            let owner = Arc::new(AtomicU64::new(0));
            let other = {
                let owner = Arc::clone(&owner);
                thread::spawn(move || {
                    owner.try_update(AcqRel, Acquire, |seen| (seen == 0).then_some(2))
                })
            };
            let mine = owner.try_update(AcqRel, Acquire, |seen| (seen == 0).then_some(1));
            let theirs = other.join().expect("the other claimant does not panic");
            let (winner, won, lost) =
                if mine.is_ok() { (1, mine, theirs) } else { (2, theirs, mine) };
            assert_eq!(won, Ok(0), "the claim that landed saw the owner free");
            assert_eq!(owner.load(Acquire), winner, "the owner is that claim's");
            assert_eq!(lost, Err(winner), "the other claim saw it and declined");
        });
    }

    #[test]
    fn a_try_update_that_loses_the_race_retries_on_what_it_found() {
        check(|| {
            let seats = Arc::new(AtomicU64::new(2));
            let book = |left: u64| left.checked_sub(1);
            let other = {
                let seats = Arc::clone(&seats);
                thread::spawn(move || seats.try_update(AcqRel, Acquire, book))
            };
            let mine = seats.try_update(AcqRel, Acquire, book);
            let theirs = other.join().expect("the other booking does not panic");
            let mut seen = [mine, theirs];
            seen.sort_unstable();
            assert_eq!(
                seen,
                [Ok(1), Ok(2)],
                "one saw two seats, and the other the one left, after a retry if it lost the race"
            );
            assert_eq!(seats.load(Acquire), 0, "and the two bookings take both seats");
        });
    }

    #[test]
    fn concurrent_pointer_offsets_never_lose_one() {
        check(|| {
            let base = ptr::null_mut::<u64>();
            let cursor = Arc::new(AtomicPtr::new(base));
            let other = {
                let cursor = Arc::clone(&cursor);
                thread::spawn(move || {
                    cursor.fetch_ptr_add(3, AcqRel);
                    cursor.fetch_byte_sub(8, AcqRel);
                })
            };
            let before_add = cursor.fetch_byte_add(16, AcqRel);
            let before_sub = cursor.fetch_ptr_sub(1, AcqRel);
            other.join().expect("the other thread does not panic");
            // `elements` past `base`, plus what the other thread has added by then: 0, 3 or 2.
            let reachable = |elements: usize| {
                [0, 3, 2].map(|other| base.wrapping_add(elements).wrapping_add(other))
            };
            assert!(
                reachable(0).contains(&before_add),
                "fetch_byte_add returns the pointer before"
            );
            assert!(reachable(2).contains(&before_sub), "fetch_ptr_sub returns the pointer before");
            assert_eq!(
                cursor.load(Acquire),
                base.wrapping_add(3),
                "every offset lands: 3 - 1 + 2 - 1"
            );
        });
    }

    pointer_word! {
        /// Two tags below an address no model reads through.
        struct Tagged, projected as TaggedFields {
            0 => pointer: *mut u64,
            1 => low: bool,
            2 => high: bool,
        }
    }

    /// Loom's pointer cell has no bitwise operation, so each tag operation is a compare-exchange
    /// loop: one that loses a race retries on what it found, keeping the other's bit and the
    /// address.
    #[test]
    fn concurrent_tag_operations_never_lose_one() {
        check(|| {
            let base = ptr::without_provenance_mut::<u64>(0x1000);
            let word = Arc::new(Atomic::new(Tagged { pointer: base, low: false, high: false }));
            let other = {
                let word = Arc::clone(&word);
                thread::spawn(move || word.fields().low.set(AcqRel))
            };
            let was_set = word.fields().high.test_and_set(AcqRel);
            other.join().expect("the other thread does not panic");
            assert!(!was_set, "the high bit clear before its test-and-set");
            let last = Tagged { pointer: base, low: true, high: true };
            assert_eq!(word.load(Acquire), last, "both bits set, and the address kept");
        });
    }

    #[test]
    fn a_release_pointer_offset_publishes() {
        check(|| {
            let shared = Arc::new((AtomicU64::new(0), AtomicPtr::new(ptr::null_mut::<u64>())));
            let writer = {
                let shared = Arc::clone(&shared);
                thread::spawn(move || {
                    shared.0.store(7, Relaxed);
                    shared.1.fetch_byte_add(8, Release);
                })
            };
            // An offset of zero reads the pointer with an exchange, as `publish_through_a_loop`'s
            // reader does.
            if !shared.1.fetch_byte_add(0, Acquire).is_null() {
                assert_eq!(shared.0.load(Relaxed), 7, "the offset publishes the value");
            }
            writer.join().expect("the writer does not panic");
        });
    }

    #[test]
    fn exclusive_access_writes_what_an_atomic_load_reads() {
        check(|| {
            let mut count = AtomicU64::new(1);
            count.set(2);
            count.with_mut(|seen| *seen += 1);
            assert_eq!(count.get(), 3, "get reads what set and with_mut wrote");
            let mut flag = AtomicBool::new(false);
            flag.set(true);
            assert!(flag.get(), "get reads the bool set wrote");
            let count = Arc::new(count);
            let reader = {
                let count = Arc::clone(&count);
                thread::spawn(move || count.load(Relaxed))
            };
            let seen = reader.join().expect("the reader does not panic");
            assert_eq!(seen, 3, "the spawn publishes it");
            let Ok(count) = Arc::try_unwrap(count) else { panic!("the reader has ended") };
            assert_eq!(count.into_inner(), 3, "into_inner reads it too");
        });
    }

    #[test]
    fn a_store_store_fence_publishes_as_a_release_fence() {
        check(|| {
            let shared = Arc::new((AtomicU64::new(0), AtomicBool::new(false)));
            let writer = {
                let shared = Arc::clone(&shared);
                thread::spawn(move || {
                    shared.0.store(7, Relaxed);
                    fence(StoreStore);
                    shared.1.store(true, Relaxed);
                })
            };
            if shared.1.load(Acquire) {
                assert_eq!(shared.0.load(Relaxed), 7, "the fence publishes the value");
            }
            writer.join().expect("the writer does not panic");
        });
    }

    #[test]
    fn a_spin_wait_yields_to_the_writer() {
        check(|| {
            let ready = Arc::new(AtomicBool::new(false));
            let writer = {
                let ready = Arc::clone(&ready);
                thread::spawn(move || ready.store(true, Release))
            };
            while !ready.load(Acquire) {
                hint::spin_loop();
            }
            writer.join().expect("the writer does not panic");
        });
    }

    /// A write and a read through loom's cell guards: the calls `tests/cell.rs` makes off loom.
    ///
    /// The calls are the same, `get_mut`, `get` and `deref`, so the two cannot drift apart unseen.
    #[test]
    fn a_release_flag_publishes_a_write_through_a_guard() {
        check(|| {
            let shared = Arc::new((UnsafeCell::new(0_u64), AtomicBool::new(false)));
            let writer = {
                let shared = Arc::clone(&shared);
                thread::spawn(move || {
                    {
                        let guard = shared.0.get_mut();
                        // SAFETY: the reader reaches the cell only after the flag, which this
                        // thread stores once the guard has ended.
                        #[expect(unsafe_code, reason = "writing through the guard the cell lends")]
                        unsafe {
                            *guard.deref() = 7;
                        }
                    }
                    shared.1.store(true, Release);
                })
            };
            if shared.1.load(Acquire) {
                let guard = shared.0.get();
                // SAFETY: the flag says the writer's guard has ended, and nothing writes after it.
                #[expect(unsafe_code, reason = "reading through the guard the cell lends")]
                let seen = unsafe { *guard.deref() };
                assert_eq!(seen, 7, "the flag publishes the write");
            }
            writer.join().expect("the writer does not panic");
        });
    }

    #[test]
    #[should_panic(expected = "Causality violation: Concurrent write accesses to `UnsafeCell`")]
    fn unsynchronized_cell_writes_are_reported() {
        check(|| {
            let cell = Arc::new(UnsafeCell::new(0_u64));
            let other = {
                let cell = Arc::clone(&cell);
                // SAFETY: none: the model exists to report this race.
                #[expect(unsafe_code, reason = "the race the model must report")]
                thread::spawn(move || cell.with_mut(|value| unsafe { *value = 1 }))
            };
            // SAFETY: none: the model exists to report this race.
            #[expect(unsafe_code, reason = "the race the model must report")]
            cell.with_mut(|value| unsafe { *value = 2 });
            other.join().expect("the other writer does not panic");
        });
    }

    /// A double word: two pointers, or a slice's pointer and length, in one 16-byte atomic.
    #[cfg(wide)]
    mod double {
        use core::ptr::NonNull;

        use atomix_core::Atomic;
        use atomix_core::model::{Arc, check, thread};
        use atomix_core::ordering::{AcqRel, Acquire, Relaxed, Release, RmwOrdering};

        use super::AtomicU64;

        /// What the double words point to: never written, so shared without a lock.
        static NODES: [u64; 4] = [10, 11, 12, 13];

        /// A pointer to the node at `index`.
        fn node(index: usize) -> NonNull<u64> {
            NonNull::from(&NODES[index])
        }

        /// What the node `pointer` points to holds, read through it.
        fn read(pointer: NonNull<u64>) -> u64 {
            // SAFETY: every node is a static's, never written.
            #[expect(unsafe_code, reason = "reads through a pointer a pair held")]
            let value = unsafe { pointer.as_ref() };
            *value
        }

        /// A value published behind a pair of pointers an exchange of `order` writes.
        fn publish<O: RmwOrdering + Send + Sync + 'static>(order: O) {
            check(move || {
                let shared = Arc::new((AtomicU64::new(0), Atomic::new((node(0), node(0)))));
                let writer = {
                    let shared = Arc::clone(&shared);
                    thread::spawn(move || {
                        shared.0.store(7, Relaxed);
                        let published = shared.1.compare_exchange(
                            (node(0), node(0)),
                            (node(1), node(2)),
                            order,
                            Relaxed,
                        );
                        assert_eq!(
                            published,
                            Ok((node(0), node(0))),
                            "nothing else writes the pair"
                        );
                    })
                };
                let seen = shared.1.load_rmw(Acquire);
                if seen == (node(1), node(2)) {
                    assert_eq!(shared.0.load(Relaxed), 7, "the pair publishes the value");
                    let read_through = (read(seen.0), read(seen.1));
                    assert_eq!(read_through, (11, 12), "each pointer reads its node");
                }
                writer.join().expect("the writer does not panic");
            });
        }

        #[test]
        fn a_release_pair_exchange_publishes() {
            publish(Release);
        }

        #[test]
        #[should_panic(expected = "the pair publishes the value")]
        fn a_relaxed_pair_exchange_does_not() {
            publish(Relaxed);
        }

        /// Two threads each lengthen a slice pointer by one element: neither update is lost.
        #[test]
        fn concurrent_slice_updates_never_lose_one() {
            check(|| {
                let lengthen = |seen: NonNull<[u64]>| NonNull::from(&NODES[..=seen.len()]);
                let slice = Arc::new(Atomic::new(NonNull::from(&NODES[..1])));
                let other = {
                    let slice = Arc::clone(&slice);
                    thread::spawn(move || {
                        let _ = slice.update(AcqRel, Acquire, lengthen);
                    })
                };
                let _ = slice.update(AcqRel, Acquire, lengthen);
                other.join().expect("the other updater does not panic");
                assert_eq!(slice.load_rmw(Acquire).len(), 3, "both updates landed");
            });
        }
    }

    #[cfg(wide)]
    mod wide {
        use core::sync::atomic::{AtomicU64 as CoreAtomicU64, Ordering as CoreOrdering};

        use atomix_core::AtomicU128;
        use atomix_core::model::{Arc, check, thread};
        use atomix_core::ordering::{AcqRel, Acquire, Relaxed, Release, SeqCst};

        use super::{AtomicBool, AtomicU64, UnsafeCell};

        const HIGH: u128 = 1 << 100;

        /// A value published behind a 128-bit word `raise` writes.
        fn publish<W: Fn(&AtomicU128) + Copy + Send + Sync + 'static>(raise: W) {
            check(move || {
                let shared = Arc::new((AtomicU64::new(0), AtomicU128::new(0)));
                let writer = {
                    let shared = Arc::clone(&shared);
                    thread::spawn(move || {
                        shared.0.store(7, Relaxed);
                        raise(&shared.1);
                    })
                };
                if shared.1.load_rmw(Acquire) == HIGH {
                    assert_eq!(shared.0.load(Relaxed), 7, "the wide word publishes the value");
                }
                writer.join().expect("the writer does not panic");
            });
        }

        #[test]
        fn a_release_exchange_publishes() {
            publish(|word| {
                let published = word.compare_exchange(0, HIGH, Release, Relaxed);
                assert_eq!(published, Ok(0), "nothing else writes the wide word");
            });
        }

        #[test]
        #[should_panic(expected = "the wide word publishes the value")]
        fn a_relaxed_exchange_does_not() {
            publish(|word| {
                let published = word.compare_exchange(0, HIGH, Relaxed, Relaxed);
                assert_eq!(published, Ok(0), "nothing else writes the wide word");
            });
        }

        #[test]
        fn a_release_store_rmw_publishes() {
            publish(|word| word.store_rmw(HIGH, Release));
        }

        /// A wide `update`'s first read carries `fetch_order` (off loom, without `Load`, it is a
        /// compare-exchange; under loom, a load), so `f` may read what was written before the value
        /// it sees.
        #[test]
        fn a_wide_update_first_reads_with_its_fetch_order() {
            check(|| {
                let cell = Arc::new(UnsafeCell::new(0_u64));
                let word = Arc::new(AtomicU128::new(0));
                let writer = {
                    let (cell, word) = (Arc::clone(&cell), Arc::clone(&word));
                    thread::spawn(move || {
                        // SAFETY: the reader reaches the cell only once the word holds `HIGH`,
                        // which this thread writes after its last access to the cell.
                        #[expect(unsafe_code, reason = "writing a cell the word publishes")]
                        cell.with_mut(|value| unsafe { *value = 7 });
                        // One write and no read before it: after a read, loom 0.7.2 would hide
                        // a missing acquire, as `publish_through_a_loop`'s doc says.
                        let published = word.compare_exchange(0, HIGH, Release, Relaxed);
                        assert_eq!(published, Ok(0), "nothing else changes the wide word");
                    })
                };
                let _ = word.update(AcqRel, Acquire, |seen| {
                    if seen == HIGH {
                        // SAFETY: `seen` is `HIGH`, so the writer is done with the cell.
                        #[expect(unsafe_code, reason = "reading a cell the word published")]
                        let value = cell.with(|value| unsafe { *value });
                        assert_eq!(
                            value, 7,
                            "the read that sees `HIGH` acquires the writer's write"
                        );
                    }
                    seen
                });
                writer.join().expect("the writer does not panic");
            });
        }

        #[test]
        fn concurrent_wide_updates_never_lose_one() {
            check(|| {
                let count = Arc::new(AtomicU128::new(HIGH));
                let other = {
                    let count = Arc::clone(&count);
                    thread::spawn(move || count.update(AcqRel, Acquire, |seen| seen + 1))
                };
                let _ = count.update(AcqRel, Acquire, |seen| seen + 1);
                other.join().expect("the other updater does not panic");
                assert_eq!(count.load_rmw(Acquire), HIGH + 2, "both increments landed");
            });
        }

        /// A strong compare-exchange racing the store of its `current`.
        ///
        /// The table of 128-bit values outlives each execution, so each takes a `current` no
        /// earlier one interned, which either thread may intern first.
        #[test]
        fn a_strong_wide_exchange_never_fails_spuriously() {
            /// The executions so far, counted outside the model.
            static EXECUTIONS: CoreAtomicU64 = CoreAtomicU64::new(0);
            check(|| {
                // Down from `u128::MAX`, which no other model writes.
                let run = EXECUTIONS.fetch_add(1, CoreOrdering::Relaxed);
                let current = u128::MAX.wrapping_sub(u128::from(run));
                let word = Arc::new(AtomicU128::new(0));
                let writer = {
                    let word = Arc::clone(&word);
                    thread::spawn(move || word.store_rmw(current, SeqCst))
                };
                if let Err(found) = word.compare_exchange(current, 1, SeqCst, SeqCst) {
                    assert_ne!(
                        found, current,
                        "a strong compare-exchange fails only on another value"
                    );
                }
                writer.join().expect("the writer does not panic");
            });
        }

        #[test]
        fn exclusive_access_writes_what_a_wide_exchange_reads() {
            check(|| {
                let mut value = AtomicU128::new(1);
                value.set(HIGH);
                value.with_mut(|seen| *seen += 1);
                assert_eq!(value.get(), HIGH + 1, "get reads what set and with_mut wrote");
                let value = Arc::new(value);
                let reader = {
                    let value = Arc::clone(&value);
                    thread::spawn(move || value.load_rmw(Relaxed))
                };
                let seen = reader.join().expect("the reader does not panic");
                assert_eq!(seen, HIGH + 1, "the spawn publishes it");
                let Ok(value) = Arc::try_unwrap(value) else { panic!("the reader has ended") };
                assert_eq!(value.into_inner(), HIGH + 1, "into_inner reads it too");
            });
        }

        /// A race the table of 128-bit values must not hide.
        ///
        /// A plain write, then a Relaxed flag, with a 128-bit atomic built in each thread: were the
        /// table loom's, building one would order the reader after the writer.
        #[test]
        #[should_panic(expected = "Causality violation: Concurrent read and write accesses")]
        fn building_a_wide_atomic_publishes_nothing() {
            check(|| {
                let cell = Arc::new(UnsafeCell::new(0_u64));
                let flag = Arc::new(AtomicBool::new(false));
                let writer = {
                    let (cell, flag) = (Arc::clone(&cell), Arc::clone(&flag));
                    thread::spawn(move || {
                        // SAFETY: none: the model exists to report this race.
                        #[expect(unsafe_code, reason = "the race the model must report")]
                        cell.with_mut(|value| unsafe { *value = 1 });
                        let _built = AtomicU128::new(1);
                        flag.store(true, Relaxed);
                    })
                };
                if flag.load(Relaxed) {
                    let _built = AtomicU128::new(2);
                    // SAFETY: none: the model exists to report this race.
                    #[expect(unsafe_code, reason = "the race the model must report")]
                    let _ = cell.with(|value| unsafe { *value });
                }
                writer.join().expect("the writer does not panic");
            });
        }
    }
}
