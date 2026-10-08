//! `Atomic<T>`'s operations, over the built-in values.

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    use core::fmt::Write;
    use core::panic::{AssertUnwindSafe, RefUnwindSafe, UnwindSafe};
    use std::panic;

    use atomix_core::ordering::{AcqRel, Acquire, Relaxed, Release, SeqCst};
    use atomix_core::{Atom, Atomic, AtomicBool, AtomicI64, AtomicU8, AtomicU64};

    static COUNT: AtomicU64 = AtomicU64::new(5);

    #[test]
    fn a_static_is_built_at_compile_time() {
        assert_eq!(COUNT.load(Acquire), 5, "the const initializer");
    }

    #[test]
    fn an_atomic_is_the_size_and_alignment_of_its_repr() {
        assert_eq!(size_of::<AtomicU64>(), 8, "no wrapper overhead");
        assert_eq!(align_of::<AtomicU64>(), 8, "no extra alignment");
        assert_eq!(size_of::<AtomicBool>(), 1, "a bool is a byte");
    }

    #[test]
    fn an_option_of_an_atomic_borrows_no_niche_from_its_value() {
        assert_eq!(size_of::<Option<AtomicBool>>(), 2, "no niche borrowed from a live value");
    }

    #[test]
    fn a_store_is_what_the_next_load_reads() {
        let flag = AtomicBool::new(false);
        flag.store(true, Release);
        assert!(flag.load(Acquire), "the stored value");
    }

    #[test]
    fn swap_returns_the_value_before() {
        let value = AtomicI64::new(-3);
        assert_eq!(value.swap(4, AcqRel), -3, "the value before");
        assert_eq!(value.load(Relaxed), 4, "and the new one after");
    }

    #[test]
    fn compare_exchange_writes_only_over_the_expected_value() {
        let value = AtomicU64::new(1);
        assert_eq!(value.compare_exchange(2, 3, AcqRel, Acquire), Err(1), "not 2: the value seen");
        assert_eq!(value.compare_exchange(1, 3, AcqRel, Acquire), Ok(1), "1: the value before");
        assert_eq!(value.load(Relaxed), 3, "the exchange landed");
    }

    #[test]
    fn a_weak_exchange_converges_in_a_loop() {
        let value = AtomicU64::new(5);
        let mut expected = 0;
        while let Err(seen) = value.compare_exchange_weak(expected, 6, AcqRel, Acquire) {
            expected = seen;
        }
        assert_eq!(value.load(Relaxed), 6, "the loop landed its write");
    }

    #[test]
    fn update_returns_the_value_before_and_stores_the_new_one() {
        let value = AtomicU64::new(2);
        assert_eq!(value.update(AcqRel, Acquire, |seen| seen * 10), 2, "the value before");
        assert_eq!(value.load(Relaxed), 20, "the value after");
    }

    #[test]
    fn try_update_stores_nothing_when_the_closure_declines() {
        let value = AtomicU64::new(7);
        assert_eq!(value.try_update(AcqRel, Acquire, |_| None), Err(7), "declined: the value seen");
        assert_eq!(value.try_update(AcqRel, Acquire, |seen| Some(seen + 1)), Ok(7), "accepted");
        assert_eq!(value.load(Relaxed), 8, "and stored");
    }

    #[test]
    fn the_rmw_load_and_store_agree_with_the_plain_ones() {
        let value = AtomicU64::new(0);
        assert_eq!(value.load_rmw(SeqCst), 0, "a zero cell");
        value.store_rmw(9, Release);
        assert_eq!(value.load_rmw(Acquire), 9, "the value stored");
        assert_eq!(value.load(Relaxed), 9, "as a plain load sees it");
    }

    #[test]
    fn an_atomic_round_trips_through_its_pointer() {
        let value = AtomicU8::new(3);
        // SAFETY: the pointer is the live atomic's own, aligned, and reached only atomically; it
        // holds a repr that decodes, and `value` outlives `again`.
        #[expect(unsafe_code, reason = "rebuilding an atomic from the pointer it gave out")]
        let again = unsafe { AtomicU8::from_ptr(value.as_ptr()) };
        again.store(4, Relaxed);
        assert_eq!(value.load(Relaxed), 4, "the same cell");
    }

    #[test]
    fn construction_conversion_and_default_agree() {
        assert_eq!(AtomicU64::new(3).into_inner(), 3, "new, then into_inner");
        assert_eq!(AtomicU64::from(4).into_inner(), 4, "From");
        assert_eq!(AtomicU64::default().into_inner(), 0, "Default is the value's");
    }

    #[test]
    fn new_get_mut_and_into_inner_are_const() {
        const {
            let mut value = AtomicU64::new(3);
            *value.get_mut() += 1;
            assert!(value.into_inner() == 4, "into_inner reads what get_mut wrote, in const");
        }
    }

    #[test]
    fn debug_prints_the_value() {
        let mut text = String::new();
        write!(text, "{:?}", AtomicI64::new(-1)).expect("a String takes any write");
        assert_eq!(text, "-1", "the value, as its own Debug prints it");
    }

    /// Generic runtime code builds an atomic through `From`, needing no const bound.
    fn build<T: Atom>(value: T) -> Atomic<T> {
        Atomic::from(value)
    }

    #[test]
    fn generic_code_builds_through_from() {
        assert!(build(true).load(Relaxed), "the value given");
    }

    /// Compiles only where `T` has the auto traits core's atomics have.
    const fn has_auto_traits<T: Send + Sync + Unpin + RefUnwindSafe + UnwindSafe>() {}

    /// Compiles only where every `Atomic<T>` of an `Unpin` and `UnwindSafe` `T` has them.
    const fn atomic_has_auto_traits<T: Atom + Unpin + UnwindSafe>() {
        has_auto_traits::<Atomic<T>>();
    }

    // A pointer is neither `Send` nor `Sync`, so only `Atomic`'s own impls give its atomic either.
    const _: () = has_auto_traits::<Atomic<*mut u8>>();

    #[test]
    fn an_atomic_is_send_sync_and_ref_unwind_safe_whatever_its_value() {
        atomic_has_auto_traits::<u64>();
        atomic_has_auto_traits::<bool>();
    }

    #[test]
    fn exclusive_access_reads_and_writes_the_value() {
        let mut value = AtomicU64::new(1);
        assert_eq!(value.get(), 1, "get reads what new wrote");
        value.set(2);
        assert_eq!(value.get(), 2, "get reads what set wrote");
        let doubled = value.with_mut(|seen| {
            *seen *= 2;
            *seen
        });
        assert_eq!(
            (doubled, value.get()),
            (4, 4),
            "with_mut returns f's result and keeps its write"
        );
        *value.get_mut() += 1;
        assert_eq!(value.into_inner(), 5, "into_inner reads what get_mut wrote");
    }

    #[test]
    fn a_panic_in_with_mut_leaves_the_value_before() {
        let mut value = AtomicU64::new(1);
        let payload = panic::catch_unwind(AssertUnwindSafe(|| {
            value.with_mut(|seen| {
                *seen = 2;
                panic::resume_unwind(Box::new("the closure's panic"));
            });
        }))
        .expect_err("the closure's panic reaches `catch_unwind`");
        assert_eq!(
            payload.downcast_ref::<&str>(),
            Some(&"the closure's panic"),
            "the panic is the closure's"
        );
        assert_eq!(value.get(), 1, "the value before stays, without the closure's write");
    }

    /// Builds an atomic and reads it back, as generic run-time code can: `new` and `into_inner`
    /// need `T: const Atom`.
    fn round_trip<T: Atom>(value: T) -> T {
        Atomic::from(value).get()
    }

    #[test]
    fn generic_code_reads_back_through_get() {
        assert_eq!(round_trip(-3_i64), -3, "get reads what `From` wrote");
    }
}
