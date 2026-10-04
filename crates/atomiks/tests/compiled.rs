//! The tests that run the compiler: what a crate that derives sees, and what a derived value's
//! load lowers to.

// Neither loom's model nor Miri's interpreter can run the compiler, and loom's atomics are not
// what ships.
#![cfg(not(any(loom, miri)))]

// atomiks-core's, by its path; its `//!` says why.
#[cfg(test)]
#[path = "../../atomiks-core/tests/testing/mod.rs"]
mod testing;

// Every fixture derives.
#[cfg(feature = "derive")]
#[cfg(test)]
mod trybuild {
    //! What a crate that derives sees: what compiles under strict lints, with no feature gate for
    //! a type without parameters, and each refusal with its message.

    #[test]
    fn user_crates_compile_or_are_refused_with_the_message() {
        let cases = trybuild::TestCases::new();
        // With a pass case, trybuild builds every fixture rather than checking it, so the
        // post-monomorphization errors show.
        cases.pass("tests/compile_pass/*.rs");
        cases.compile_fail("tests/compile_fail/*.rs");
    }
}

#[cfg(test)]
mod codegen {
    //! What a derived value's load lowers to, per target, read from the assembly of
    //! `tests/codegen`.
    //!
    //! The fixture builds for Linux and macOS with the repository's CPU floor and, on `aarch64`
    //! Linux, with LSE2 too. Each load is the instruction an `Acquire` load is, then the decode
    //! `from_repr_unchecked` gives, with no check: shifts and masks with no branch for a packed
    //! struct, nothing for a fieldless enum, and no loop for a niche-filling enum.

    use crate::testing::codegen::Lowering::{self, InOrder, Only};
    use crate::testing::codegen::{
        AARCH64_LINUX, AARCH64_MACOS, X86_64_LINUX, X86_64_MACOS, lowers_as_expected,
    };

    /// Each load on `aarch64` with the `+lse` floor, which reads with `ldar`.
    const AARCH64_FLOOR: &[(&str, Lowering)] = &[
        ("packed_struct_load", Only(&["ldar", "lsr", "and", "bfi", "ret"])),
        ("fieldless_enum_load", Only(&["ldarb", "ret"])),
        ("niche_filling_enum_load", InOrder(&["ldarb"])),
    ];

    /// Each load on `aarch64` with LSE2, `neoverse-v1` on Linux and macOS's floor, `apple-m1`,
    /// which reads with `ldapr`.
    const AARCH64_LSE2: &[(&str, Lowering)] = &[
        ("packed_struct_load", Only(&["ldapr", "lsr", "and", "bfi", "ret"])),
        ("fieldless_enum_load", Only(&["ldaprb", "ret"])),
        ("niche_filling_enum_load", InOrder(&["ldaprb"])),
    ];

    /// Each load on `x86_64`, Linux or macOS, at the `x86-64-v3` floor, whose BMI2 masks with
    /// `bzhi`.
    const X86_64: &[(&str, Lowering)] = &[
        (
            "packed_struct_load",
            Only(&["movq", "movb", "bzhiq", "shlq", "movabsq", "andq", "orq", "retq"]),
        ),
        ("fieldless_enum_load", Only(&["movzbl", "retq"])),
        ("niche_filling_enum_load", InOrder(&["movzbl"])),
    ];

    #[test]
    fn aarch64_linux_lowers_each_derived_load_to_a_load_and_its_decode() {
        lowers_as_expected(AARCH64_LINUX, None, &[AARCH64_FLOOR]);
    }

    #[test]
    fn aarch64_linux_with_lse2_lowers_each_derived_load_to_a_load_and_its_decode() {
        lowers_as_expected(AARCH64_LINUX, Some("neoverse-v1"), &[AARCH64_LSE2]);
    }

    #[test]
    fn aarch64_macos_lowers_each_derived_load_to_a_load_and_its_decode() {
        lowers_as_expected(AARCH64_MACOS, None, &[AARCH64_LSE2]);
    }

    #[test]
    fn x86_64_linux_lowers_each_derived_load_to_a_load_and_its_decode() {
        lowers_as_expected(X86_64_LINUX, None, &[X86_64]);
    }

    #[test]
    fn x86_64_macos_lowers_each_derived_load_to_a_load_and_its_decode() {
        lowers_as_expected(X86_64_MACOS, None, &[X86_64]);
    }
}
