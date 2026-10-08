//! 128-bit atomics, on every target with a 16-byte compare-exchange.
//!
//! The pure load and store exist only with LSE2 (`aarch64`) or AVX (`x86_64`).

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#![cfg(not(loom))]
#![cfg(wide)]

#[cfg(test)]
mod tests {
    use core::num::NonZero;

    use atomix_core::ordering::{AcqRel, Acquire, Relaxed, Release, SeqCst};
    use atomix_core::{Atomic, AtomicI128, AtomicU128};

    const HIGH: u128 = 1 << 100;

    static WIDE: AtomicU128 = AtomicU128::new(HIGH | 3);

    #[test]
    fn a_wide_atomic_is_sixteen_bytes_aligned_to_sixteen() {
        assert_eq!(size_of::<AtomicU128>(), 16, "one 16-byte word");
        assert_eq!(align_of::<AtomicU128>(), 16, "aligned as a 16-byte atomic must be");
        assert_eq!(size_of::<Atomic<NonZero<u128>>>(), 16, "a nonzero's opaque cell is one too");
        assert_eq!(align_of::<Atomic<NonZero<u128>>>(), 16, "and aligned to sixteen");
    }

    #[test]
    fn a_static_starts_with_its_const_initializer() {
        assert_eq!(
            WIDE.compare_exchange(HIGH | 3, HIGH, AcqRel, Acquire),
            Ok(HIGH | 3),
            "the exchange finds the const initializer"
        );
    }

    #[test]
    fn compare_exchange_moves_all_sixteen_bytes() {
        let value = AtomicU128::new(HIGH | 1);
        assert_eq!(
            value.compare_exchange(1, 2, AcqRel, Acquire),
            Err(HIGH | 1),
            "the high half counts"
        );
        assert_eq!(
            value.compare_exchange(HIGH | 1, HIGH | 2, AcqRel, Acquire),
            Ok(HIGH | 1),
            "both halves match"
        );
        assert_eq!(value.load_rmw(Acquire), HIGH | 2, "and both landed");
    }

    #[test]
    fn update_and_the_rmw_store_reach_all_sixteen_bytes() {
        let value = AtomicI128::new(-1);
        assert_eq!(
            value.update(AcqRel, Acquire, |seen| seen.wrapping_add(2)),
            -1,
            "the value before"
        );
        assert_eq!(value.load_rmw(Acquire), 1, "the carry crossed into the high half");
        value.store_rmw(i128::MIN, Release);
        assert_eq!(value.load_rmw(Relaxed), i128::MIN, "the stored value");
    }

    #[test]
    fn update_first_sees_the_value_the_cell_holds() {
        for start in [0, -1, i128::MIN] {
            let value = AtomicI128::new(start);
            let mut first = None;
            let _ = value.update(AcqRel, Acquire, |seen| {
                let _ = first.get_or_insert(seen);
                seen.wrapping_add(2)
            });
            assert_eq!(
                first,
                Some(start),
                "the first read, a load or an exchange of zero, finds it"
            );
        }
    }

    #[test]
    fn update_reaches_a_nonzero_and_its_option() {
        let (one, high) = (NonZero::<u128>::MIN, NonZero::new(HIGH).expect("HIGH is not zero"));
        let nonzero = Atomic::new(one);
        assert_eq!(nonzero.update(AcqRel, Acquire, |_| high), one, "the value before");
        assert_eq!(nonzero.load_rmw(Acquire), high, "a load reads what update wrote");
        // Without a pure load, the first read is an exchange of zero for zero, which lands on
        // `None`'s repr.
        let option = Atomic::new(None);
        assert_eq!(option.update(AcqRel, Acquire, |_| Some(high)), None, "the value before");
        assert_eq!(option.load_rmw(Acquire), Some(high), "a load reads what update wrote");
    }

    #[test]
    fn exclusive_access_reaches_all_sixteen_bytes() {
        let mut signed = AtomicI128::new(-1);
        *signed.get_mut() = i128::MIN + 1;
        assert_eq!(signed.get(), i128::MIN + 1, "get reads what get_mut wrote");
        signed.set(-2);
        signed.with_mut(|seen| *seen = seen.wrapping_sub(1));
        assert_eq!(signed.load_rmw(Relaxed), -3, "a load reads what set and with_mut left");
        assert_eq!(signed.into_inner(), -3, "into_inner reads it too");
        let mut unsigned = AtomicU128::new(HIGH);
        *unsigned.get_mut() |= 1;
        assert_eq!(unsigned.into_inner(), HIGH | 1, "into_inner reads what get_mut wrote");
    }

    #[test]
    fn new_get_mut_and_into_inner_are_const() {
        const {
            let mut value = AtomicI128::new(-1);
            *value.get_mut() = i128::MIN;
            assert!(
                value.into_inner() == i128::MIN,
                "into_inner reads what get_mut wrote, in const"
            );
        }
    }

    #[test]
    fn a_plain_word_is_an_atomic_through_its_pointer() {
        let mut plain = HIGH;
        let place = &raw mut plain;
        {
            // SAFETY: `place` is a live `u128`, aligned to 16 as `AtomicU128` is, and every repr of
            // it decodes; it outlives `view`, and nothing else reaches it while `view` lives.
            #[expect(unsafe_code, reason = "a plain `u128` viewed as an atomic")]
            let view = unsafe { AtomicU128::from_ptr(place) };
            assert_eq!(view.as_ptr(), place, "as_ptr is the word's address");
            assert_eq!(
                view.compare_exchange(HIGH, HIGH | 1, AcqRel, Acquire),
                Ok(HIGH),
                "the view reads the word"
            );
        }
        assert_eq!(plain, HIGH | 1, "the word holds the view's write once the view has ended");
        let mut signed = -1_i128;
        {
            // SAFETY: as above, for an `i128`.
            #[expect(unsafe_code, reason = "a plain `i128` viewed as an atomic")]
            let view = unsafe { AtomicI128::from_ptr(&raw mut signed) };
            view.store_rmw(i128::MIN, SeqCst);
        }
        assert_eq!(signed, i128::MIN, "an `i128` holds what its view's `u128` cell wrote");
    }

    #[test]
    #[cfg(wide_load_store)]
    fn a_pure_load_and_store_exist_where_the_target_has_them() {
        let value = AtomicU128::new(0);
        value.store(HIGH, Release);
        assert_eq!(value.load(Acquire), HIGH, "the load reads all sixteen bytes the store wrote");
        value.store(HIGH | 1, Relaxed);
        assert_eq!(value.load(Relaxed), HIGH | 1, "and so with Relaxed");
        value.store(HIGH | 2, SeqCst);
        assert_eq!(value.load(SeqCst), HIGH | 2, "and with SeqCst");
    }
}
