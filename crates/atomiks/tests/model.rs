//! Models of a derived packed struct's field operations under loom, through the seam: each is one
//! read-modify-write of loom's atomic, or `update`'s loop of them, so concurrent operations on
//! different fields all land, one test-and-set wins, and a bit set with Release publishes what came
//! before it.
//!
//! Each `should_panic` model is a bug loom must report; the rest hold under every interleaving.

// Loom's types exist only under `--cfg loom`; a crate-level `cfg` would empty the crate, and with
// it the attributes `just check-rust-lints` injects.
#[cfg(all(loom, feature = "derive"))]
#[cfg(test)]
mod tests {
    use atomiks::model::{Arc, check, thread};
    use atomiks::ordering::{AcqRel, Acquire, Relaxed, Release, RmwOrdering};
    use atomiks::{Atom, AtomBitwise, Atomic, AtomicU64, RangedU32};

    /// Flags, each pattern of a byte a value.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom, AtomBitwise)]
    struct Flags(u8);

    /// Flags and a bit, then a count in the top half of the word.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct Counted {
        flags: Flags,
        ready: bool,
        spare: RangedU32<0, 0x7F_FFFF>,
        count: u32,
    }

    /// The value each model starts from.
    const ZERO: Counted =
        Counted { flags: Flags(0), ready: false, spare: RangedU32::MIN, count: 0 };

    #[test]
    fn operations_on_different_fields_each_land() {
        check(|| {
            let counted = Arc::new(Atomic::new(ZERO));
            let others: Vec<_> = [Flags(0b01), Flags(0b10)]
                .into_iter()
                .map(|flag| {
                    let counted = Arc::clone(&counted);
                    thread::spawn(move || {
                        // ORDERING: Relaxed throughout: the join publishes each write.
                        counted.fields().count.fetch_add(1, Relaxed);
                        counted.fields().flags.or(flag, Relaxed);
                    })
                })
                .collect();
            counted.fields().ready.set(Relaxed);
            // The loop rebuilds the word from what it read: a stale read would drop a write.
            counted.fields().spare.update(Relaxed, Relaxed, |_| RangedU32::MAX);
            for other in others {
                other.join().expect("no thread panics");
            }
            let last = counted.load_rmw(Acquire);
            let expected =
                Counted { flags: Flags(0b11), ready: true, spare: RangedU32::MAX, count: 2 };
            assert_eq!(last, expected, "every operation landed");
        });
    }

    #[test]
    fn one_test_and_set_wins() {
        check(|| {
            let counted = Arc::new(Atomic::new(ZERO));
            let other = {
                let counted = Arc::clone(&counted);
                thread::spawn(move || counted.fields().ready.test_and_set(AcqRel))
            };
            let mine = counted.fields().ready.test_and_set(AcqRel);
            let theirs = other.join().expect("no thread panics");
            assert_ne!(mine, theirs, "exactly one finds the bit clear");
        });
    }

    /// A value published behind the bit `ready`, which the writer sets with `order`.
    fn publish<O: RmwOrdering + Send + Sync + 'static>(order: O) {
        check(move || {
            let shared = Arc::new((AtomicU64::new(0), Atomic::new(ZERO)));
            let writer = {
                let shared = Arc::clone(&shared);
                thread::spawn(move || {
                    shared.0.store(7, Relaxed);
                    shared.1.fields().ready.set(order);
                })
            };
            if shared.1.fields().ready.test_and_clear(Acquire) {
                assert_eq!(shared.0.load(Relaxed), 7, "the bit publishes the value");
            }
            writer.join().expect("the writer does not panic");
        });
    }

    #[test]
    fn a_release_set_publishes() {
        publish(Release);
    }

    #[test]
    #[should_panic(expected = "the bit publishes the value")]
    fn a_relaxed_set_does_not() {
        publish(Relaxed);
    }
}
