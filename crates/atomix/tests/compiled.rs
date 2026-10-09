//! The tests that run the compiler: what a crate that derives sees, and what each operation on a
//! derived value lowers to.

// Neither loom's model nor Miri's interpreter can run the compiler, and loom's atomics are not
// what ships.
#![cfg(not(any(loom, miri)))]

// atomix-core's, by its path; its `//!` says why.
#[cfg(test)]
#[path = "../../atomix-core/tests/testing/mod.rs"]
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
        // A refusal whose notes `x86_64` adds to, which `aarch64`'s message alone pins.
        if cfg!(target_arch = "aarch64") {
            cases.compile_fail("tests/compile_fail/aarch64/*.rs");
        }
    }
}

#[cfg(test)]
mod codegen {
    //! What each operation on a derived value lowers to, per target, read from the assembly of
    //! `tests/codegen`.
    //!
    //! The fixture builds for Linux and macOS with the repository's CPU floor and, on `aarch64`
    //! Linux, with LSE2 too. Each load is the instruction an `Acquire` load is, then the decode
    //! `from_repr_unchecked` gives, with no check: shifts and masks with no branch for a packed
    //! struct, nothing for a fieldless enum, and no loop for a niche-filling enum. Each field
    //! operation through the projection is the one instruction its whole word's is, its operand
    //! confined to the field, a pointer word's tag's too, beside a pointer, a pointer enum or an
    //! inner word's tags. A match of a pointer enum is one mask and the compare chain the
    //! match needs, then a load through an arm's pointer that takes the arm's tag into its own
    //! offset, or, where LLVM merges arms that load through pointers of several tags, one mask to
    //! take them off; a word read through its pointer's place is one mask. A store tests its
    //! pointer once against every tag bit, a word's over a word or over a pointer enum too, an
    //! exchange both its pointers in one test, and a store of a pointer enum's unit or data, or of
    //! a pointer that fills a niche, tests nothing; each test's cold refusal saves the frame record
    //! off the fast path. An update that keeps a pointer enum's pointer writes back the word it
    //! read, and tests, calls and saves nothing; but a word's over an enum of several pointer
    //! variants saves a frame record on `aarch64`, and on `x86_64` keeps a branch to the cold
    //! refusal that it never takes. A struct of two words lowers as a double word does: a pointer
    //! beside an integer word of its tags tests nothing, its exchange `u128`'s exactly on `x86_64`,
    //! and two pointers, a mark in the first's low bits, are tested in one test before their one
    //! exchange or store. Where `x86_64` has no `cmpxchg16b`, each shape of two words is refused
    //! once, at its derive.

    use crate::testing::codegen::Lowering::{self, InOrder, Only, Refuses, Retry, RetryOnly};
    use crate::testing::codegen::{
        AARCH64_LINUX, AARCH64_MACOS, X86_64_LINUX, X86_64_MACOS, lowers_as_expected, refused,
    };

    /// Each store, exchange and field operation on `aarch64`, the same at the floor, with LSE2 and
    /// on macOS: a field operation is one LSE instruction, then a shift for a bit or a count.
    const AARCH64: &[(&str, Lowering)] = &[
        ("counted_head_load_rmw", Only(&["mov", "mov", "caspa", "mov", "mov", "ret"])),
        ("counted_head_compare_exchange", InOrder(&["caspal", "cmp", "ccmp", "cset"])),
        ("marked_pair_compare_exchange", InOrder(&["orr", "tbnz", "caspal", "bl"])),
        ("field_set", Only(&["mov", "ldsetl", "ret"])),
        ("field_clear", Only(&["mov", "ldclrl", "ret"])),
        ("field_test_and_set", Only(&["mov", "ldsetal", "ubfx", "ret"])),
        ("flags_or", Only(&["and", "lsl", "ldsetl", "ret"])),
        ("nested_clear", Only(&["mov", "ldclrl", "ret"])),
        ("top_fetch_add", Only(&["lsl", "ldaddal", "lsr", "ret"])),
        ("head_test_and_set_marked", Only(&["mov", "ldsetal", "ubfx", "ret"])),
        ("slot_store_inline", Only(&["mov", "mov", "orr", "stlr", "ret"])),
        ("slot_store_node", Refuses(&["tst", "b.ne", "add", "stlr", "ret"])),
        ("next_store_node", Only(&["stlr", "ret"])),
        ("marked_store", Refuses(&["tbnz", "add", "stlr", "ret"])),
        ("guarded_store", Refuses(&["tst", "b.ne", "add", "stlr", "ret"])),
        ("locked_slot_store_node", Refuses(&["tst", "b.ne", "add", "stlr", "ret"])),
        ("head_compare_exchange", Refuses(&["orr", "tst", "b.ne", "casal", "ret"])),
        ("locked_slot_set_locked", Only(&["mov", "ldsetl", "ret"])),
        ("locked_slot_test_and_set_locked", Only(&["mov", "ldsetal", "ubfx", "ret"])),
        ("guarded_set_locked", Only(&["mov", "ldsetl", "ret"])),
        ("guarded_test_and_set_marked", Only(&["mov", "ldsetal", "and", "ret"])),
    ];

