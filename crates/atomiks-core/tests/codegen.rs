//! What each operation lowers to, per target, read from the assembly of `tests/codegen`.
//!
//! The fixture builds for Linux and macOS with the repository's CPU floor and, on `aarch64` Linux,
//! with LSE2 too. Each operation is the instructions and barriers its name promises, with no
//! compare-exchange loop but `update`'s, and each target refuses each operation it lacks: `x86_64`
//! those only `aarch64` has, and `aarch64` each 128-bit one it has no instruction for. A ranged
//! integer's range reaches LLVM, so a comparison outside it folds to a constant.

// Miri cannot run the compiler, and loom's atomics are not what ships.
#![cfg(on_hardware)]

#[cfg(test)]
mod testing;

#[cfg(test)]
mod tests {
    use crate::testing::codegen::Lowering::{self, InOrder, Only, Retry};
    use crate::testing::codegen::{
        AARCH64_LINUX, AARCH64_MACOS, X86_64_LINUX, X86_64_MACOS, lowers_as_expected, refused,
    };

    /// Each function of the fixture on `aarch64` that lowers the same with LSE2 as without.
    const AARCH64: &[(&str, Lowering)] = &[
        ("u64_store", InOrder(&["stlr"])),
        ("u64_swap", InOrder(&["swpal"])),
        ("u64_fetch_add", InOrder(&["ldaddal"])),
        ("u64_fetch_sub", InOrder(&["neg", "ldaddal"])),
        ("u64_compare_exchange", InOrder(&["casal"])),
        ("u64_add", InOrder(&["ldadd"])),
        ("u64_sub", InOrder(&["neg", "ldadd"])),
        ("u64_or", InOrder(&["ldsetl"])),
        ("u64_and", InOrder(&["mvn", "ldclrl"])),
        ("u64_xor", InOrder(&["ldeorl"])),
        ("u64_not", InOrder(&["ldeorl"])),
        ("u64_fetch_and", InOrder(&["mvn", "ldclral"])),
        ("u64_fetch_or", InOrder(&["ldsetal"])),
        ("u64_fetch_xor", InOrder(&["ldeoral"])),
        ("u64_fetch_not", InOrder(&["ldeoral"])),
        ("u64_max", InOrder(&["ldumax"])),
        ("u64_min", InOrder(&["ldumin"])),
        ("u64_fetch_max", InOrder(&["ldumaxal"])),
        ("i64_max", InOrder(&["ldsmax"])),
        ("i64_fetch_min", InOrder(&["ldsminal"])),
        ("bool_or", InOrder(&["ldsetlb"])),
        ("ptr_byte_add", InOrder(&["ldadd"])),
        ("ptr_fetch_ptr_sub", InOrder(&["neg", "ldaddal"])),
        ("u128_compare_exchange", InOrder(&["caspal"])),
        ("u128_load_rmw", InOrder(&["caspa"])),
        ("store_store_fence", InOrder(&["str", "dmb ishst", "str"])),
        ("seq_cst_compiler_fence", Only(&["ret"])),
        ("acquire_fence", Only(&["dmb ishld", "ret"])),
        ("release_fence", Only(&["dmb ish", "ret"])),
        ("seq_cst_fence", Only(&["dmb ish", "ret"])),
        ("ranged_below_min", Only(&["mov", "ret"])),
    ];

    /// The rest on `aarch64` with the `+lse` floor, which has no LSE2.
    ///
    /// With no 128-bit load, `update` reads with a compare-exchange. A load of a `char`, an
    /// `Option` or a ranged integer, as every load, decodes without a check.
    const AARCH64_FLOOR: &[(&str, Lowering)] = &[
        ("u64_load", InOrder(&["ldar"])),
        ("char_load", Only(&["ldar", "ret"])),
        ("option_load", Only(&["ldar", "ret"])),
        ("ranged_load", Only(&["ldar", "ret"])),
        ("u128_update", Retry(&["caspa", "caspal"])),
    ];

