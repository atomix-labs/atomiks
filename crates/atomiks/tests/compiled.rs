//! The tests that run the compiler: what a crate that derives sees, and what a derived value's
//! load and its fields' operations lower to.

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
    //! What a derived value's load and its fields' operations lower to, per target, read from the
    //! assembly of `tests/codegen`.
    //!
    //! The fixture builds for Linux and macOS with the repository's CPU floor and, on `aarch64`
    //! Linux, with LSE2 too. Each load is the instruction an `Acquire` load is, then the decode
    //! `from_repr_unchecked` gives, with no check: shifts and masks with no branch for a packed
    //! struct, nothing for a fieldless enum, and no loop for a niche-filling enum. Each field
    //! operation through the projection is the one instruction its whole word's is, its operand
    //! confined to the field.

    use crate::testing::codegen::Lowering::{self, InOrder, Only};
    use crate::testing::codegen::{
        AARCH64_LINUX, AARCH64_MACOS, X86_64_LINUX, X86_64_MACOS, lowers_as_expected,
    };

    /// Each field operation on `aarch64`, the same at the floor, with LSE2 and on macOS: one LSE
    /// instruction, then a shift for a bit or a count.
    const AARCH64: &[(&str, Lowering)] = &[
        ("field_set", Only(&["mov", "ldsetl", "ret"])),
        ("field_clear", Only(&["mov", "ldclrl", "ret"])),
        ("field_test_and_set", Only(&["mov", "ldsetal", "ubfx", "ret"])),
        ("flags_or", Only(&["and", "lsl", "ldsetl", "ret"])),
        ("nested_clear", Only(&["mov", "ldclrl", "ret"])),
        ("top_fetch_add", Only(&["lsl", "ldaddal", "lsr", "ret"])),
    ];

    /// Each load on `aarch64` with the `+lse` floor, which reads with `ldar`.
    const AARCH64_FLOOR: &[(&str, Lowering)] = &[
        ("packed_struct_load", Only(&["ldar", "lsr", "and", "bfi", "ret"])),
        ("fieldless_enum_load", Only(&["ldarb", "ret"])),
        ("niche_filling_enum_load", InOrder(&["ldarb"])),
        ("field_load", Only(&["ldar", "ubfx", "ret"])),
    ];

    /// Each load on `aarch64` with LSE2, `neoverse-v1` on Linux and macOS's floor, `apple-m1`,
    /// which reads with `ldapr`.
    const AARCH64_LSE2: &[(&str, Lowering)] = &[
        ("packed_struct_load", Only(&["ldapr", "lsr", "and", "bfi", "ret"])),
        ("fieldless_enum_load", Only(&["ldaprb", "ret"])),
        ("niche_filling_enum_load", InOrder(&["ldaprb"])),
        ("field_load", Only(&["ldapr", "ubfx", "ret"])),
    ];

    /// Each load on `x86_64`, Linux or macOS, at the `x86-64-v3` floor, whose BMI2 masks with
    /// `bzhi`, and each field read-modify-write: one `lock` instruction, `lock bts` for a bit's
    /// test.
    const X86_64: &[(&str, Lowering)] = &[
        (
            "packed_struct_load",
            Only(&["movq", "movb", "bzhiq", "shlq", "movabsq", "andq", "orq", "retq"]),
        ),
        ("fieldless_enum_load", Only(&["movzbl", "retq"])),
        ("niche_filling_enum_load", InOrder(&["movzbl"])),
        ("field_set", Only(&["movabsq", "lock orq", "retq"])),
        ("field_clear", Only(&["movabsq", "lock andq", "retq"])),
        ("field_test_and_set", Only(&["lock btsq", "setb", "retq"])),
        ("field_load", Only(&["movq", "shrq", "andl", "retq"])),
        ("flags_or", Only(&["movzbl", "shlq", "lock orq", "retq"])),
        ("nested_clear", Only(&["lock andl", "retq"])),
        ("top_fetch_add", Only(&["movl", "shlq", "lock xaddq", "shrq", "retq"])),
    ];

    #[test]
    fn aarch64_linux_lowers_each_derived_load_and_field_operation_to_its_instructions() {
        lowers_as_expected(AARCH64_LINUX, None, &[AARCH64, AARCH64_FLOOR]);
    }

    #[test]
    fn aarch64_linux_with_lse2_lowers_each_derived_load_and_field_operation_to_its_instructions() {
        lowers_as_expected(AARCH64_LINUX, Some("neoverse-v1"), &[AARCH64, AARCH64_LSE2]);
    }

    #[test]
    fn aarch64_macos_lowers_each_derived_load_and_field_operation_to_its_instructions() {
        lowers_as_expected(AARCH64_MACOS, None, &[AARCH64, AARCH64_LSE2]);
    }

    #[test]
    fn x86_64_linux_lowers_each_derived_load_and_field_operation_to_its_instructions() {
        lowers_as_expected(X86_64_LINUX, None, &[X86_64]);
    }

    #[test]
    fn x86_64_macos_lowers_each_derived_load_and_field_operation_to_its_instructions() {
        lowers_as_expected(X86_64_MACOS, None, &[X86_64]);
    }
}