    /// Each load and update on `aarch64` with the `+lse` floor, which reads with `ldar`.
    const AARCH64_FLOOR: &[(&str, Lowering)] = &[
        ("counted_head_update_version", Retry(&["caspa", "add", "caspal"])),
        ("packed_struct_load", Only(&["ldar", "lsr", "and", "bfi", "ret"])),
        ("fieldless_enum_load", Only(&["ldarb", "ret"])),
        ("niche_filling_enum_load", InOrder(&["ldarb"])),
        ("field_load", Only(&["ldar", "ubfx", "ret"])),
        ("head_load_top", Only(&["ldar", "and", "ret"])),
        ("next_load_match", Only(&["ldar", "cbz", "ldr", "ret", "mov", "ret"])),
        ("child_load_match", Only(&["ldar", "ubfiz", "and", "ldr", "ret"])),
        ("child_load_leaf", Only(&["ldar", "tbz", "mov", "ret", "ldr", "ret"])),
        ("child_load_branch", Only(&["ldar", "tbz", "ldur", "ret", "mov", "ret"])),
        (
            "slot_load_match",
            Only(&[
                "ldar", "ands", "b.eq", "cmp", "b.ne", "ubfx", "ret", "mov", "ret", "ldur", "ret",
            ]),
        ),
        ("entry_load_match", Only(&["ldar", "tbz", "ldur", "ret", "ubfx", "ret"])),
        ("guarded_load_inner", Only(&["ldar", "and", "ldr", "ret"])),
        (
            "slot_update",
            RetryOnly(&[
                "ldar", "b", "mov", "mov", "casal", "cmp", "mov", "b.eq", "ands", "b.eq", "cmp",
                "b.ne", "add", "and", "orr", "b", "mov", "b", "ands", "b.eq", "cmp", "b.ne", "lsr",
                "mov", "stp", "ret", "sub", "str", "mov", "str", "ret",
            ]),
        ),
        (
            "entry_update",
            RetryOnly(&[
                "ldar", "mov", "add", "tst", "and", "csel", "casal", "cmp", "mov", "b.ne", "tbz",
                "sub", "mov", "str", "str", "ret", "lsr", "stp", "ret",
            ]),
        ),
        (
            "child_update",
            RetryOnly(&["ldar", "mov", "casal", "cmp", "mov", "b.ne", "and", "and", "ret"]),
        ),
        (
            "locked_child_update",
            RetryOnly(&[
                "stp", "mov", "ldar", "mvn", "and", "and", "add", "mov", "casal", "cmp", "mov",
                "b.ne", "and", "and", "ubfx", "stp", "strb", "ldp", "ret",
            ]),
        ),
    ];