    /// The rest on `aarch64` with LSE2: `neoverse-v1` on Linux, and macOS's floor, `apple-m1`.
    ///
    /// A 128-bit load or store is `ldp` or `stp` with the barriers its ordering needs, and an
    /// Acquire load is `ldapr`.
    const AARCH64_LSE2: &[(&str, Lowering)] = &[
        ("u64_load", InOrder(&["ldapr"])),
        ("char_load", Only(&["ldapr", "ret"])),
        ("option_load", Only(&["ldapr", "ret"])),
        ("ranged_load", Only(&["ldapr", "ret"])),
        ("u128_load", InOrder(&["ldp", "dmb ishld"])),
        ("u128_load_relaxed", InOrder(&["ldp"])),
        ("u128_load_seq_cst", InOrder(&["ldar", "ldp", "dmb ish"])),
        ("u128_store", InOrder(&["dmb ish", "stp"])),
        ("u128_store_seq_cst", InOrder(&["dmb ish", "stp", "dmb ish"])),
        ("u128_update", Retry(&["ldp", "dmb ishld", "caspal"])),
    ];

    /// Each function of the fixture on `x86_64`, Linux or macOS, at the `x86-64-v3` floor (AVX).
    const X86_64: &[(&str, Lowering)] = &[
        ("u64_load", Only(&["movq", "retq"])),
        ("u64_store", Only(&["movq", "retq"])),
        ("u64_swap", InOrder(&["xchgq"])),
        ("u64_fetch_add", InOrder(&["lock xaddq"])),
        ("u64_fetch_sub", InOrder(&["negq", "lock xaddq"])),
        ("u64_compare_exchange", InOrder(&["lock cmpxchgq"])),
        ("u64_add", InOrder(&["lock addq"])),
        ("u64_sub", InOrder(&["lock subq"])),
        ("u64_or", InOrder(&["lock orq"])),
        ("u64_and", InOrder(&["lock andq"])),
        ("u64_xor", InOrder(&["lock xorq"])),
        ("u64_not", InOrder(&["lock xorq"])),
        ("bool_or", InOrder(&["lock orb"])),
        ("ptr_byte_add", InOrder(&["lock addq"])),
        ("ptr_fetch_ptr_sub", InOrder(&["negq", "lock xaddq"])),
        ("char_load", Only(&["movl", "retq"])),
        ("option_load", Only(&["movq", "retq"])),
        ("ranged_load", Only(&["movq", "retq"])),
        ("u128_load", InOrder(&["vmovdqa"])),
        ("u128_load_relaxed", InOrder(&["vmovdqa"])),
        ("u128_load_seq_cst", InOrder(&["vmovdqa"])),
        ("u128_store", InOrder(&["vmovdqa"])),
        ("u128_store_seq_cst", InOrder(&["vmovdqa", "lock orl"])),
        ("u128_compare_exchange", InOrder(&["lock cmpxchg16b"])),
        ("u128_load_rmw", InOrder(&["lock cmpxchg16b"])),
        ("u128_update", Retry(&["vmovdqa", "lock cmpxchg16b"])),
        ("store_store_fence", Only(&["movq", "movq", "retq"])),
        ("seq_cst_compiler_fence", Only(&["retq"])),
        ("acquire_fence", Only(&["retq"])),
        ("release_fence", Only(&["retq"])),
        ("seq_cst_fence", Only(&["lock orl", "retq"])),
        ("ranged_below_min", Only(&["xorl", "retq"])),
    ];

    #[test]
    fn aarch64_linux_lowers_each_operation_to_its_instruction() {
        lowers_as_expected(AARCH64_LINUX, None, &[AARCH64, AARCH64_FLOOR]);
    }

    #[test]
    fn aarch64_linux_with_lse2_lowers_each_operation_to_its_instruction() {
        lowers_as_expected(AARCH64_LINUX, Some("neoverse-v1"), &[AARCH64, AARCH64_LSE2]);
    }

    #[test]
    fn aarch64_macos_lowers_each_operation_to_its_instruction() {
        lowers_as_expected(AARCH64_MACOS, None, &[AARCH64, AARCH64_LSE2]);
    }

    #[test]
    fn x86_64_linux_lowers_each_operation_to_its_instruction() {
        lowers_as_expected(X86_64_LINUX, None, &[X86_64]);
    }

    #[test]
    fn x86_64_macos_lowers_each_operation_to_its_instruction() {
        lowers_as_expected(X86_64_MACOS, None, &[X86_64]);
    }