    /// Each load and update on `aarch64` with LSE2, `neoverse-v1` on Linux and macOS's floor,
    /// `apple-m1`, which reads with `ldapr`.
    const AARCH64_LSE2: &[(&str, Lowering)] = &[
        ("counted_head_load", Only(&["ldp", "dmb ishld", "ret"])),
        ("counted_head_update_version", Retry(&["ldp", "dmb ishld", "add", "caspal"])),
        ("marked_pair_store", InOrder(&["tbnz", "add", "dmb ish", "stp", "bl"])),
        ("packed_struct_load", Only(&["ldapr", "lsr", "and", "bfi", "ret"])),
        ("fieldless_enum_load", Only(&["ldaprb", "ret"])),
        ("niche_filling_enum_load", InOrder(&["ldaprb"])),
        ("field_load", Only(&["ldapr", "ubfx", "ret"])),
        ("head_load_top", Only(&["ldapr", "and", "ret"])),
        ("next_load_match", Only(&["ldapr", "cbz", "ldr", "ret", "mov", "ret"])),
        ("child_load_match", Only(&["ldapr", "and", "ubfiz", "ldr", "ret"])),
        ("child_load_leaf", Only(&["ldapr", "tbz", "mov", "ret", "ldr", "ret"])),
        ("child_load_branch", Only(&["ldapr", "tbz", "ldur", "ret", "mov", "ret"])),
        (
            "slot_load_match",
            Only(&[
                "ldapr", "ands", "b.eq", "cmp", "b.ne", "ubfx", "ret", "mov", "ret", "ldur", "ret",
            ]),
        ),
        ("entry_load_match", Only(&["ldapr", "tbz", "ldur", "ret", "ubfx", "ret"])),
        ("guarded_load_inner", Only(&["ldapr", "and", "ldr", "ret"])),
        (
            "slot_update",
            RetryOnly(&[
                "ldapr", "b", "mov", "mov", "casal", "cmp", "mov", "b.eq", "ands", "b.eq", "cmp",
                "b.ne", "add", "and", "orr", "b", "mov", "b", "ands", "b.eq", "cmp", "b.ne", "lsr",
                "mov", "stp", "ret", "sub", "str", "mov", "str", "ret",
            ]),
        ),
        (
            "child_update",
            RetryOnly(&["ldapr", "mov", "casal", "cmp", "mov", "b.ne", "and", "and", "ret"]),
        ),
    ];

    /// The rest on `aarch64` Linux with LSE2, `neoverse-v1`, which puts an update's
    /// instructions in its own order.
    const AARCH64_LINUX_LSE2: &[(&str, Lowering)] = &[
        (
            "entry_update",
            RetryOnly(&[
                "ldapr", "mov", "add", "tst", "and", "csel", "casal", "cmp", "mov", "b.ne", "tbz",
                "sub", "mov", "str", "str", "ret", "lsr", "stp", "ret",
            ]),
        ),
        (
            "locked_child_update",
            RetryOnly(&[
                "stp", "mov", "ldapr", "mvn", "and", "and", "add", "mov", "casal", "cmp", "mov",
                "b.ne", "and", "and", "stp", "ubfx", "strb", "ldp", "ret",
            ]),
        ),
    ];

    /// The rest on `aarch64` macOS, whose `apple-m1` puts an update's instructions
    /// in its own order too.
    const AARCH64_MACOS_LSE2: &[(&str, Lowering)] = &[
        (
            "entry_update",
            RetryOnly(&[
                "ldapr", "mov", "add", "and", "tst", "csel", "casal", "cmp", "mov", "b.ne", "tbz",
                "sub", "str", "mov", "str", "ret", "lsr", "stp", "ret",
            ]),
        ),
        (
            "locked_child_update",
            RetryOnly(&[
                "stp", "mov", "ldapr", "and", "mvn", "and", "add", "mov", "casal", "cmp", "mov",
                "b.ne", "and", "and", "stp", "ubfx", "strb", "ldp", "ret",
            ]),
        ),
    ];