    /// Checks that `target`, an `x86_64` one, refuses each probe only `aarch64` has, each with its
    /// capability's message.
    fn refuses_each_operation_only_aarch64_has(target: &str) {
        let stderr = refused(target, "aarch64-only");
        let errors: Vec<&str> = stderr.lines().filter(|line| line.starts_with("error[")).collect();
        let fetch_bitwise = "has no `fetch_and`, `fetch_or`, `fetch_xor` or `fetch_not` without a \
                             compare-exchange loop on this target";
        let min_max =
            "has no atomic maximum or minimum without a compare-exchange loop on this target";
        assert!(
            errors.iter().all(|error| error.starts_with("error[E0277]: `")
                && (error.ends_with(fetch_bitwise) || error.ends_with(min_max))),
            "every error is `FetchBitwise`'s or `MinMax`'s:\n{stderr}"
        );
        let aarch64_only = AARCH64
            .iter()
            .chain(AARCH64_FLOOR)
            .filter(|(name, _)| X86_64.iter().all(|(other, _)| other != name))
            .count();
        assert_eq!(errors.len(), aarch64_only, "one error per probe only aarch64 has:\n{stderr}");
        for line in [
            "= help: the trait `FetchBitwise` is not implemented for `u64`",
            "= note: x86_64's `lock and`, `lock or` and `lock xor` cannot return the value before",
            "= help: the trait `MinMax` is not implemented for `i64`",
            "= note: x86_64 has no atomic maximum or minimum",
            "= note: to accept a compare-exchange loop, call `update`",
        ] {
            assert!(stderr.contains(line), "the diagnostics say `{line}`:\n{stderr}");
        }
    }

    #[test]
    fn x86_64_linux_refuses_each_operation_only_aarch64_has() {
        refuses_each_operation_only_aarch64_has(X86_64_LINUX);
    }

    #[test]
    fn x86_64_macos_refuses_each_operation_only_aarch64_has() {
        refuses_each_operation_only_aarch64_has(X86_64_MACOS);
    }

    #[test]
    fn aarch64_linux_refuses_each_wide_capability() {
        let stderr = refused(AARCH64_LINUX, "aarch64-refused");
        let errors: Vec<&str> = stderr.lines().filter(|line| line.starts_with("error[")).collect();
        assert_eq!(
            errors,
            [
                "error[E0277]: `u128` has no pure-read atomic load on this target",
                "error[E0277]: `u128` has no atomic store without a compare-exchange loop on this \
                 target",
                "error[E0277]: `u128` has no atomic exchange without a compare-exchange loop on \
                 this target",
                "error[E0277]: `u128` has no atomic maximum or minimum without a compare-exchange \
                 loop on this target",
            ],
            "one error per probe: `Load`'s, `Store`'s, `Swap`'s and `MinMax`'s:\n{stderr}"
        );
        for line in [
            "= note: a 128-bit load is one instruction with FEAT_LSE2",
            "= note: to accept a load that writes the cache line, call `load_rmw`",
            "= note: a 128-bit store is one instruction with FEAT_LSE2",
            "= note: to accept a compare-exchange loop, call `store_rmw`",
            "= note: atomiks has no 128-bit exchange: call `update` with `|_| new`",
            "= note: aarch64's atomic maximum and minimum take an integer of at most 64 bits",
            "= note: to accept a compare-exchange loop, call `update`",
        ] {
            assert!(stderr.contains(line), "the diagnostics say `{line}`:\n{stderr}");
        }
    }

    #[test]
    fn aarch64_macos_refuses_the_wide_exchange_and_maximum() {
        let stderr = refused(AARCH64_MACOS, "aarch64-refused");
        let errors: Vec<&str> = stderr.lines().filter(|line| line.starts_with("error[")).collect();
        assert_eq!(
            errors,
            [
                "error[E0277]: `u128` has no atomic exchange without a compare-exchange loop on \
                 this target",
                "error[E0277]: `u128` has no atomic maximum or minimum without a compare-exchange \
                 loop on this target",
            ],
            "LSE2 has the load and store, so only `Swap`'s and `MinMax`'s probes fail:\n{stderr}"
        );
    }
}