    /// Each load and update on `x86_64`, Linux or macOS, at the `x86-64-v3` floor, whose BMI2
    /// masks with `bzhi`, and each field read-modify-write: one `lock` instruction, `lock bts` for
    /// a bit's test.
    const X86_64: &[(&str, Lowering)] = &[
        ("counted_head_load", Only(&["vmovdqa", "vmovq", "vpextrq", "retq"])),
        (
            "counted_head_load_rmw",
            Only(&["pushq", "xorl", "xorl", "xorl", "xorl", "lock cmpxchg16b", "popq", "retq"]),
        ),
        (
            "counted_head_compare_exchange",
            Only(&[
                "pushq",
                "movq",
                "xorl",
                "movq",
                "movq",
                "movq",
                "lock cmpxchg16b",
                "setne",
                "movq",
                "movq",
                "movq",
                "movq",
                "popq",
                "retq",
            ]),
        ),
        ("counted_head_update_version", Retry(&["vmovdqa", "leaq", "lock cmpxchg16b"])),
        ("marked_pair_store", Refuses(&["testb", "jne", "addq", "vmovdqa", "retq"])),
        (
            "marked_pair_compare_exchange",
            InOrder(&["orl", "testb", "jne", "lock cmpxchg16b", "callq"]),
        ),
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
        ("head_test_and_set_marked", Only(&["lock btsq", "setb", "retq"])),
        ("head_load_top", Only(&["movq", "andq", "retq"])),
        ("next_load_match", Only(&["movq", "testq", "je", "movq", "retq", "movl", "retq"])),
        ("child_load_match", Only(&["movq", "movq", "andq", "andl", "movq", "retq"])),
        ("child_load_leaf", Only(&["movq", "testb", "je", "xorl", "retq", "movq", "retq"])),
        ("child_load_branch", Only(&["movq", "testb", "je", "movq", "retq", "xorl", "retq"])),
        (
            "slot_load_match",
            Only(&[
                "movq", "movq", "andq", "je", "cmpl", "jne", "shrq", "movl", "retq", "movl",
                "retq", "movq", "retq",
            ]),
        ),
        (
            "entry_load_match",
            Only(&["movq", "testb", "je", "movq", "retq", "shrq", "movl", "retq"]),
        ),
        ("guarded_load_inner", Only(&["movq", "andq", "movq", "retq"])),
        ("slot_store_inline", Only(&["movl", "leaq", "movq", "retq"])),
        ("slot_store_node", Refuses(&["testb", "jne", "addq", "movq", "retq"])),
        ("next_store_node", Only(&["movq", "retq"])),
        ("marked_store", Refuses(&["testb", "jne", "incq", "movq", "retq"])),
        ("guarded_store", Refuses(&["testb", "jne", "addq", "movq", "retq"])),
        ("locked_slot_store_node", Refuses(&["testb", "jne", "addq", "movq", "retq"])),
        ("head_compare_exchange", Refuses(&["orl", "testb", "jne", "lock cmpxchgq", "retq"])),
        (
            "slot_update",
            RetryOnly(&[
                "movq",
                "movabsq",
                "jmp",
                "movl",
                "lock cmpxchgq",
                "je",
                "movq",
                "andq",
                "je",
                "cmpl",
                "jne",
                "leaq",
                "andq",
                "incq",
                "jmp",
                "movq",
                "jmp",
                "movq",
                "andq",
                "je",
                "cmpl",
                "jne",
                "shrq",
                "movl",
                "movl",
                "movl",
                "movq",
                "retq",
                "xorl",
                "movl",
                "movq",
                "retq",
                "addq",
                "movq",
                "movl",
                "movl",
                "movq",
                "retq",
            ]),
        ),
        (
            "entry_update",
            RetryOnly(&[
                "movq",
                "movabsq",
                "jmp",
                "lock cmpxchgq",
                "je",
                "movq",
                "testb",
                "jne",
                "leaq",
                "andq",
                "jmp",
                "testb",
                "je",
                "decq",
                "movq",
                "movl",
                "movl",
                "movq",
                "retq",
                "shrq",
                "movl",
                "xorl",
                "movl",
                "movq",
                "retq",
            ]),
        ),
        (
            "child_update",
            RetryOnly(&[
                "movq",
                "movq",
                "lock cmpxchgq",
                "movq",
                "jne",
                "movl",
                "andl",
                "andq",
                "retq",
            ]),
        ),
        ("locked_slot_set_locked", Only(&["lock orq", "retq"])),
        ("locked_slot_test_and_set_locked", Only(&["lock btsq", "setb", "retq"])),
        ("guarded_set_locked", Only(&["lock orq", "retq"])),
        ("guarded_test_and_set_marked", Only(&["lock btsq", "setb", "retq"])),
    ];

    /// The rest on `x86_64` Linux, where a function that calls aligns the stack with a push.
    const X86_64_LINUX_PUSH: &[(&str, Lowering)] = &[(
        "locked_child_update",
        RetryOnly(&[
            "movq",
            "movb",
            "testb",
            "je",
            "movl",
            "movq",
            "andq",
            "notl",
            "andl",
            "addq",
            "lock cmpxchgq",
            "jne",
            "movl",
            "andl",
            "movq",
            "andq",
            "shrb",
            "andb",
            "movq",
            "movq",
            "movb",
            "movq",
            "retq",
            "pushq",
            "leaq",
            "callq",
        ]),
    )];

    /// The rest on `x86_64` macOS, where every function's frame record aligns the stack.
    const X86_64_MACOS_FRAME: &[(&str, Lowering)] = &[(
        "locked_child_update",
        RetryOnly(&[
            "movq",
            "movb",
            "testb",
            "je",
            "movl",
            "movq",
            "andq",
            "notl",
            "andl",
            "addq",
            "lock cmpxchgq",
            "jne",
            "movl",
            "andl",
            "movq",
            "andq",
            "shrb",
            "andb",
            "movq",
            "movq",
            "movb",
            "movq",
            "retq",
            "leaq",
            "callq",
        ]),
    )];

    #[test]
    fn aarch64_linux_lowers_each_derived_operation_to_its_instructions() {
        lowers_as_expected(AARCH64_LINUX, None, &[AARCH64, AARCH64_FLOOR]);
    }

    #[test]
    fn aarch64_linux_with_lse2_lowers_each_derived_operation_to_its_instructions() {
        lowers_as_expected(
            AARCH64_LINUX,
            Some("neoverse-v1"),
            &[AARCH64, AARCH64_LSE2, AARCH64_LINUX_LSE2],
        );
    }

    #[test]
    fn aarch64_macos_lowers_each_derived_operation_to_its_instructions() {
        lowers_as_expected(AARCH64_MACOS, None, &[AARCH64, AARCH64_LSE2, AARCH64_MACOS_LSE2]);
    }

    #[test]
    fn x86_64_linux_lowers_each_derived_operation_to_its_instructions() {
        lowers_as_expected(X86_64_LINUX, None, &[X86_64, X86_64_LINUX_PUSH]);
    }

    #[test]
    fn x86_64_macos_lowers_each_derived_operation_to_its_instructions() {
        lowers_as_expected(X86_64_MACOS, None, &[X86_64, X86_64_MACOS_FRAME]);
    }

    #[test]
    fn x86_64_without_cmpxchg16b_refuses_each_shape_of_two_words_once() {
        let stderr = refused(X86_64_LINUX, Some("x86-64"), "x86-64-refused");
        let errors: Vec<&str> = stderr.lines().filter(|line| line.starts_with("error")).collect();
        let refusal = |name: &str| {
            format!(
                "`atomix_codegen::without_cmpxchg16b::{name}` needs two words, but an atomic holds \
                 at most one: build with `-C target-cpu=x86-64-v2` or newer for two"
            )
        };
        assert_eq!(
            errors,
            [
                format!("error: {}", refusal("StatedLink")),
                format!("error: {}", refusal("MarkedPair")),
                format!("error: {}", refusal("Chunk")),
                format!("error[E0080]: evaluation panicked: {}", refusal("Head")),
                "error: could not compile `atomix-codegen` (lib) due to 4 previous errors"
                    .to_owned(),
            ],
            "each shape refused once, at its derive, naming the CPU it needs:\n{stderr}"
        );
    }
}
