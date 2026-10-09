//! What each operation lowers to, per target, read from the assembly of `tests/codegen`.
//!
//! The fixture builds for Linux and macOS with the repository's CPU floor and, on `aarch64` Linux,
//! with LSE2 too; and for Android and Windows at their default CPUs, as a dependent builds it:
//! Android's has no LSE, and Windows' on `x86_64` has `cmpxchg16b` but no AVX. Each operation is
//! the instructions and barriers its name promises, with no compare-exchange loop but `update`'s,
//! and each target refuses each operation it lacks: `x86_64` those only `aarch64` has, and
//! `aarch64` each 128-bit one it has no instruction for. At Android's default CPU, without LSE,
//! each read-modify-write is an LL/SC loop, which retries only where the core lost its reservation
//! between its load and its store, never because the value differs, so it counts as one operation,
//! as LSE's instruction does. A ranged integer's range reaches LLVM, so a comparison outside it
//! folds to a constant, and a conversion to or from deranged's of the same bounds, which saturates,
//! is a move at most.
//!
//! A field's operation is its whole word's, its operand confined to the field: one LSE instruction
//! on `aarch64`, one `lock` instruction on `x86_64`, where a bit's test is `lock bts` and its kin
//! with the position an immediate, wherever the bit goes, an `Option` too. A field of an
//! arbitrary-int integer lowers as a built-in integer's does. A pointer word's tag lowers as a
//! field does, on the pointer itself; its load masks the tags off with an `and`, and the load of a
//! word through its pointer's place masks the outer word's tags off alone. A word's store tests its
//! pointer against every tag bit in one test, a word's over a word too, and an exchange tests both
//! its pointers in one, each with one cold refusal off a fast path that saves no frame record on
//! Linux and macOS; at Android's default CPU, a word's compare-exchange saves its frame record
//! first, and on `x86_64` Windows a function that calls the refusal reserves its stack first. A
//! decode clears the tags with the pointer's `mask`, so a loop tests no pointer it decoded: a
//! word's `update` tests, calls and saves nothing, and a Treiber stack's pop tests only the next
//! node's pointer, which it reads from the node, and saves a frame record once, for its refusal.
//!
//! An atomic's bit chosen at run time is `ldset` and its kin on `aarch64`, and `lock bts` and its
//! kin on `x86_64`, after the `and` that keeps a memory `bts` within the word, wherever the bit
//! goes, an `Option` too. A constant bit, at either end or past the width too, is the same on
//! `x86_64` with its position, taken modulo the width, moved into a register; `aarch64` folds it
//! into the mask it moves, and tests the bit with a shift or a mask. A bit whose value is discarded
//! is `ldset` with no test, and still `lock bts` on `x86_64`, whose `asm!` LLVM keeps.
//!
//! A double word, two pointers or a slice's pointer and length, lowers as a 128-bit integer does,
//! each pointer's exposure no instruction: its load and store are `ldp` and `stp` with LSE2, and
//! `vmovdqa` with AVX; its read without them, and its exchange, are `casp` and `cmpxchg16b`, on
//! `x86_64` `u128`'s instructions exactly, with no branch or `cmov` to join the words found. Where
//! `x86_64` has no `cmpxchg16b`, two words are refused as 128 bits are, naming the CPU they need.

// Miri cannot run the compiler, and loom's atomics are not what ships.
#![cfg(on_hardware)]

#[cfg(test)]
mod testing;

#[cfg(test)]
mod tests {
    use crate::testing::codegen::Lowering::{self, InOrder, Only, Refuses, Retry, RetryOnly};
    use crate::testing::codegen::{
        AARCH64_ANDROID, AARCH64_LINUX, AARCH64_MACOS, X86_64_LINUX, X86_64_MACOS, X86_64_WINDOWS,
        lowers_as_expected, refused,
    };

    /// Each function of the fixture on `aarch64` that lowers the same at every CPU: a store, a
    /// fence, a ranged integer's comparison and conversions, and a tagged pointer's store, whose
    /// test's cold refusal saves the frame record off the fast path.
    const AARCH64: &[(&str, Lowering)] = &[
        ("u64_store", InOrder(&["stlr"])),
        ("store_store_fence", InOrder(&["str", "dmb ishst", "str"])),
        ("seq_cst_compiler_fence", Only(&["ret"])),
        ("acquire_fence", Only(&["dmb ishld", "ret"])),
        ("release_fence", Only(&["dmb ish", "ret"])),
        ("seq_cst_fence", Only(&["dmb ish", "ret"])),
        ("ranged_below_min", Only(&["mov", "ret"])),
        ("ranged_to_deranged", Only(&["ret"])),
        ("ranged_from_deranged", Only(&["ret"])),
        ("word_store", Refuses(&["tbnz", "add", "stlr", "ret"])),
        ("nested_store", Refuses(&["tst", "b.ne", "add", "stlr", "ret"])),
    ];

    /// Each read-modify-write on `aarch64` with LSE, the same at the floor, with LSE2 and on macOS:
    /// a field operation is one LSE instruction, then a shift for a bit or a count.
    const AARCH64_LSE: &[(&str, Lowering)] = &[
        ("u64_swap", InOrder(&["swpal"])),
        ("u64_fetch_add", InOrder(&["ldaddal"])),
        ("u64_fetch_sub", InOrder(&["neg", "ldaddal"])),
        ("u64_compare_exchange", InOrder(&["casal"])),
        ("u64_fetch_add_discarded", InOrder(&["ldadd"])),
        ("u64_fetch_sub_discarded", InOrder(&["neg", "ldadd"])),
        ("u64_or", InOrder(&["ldsetl"])),
        ("u64_and", InOrder(&["mvn", "ldclrl"])),
        ("u64_xor", InOrder(&["ldeorl"])),
        ("u64_not", InOrder(&["ldeorl"])),
        ("u64_fetch_and", InOrder(&["mvn", "ldclral"])),
        ("u64_fetch_or", InOrder(&["ldsetal"])),
        ("u64_fetch_xor", InOrder(&["ldeoral"])),
        ("u64_fetch_not", InOrder(&["ldeoral"])),
        ("u64_fetch_max_discarded", InOrder(&["ldumax"])),
        ("u64_fetch_min_discarded", InOrder(&["ldumin"])),
        ("u64_fetch_max", InOrder(&["ldumaxal"])),
        ("i64_fetch_max_discarded", InOrder(&["ldsmax"])),
        ("i64_fetch_min", InOrder(&["ldsminal"])),
        ("bool_or", InOrder(&["ldsetlb"])),
        ("ptr_fetch_byte_add_discarded", InOrder(&["ldadd"])),
        ("ptr_fetch_ptr_sub", InOrder(&["neg", "ldaddal"])),
        ("u128_compare_exchange", InOrder(&["caspal"])),
        ("u128_load_rmw", InOrder(&["caspa"])),
        ("pair_load_rmw", Only(&["mov", "mov", "caspa", "mov", "mov", "ret"])),
        ("pair_compare_exchange", InOrder(&["caspal", "cmp", "ccmp", "cset"])),
        ("slice_compare_exchange", InOrder(&["caspal", "cmp", "ccmp", "cset"])),
        ("field_set", Only(&["mov", "ldsetl", "ret"])),
        ("field_clear", Only(&["mov", "ldclrl", "ret"])),
        ("field_toggle", Only(&["mov", "ldeorl", "ret"])),
        ("field_store", InOrder(&["tbz", "ldsetl", "ldclrl"])),
        ("field_test_and_set", Only(&["mov", "ldsetal", "ubfx", "ret"])),
        ("field_test_and_set_in_some", Only(&["mov", "ldsetal", "ubfx", "ret"])),
        ("field_test_and_set_through_map", InOrder(&["ldsetal", "ubfx"])),
        ("flags_or", Only(&["and", "lsl", "ldsetl", "ret"])),
        ("flags_and", Only(&["mov", "bic", "ldclrl", "ret"])),
        ("flags_xor", Only(&["and", "lsl", "ldeorl", "ret"])),
        ("flags_not", Only(&["mov", "ldeorl", "ret"])),
        ("flags_or_constant", Only(&["mov", "ldsetl", "ret"])),
        ("nested_clear", Only(&["mov", "ldclrl", "ret"])),
        ("nested_flags_or", Only(&["ubfiz", "ldsetl", "ret"])),
        ("top_fetch_add", InOrder(&["lsl", "ldaddal"])),
        ("top_fetch_sub", InOrder(&["neg", "ldaddal"])),
        ("top_fetch_add_discarded", Only(&["lsl", "ldadd", "ret"])),
        ("top_fetch_add_count", Only(&["mov", "ldaddal", "lsr", "ret"])),
        ("u61_top_fetch_add_references", Only(&["mov", "ldaddal", "lsr", "ret"])),
        ("u4_flags_or", Only(&["and", "ldsetlh", "ret"])),
        ("field_fetch_or", InOrder(&["mov", "ldsetal"])),
        ("flags_fetch_and", InOrder(&["bic", "ldclral"])),
        ("flags_fetch_or", InOrder(&["lsl", "ldsetal"])),
        ("flags_fetch_xor", InOrder(&["lsl", "ldeoral"])),
        ("flags_fetch_not", InOrder(&["mov", "ldeoral"])),
        ("ends8_low_test_and_set", Only(&["mov", "ldsetalb", "and", "ret"])),
        ("ends8_top_test_and_set", Only(&["mov", "ldsetalb", "lsr", "ret"])),
        ("ends16_low_test_and_set", Only(&["mov", "ldsetalh", "and", "ret"])),
        ("ends16_middle_test_and_set", Only(&["mov", "ldsetalh", "ubfx", "ret"])),
        ("ends16_top_test_and_set", Only(&["mov", "ldsetalh", "lsr", "ret"])),
        ("ends32_low_test_and_set", Only(&["mov", "ldsetal", "and", "ret"])),
        ("ends32_middle_test_and_set", Only(&["mov", "ldsetal", "ubfx", "ret"])),
        ("ends32_top_test_and_set", Only(&["mov", "ldsetal", "lsr", "ret"])),
        ("ends64_low_test_and_set", Only(&["mov", "ldsetal", "and", "ret"])),
        ("ends64_middle_test_and_set", Only(&["mov", "ldsetal", "ubfx", "ret"])),
        ("ends64_middle_test_and_clear", Only(&["mov", "ldclral", "ubfx", "ret"])),
        ("ends64_middle_test_and_toggle", Only(&["mov", "ldeoral", "ubfx", "ret"])),
        ("ends64_top_test_and_set", Only(&["mov", "ldsetal", "lsr", "ret"])),
        ("ends64_top_test_and_clear", Only(&["mov", "ldclral", "lsr", "ret"])),
        ("ends64_top_test_and_toggle", Only(&["mov", "ldeoral", "lsr", "ret"])),
        ("ends64_low_test_and_clear_in_some", Only(&["mov", "ldclral", "and", "ret"])),
        ("ends64_top_test_and_toggle_in_some", Only(&["mov", "ldeoral", "lsr", "ret"])),
        ("u64_bit_set", Only(&["mov", "lsl", "ldsetal", "tst", "cset", "ret"])),
        ("u64_bit_clear", Only(&["mov", "lsl", "ldclral", "tst", "cset", "ret"])),
        ("u64_bit_toggle", Only(&["mov", "lsl", "ldeoral", "tst", "cset", "ret"])),
        ("u64_bit_set_0", Only(&["mov", "ldsetal", "and", "ret"])),
        ("u64_bit_set_5", Only(&["mov", "ldsetal", "ubfx", "ret"])),
        ("u64_bit_set_63", Only(&["mov", "ldsetal", "lsr", "ret"])),
        ("u64_bit_clear_63", Only(&["mov", "ldclral", "lsr", "ret"])),
        ("u64_bit_set_64", Only(&["mov", "ldsetal", "and", "ret"])),
        ("u64_bit_set_discarded", Only(&["mov", "lsl", "ldsetal", "ret"])),
        ("u64_bit_set_in_some", Only(&["mov", "lsl", "ldsetal", "lsr", "and", "ret"])),
        ("u64_bit_clear_in_some", Only(&["mov", "lsl", "ldclral", "lsr", "and", "ret"])),
        ("u64_bit_toggle_in_some", Only(&["mov", "lsl", "ldeoral", "lsr", "and", "ret"])),
        ("u64_bit_set_63_in_some", Only(&["mov", "ldsetal", "lsr", "ret"])),
        ("u64_bit_set_through_map", InOrder(&["lsl", "ldsetal", "lsr", "and"])),
        ("u32_bit_set", Only(&["mov", "lsl", "ldsetal", "tst", "cset", "ret"])),
        ("u32_bit_set_31", Only(&["mov", "ldsetal", "lsr", "ret"])),
        ("u16_bit_set", InOrder(&["and", "lsl", "ldsetalh", "tst", "cset"])),
        ("u16_bit_set_in_some", InOrder(&["and", "lsl", "ldsetalh", "lsr", "and"])),
        ("i32_bit_set", Only(&["mov", "lsl", "ldsetal", "tst", "cset", "ret"])),
        ("i64_bit_set", Only(&["mov", "lsl", "ldsetal", "tst", "cset", "ret"])),
        ("isize_bit_set", Only(&["mov", "lsl", "ldsetal", "tst", "cset", "ret"])),
        ("u8_bit_set", InOrder(&["and", "lsl", "ldsetalb", "tst", "cset"])),
        ("tag_set", Only(&["mov", "ldsetl", "ret"])),
        ("tag_clear", Only(&["mov", "ldclrl", "ret"])),
        ("tag_toggle", Only(&["mov", "ldeorl", "ret"])),
        ("bit0_test_and_set", Only(&["mov", "ldsetal", "and", "ret"])),
        ("bit0_test_and_set_in_some", Only(&["mov", "ldsetal", "and", "ret"])),
        ("bit0_test_and_clear", Only(&["mov", "ldclral", "and", "ret"])),
        ("bit0_test_and_toggle", Only(&["mov", "ldeoral", "and", "ret"])),
        ("bit1_test_and_set", Only(&["mov", "ldsetal", "ubfx", "ret"])),
        ("bit1_test_and_clear", Only(&["mov", "ldclral", "ubfx", "ret"])),
        ("bit1_test_and_toggle", Only(&["mov", "ldeoral", "ubfx", "ret"])),
        ("bit2_test_and_set", Only(&["mov", "ldsetal", "ubfx", "ret"])),
        ("bit2_test_and_clear", Only(&["mov", "ldclral", "ubfx", "ret"])),
        ("bit2_test_and_toggle", Only(&["mov", "ldeoral", "ubfx", "ret"])),
        ("tag_flags_or", Only(&["ubfiz", "ldsetl", "ret"])),
        ("tag_flags_and", Only(&["mvn", "and", "ldclrl", "ret"])),
        ("tag_flags_xor", Only(&["ubfiz", "ldeorl", "ret"])),
        ("tag_flags_not", Only(&["mov", "ldeorl", "ret"])),
        ("nested_tag_set", Only(&["mov", "ldsetl", "ret"])),
        ("tag_fetch_or", Only(&["mov", "ldsetal", "and", "and", "ret"])),
        ("word_compare_exchange", Refuses(&["orr", "tst", "b.ne", "casal", "ret"])),
        ("nested_compare_exchange", Refuses(&["orr", "tst", "b.ne", "casal", "ret"])),
    ];

    /// Each load on `aarch64` without LSE2, at the `+lse` floor and at Android's default CPU,
    /// neither of which has `RCpc`'s `ldapr`: an Acquire load is `ldar`. A load of a `char`, an
    /// `Option` or a ranged integer, as every load, decodes without a check.
    const AARCH64_WITHOUT_LSE2: &[(&str, Lowering)] = &[
        ("u64_load", InOrder(&["ldar"])),
        ("char_load", Only(&["ldar", "ret"])),
        ("option_load", Only(&["ldar", "ret"])),
        ("ranged_load", Only(&["ldar", "ret"])),
        ("field_load", Only(&["ldar", "ubfx", "ret"])),
        ("flags_load", Only(&["ldar", "lsr", "ret"])),
        ("quantity_load", Only(&["ldar", "ret"])),
        ("word_load", Only(&["ldar", "and", "and", "ubfx", "str", "strb", "strb", "ret"])),
        ("word_load_top", Only(&["ldar", "and", "ret"])),
        ("option_word_load", Only(&["ldar", "mov", "and", "cmp", "and", "csel", "ret"])),
        ("tag_load", Only(&["ldar", "and", "ret"])),
        ("pointer_place_load", Only(&["ldar", "and", "and", "ret"])),
    ];

    /// Each update on `aarch64` at the `+lse` floor, which has no LSE2: with no 128-bit load,
    /// `update` reads with a compare-exchange.
    const AARCH64_FLOOR: &[(&str, Lowering)] = &[
        ("u128_update", Retry(&["caspa", "caspal"])),
        ("pair_update", Retry(&["caspa", "caspal"])),
        ("quantity_update", Retry(&["ldar", "casal"])),
        ("tag_update", Retry(&["ldar", "casal"])),
        (
            "word_update",
            RetryOnly(&[
                "ldar", "mov", "and", "and", "eor", "orr", "casal", "cmp", "mov", "b.ne", "and",
                "and", "ubfx", "str", "strb", "strb", "ret",
            ]),
        ),
        (
            "word_pop",
            RetryOnly(&[
                "stp", "mov", "ldar", "mov", "ands", "b.eq", "ldr", "tst", "b.ne", "add", "and",
                "and", "and", "add", "add", "add", "mov", "casal", "cmp", "b.ne", "ldp", "ret",
                "adrp", "add", "bl",
            ]),
        ),
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
        ("pair_load", Only(&["ldp", "dmb ishld", "ret"])),
        ("pair_store", Only(&["dmb ish", "stp", "ret"])),
        ("pair_update", Retry(&["ldp", "dmb ishld", "caspal"])),
        ("field_load", Only(&["ldapr", "ubfx", "ret"])),
        ("flags_load", Only(&["ldapr", "lsr", "ret"])),
        ("quantity_load", Only(&["ldapr", "ret"])),
        ("quantity_update", Retry(&["ldapr", "casal"])),
        ("word_load_top", Only(&["ldapr", "and", "ret"])),
        ("tag_load", Only(&["ldapr", "and", "ret"])),
        ("tag_update", Retry(&["ldapr", "casal"])),
        ("pointer_place_load", Only(&["ldapr", "and", "and", "ret"])),
    ];

    /// The rest on `aarch64` Linux with LSE2, `neoverse-v1`, which puts a word's instructions
    /// in the floor's order.
    const AARCH64_LINUX_LSE2: &[(&str, Lowering)] = &[
        ("word_load", Only(&["ldapr", "and", "and", "ubfx", "str", "strb", "strb", "ret"])),
        ("option_word_load", Only(&["ldapr", "mov", "and", "cmp", "and", "csel", "ret"])),
        (
            "word_update",
            RetryOnly(&[
                "ldapr", "mov", "and", "and", "eor", "orr", "casal", "cmp", "mov", "b.ne", "and",
                "and", "ubfx", "str", "strb", "strb", "ret",
            ]),
        ),
        (
            "word_pop",
            RetryOnly(&[
                "stp", "mov", "ldapr", "mov", "ands", "b.eq", "ldr", "tst", "b.ne", "add", "and",
                "and", "and", "add", "add", "add", "mov", "casal", "cmp", "b.ne", "ldp", "ret",
                "adrp", "add", "bl",
            ]),
        ),
    ];

    /// The rest on `aarch64` macOS, whose `apple-m1` puts a word's instructions in its own order.
    const AARCH64_MACOS_LSE2: &[(&str, Lowering)] = &[
        ("word_load", Only(&["ldapr", "and", "and", "str", "strb", "ubfx", "strb", "ret"])),
        ("option_word_load", Only(&["ldapr", "and", "cmp", "mov", "csel", "and", "ret"])),
        (
            "word_update",
            RetryOnly(&[
                "ldapr", "mov", "and", "and", "eor", "orr", "casal", "cmp", "mov", "b.ne", "and",
                "and", "str", "strb", "ubfx", "strb", "ret",
            ]),
        ),
        (
            "word_pop",
            RetryOnly(&[
                "stp", "mov", "mov", "ldapr", "ands", "b.eq", "ldr", "tst", "b.ne", "and", "add",
                "and", "add", "add", "and", "add", "mov", "casal", "cmp", "b.ne", "ldp", "ret",
                "adrp", "add", "bl",
            ]),
        ),
    ];

    /// Each read-modify-write and update on `aarch64` without LSE, at Android's default CPU: an
    /// LL/SC loop, its load-linked Acquire and its store-conditional Release as the ordering asks,
    /// which retries by its store-conditional's branch back. A compare-exchange is an LL/SC loop
    /// that compares too, so `update` retries one. A word's compare-exchange saves its frame
    /// record on the fast path.
    const AARCH64_WITHOUT_LSE: &[(&str, Lowering)] = &[
        ("u64_swap", InOrder(&["ldaxr", "stlxr"])),
        ("u64_fetch_add", InOrder(&["ldaxr", "add", "stlxr"])),
        ("u64_fetch_sub", InOrder(&["ldaxr", "sub", "stlxr"])),
        ("u64_compare_exchange", InOrder(&["ldaxr", "cmp", "stlxr"])),
        ("u64_fetch_add_discarded", InOrder(&["ldxr", "add", "stxr"])),
        ("u64_fetch_sub_discarded", InOrder(&["ldxr", "sub", "stxr"])),
        ("u64_or", InOrder(&["ldxr", "orr", "stlxr"])),
        ("u64_and", InOrder(&["ldxr", "and", "stlxr"])),
        ("u64_xor", InOrder(&["ldxr", "eor", "stlxr"])),
        ("u64_not", InOrder(&["ldxr", "mvn", "stlxr"])),
        ("u64_fetch_and", InOrder(&["ldaxr", "and", "stlxr"])),
        ("u64_fetch_or", InOrder(&["ldaxr", "orr", "stlxr"])),
        ("u64_fetch_xor", InOrder(&["ldaxr", "eor", "stlxr"])),
        ("u64_fetch_not", InOrder(&["ldaxr", "mvn", "stlxr"])),
        ("u64_fetch_max_discarded", InOrder(&["ldxr", "cmp", "stxr"])),
        ("u64_fetch_min_discarded", InOrder(&["ldxr", "cmp", "stxr"])),
        ("u64_fetch_max", InOrder(&["ldaxr", "cmp", "stlxr"])),
        ("i64_fetch_max_discarded", InOrder(&["ldxr", "cmp", "stxr"])),
        ("i64_fetch_min", InOrder(&["ldaxr", "cmp", "stlxr"])),
        ("bool_or", InOrder(&["ldxrb", "orr", "stlxrb"])),
        ("ptr_fetch_byte_add_discarded", InOrder(&["ldxr", "add", "stxr"])),
        ("ptr_fetch_ptr_sub", InOrder(&["ldaxr", "sub", "stlxr"])),
        ("u128_compare_exchange", InOrder(&["ldaxp", "cmp", "stlxp", "stlxp"])),
        ("u128_load_rmw", InOrder(&["ldaxp", "cmp", "stxp", "stxp"])),
        (
            "pair_load_rmw",
            Only(&[
                "ldaxp", "cmp", "cset", "cmp", "cinc", "cbz", "stxp", "cbnz", "b", "stxp", "cbnz",
                "mov", "ret",
            ]),
        ),
        ("pair_compare_exchange", InOrder(&["ldaxp", "cmp", "stlxp", "stlxp"])),
        ("slice_compare_exchange", InOrder(&["ldaxp", "cmp", "stlxp", "stlxp"])),
        ("field_set", Only(&["ldxr", "orr", "stlxr", "cbnz", "ret"])),
        ("field_clear", Only(&["ldxr", "and", "stlxr", "cbnz", "ret"])),
        ("field_toggle", Only(&["ldxr", "eor", "stlxr", "cbnz", "ret"])),
        ("field_store", InOrder(&["ldxr", "orr", "stlxr", "ldxr", "and", "stlxr"])),
        ("field_test_and_set", Only(&["ldaxr", "orr", "stlxr", "cbnz", "ubfx", "ret"])),
        ("field_test_and_set_in_some", Only(&["ldaxr", "orr", "stlxr", "cbnz", "ubfx", "ret"])),
        ("field_test_and_set_through_map", InOrder(&["ldaxr", "orr", "stlxr"])),
        ("flags_or", Only(&["and", "lsl", "ldxr", "orr", "stlxr", "cbnz", "ret"])),
        ("flags_and", Only(&["lsl", "orr", "ldxr", "and", "stlxr", "cbnz", "ret"])),
        ("flags_xor", Only(&["and", "lsl", "ldxr", "eor", "stlxr", "cbnz", "ret"])),
        ("flags_not", Only(&["ldxr", "eor", "stlxr", "cbnz", "ret"])),
        ("flags_or_constant", Only(&["ldxr", "orr", "stlxr", "cbnz", "ret"])),
        ("nested_clear", Only(&["ldxr", "and", "stlxr", "cbnz", "ret"])),
        ("nested_flags_or", Only(&["ubfiz", "ldxr", "orr", "stlxr", "cbnz", "ret"])),
        ("top_fetch_add", InOrder(&["ldaxr", "add", "stlxr"])),
        ("top_fetch_sub", InOrder(&["ldaxr", "sub", "stlxr"])),
        ("top_fetch_add_discarded", Only(&["lsl", "ldxr", "add", "stxr", "cbnz", "ret"])),
        ("top_fetch_add_count", Only(&["mov", "ldaxr", "add", "stlxr", "cbnz", "lsr", "ret"])),
        ("u61_top_fetch_add_references", Only(&["ldaxr", "add", "stlxr", "cbnz", "lsr", "ret"])),
        ("u4_flags_or", Only(&["and", "ldxrh", "orr", "stlxrh", "cbnz", "ret"])),
        ("field_fetch_or", InOrder(&["ldaxr", "orr", "stlxr"])),
        ("flags_fetch_and", InOrder(&["ldaxr", "and", "stlxr"])),
        ("flags_fetch_or", InOrder(&["ldaxr", "orr", "stlxr"])),
        ("flags_fetch_xor", InOrder(&["ldaxr", "eor", "stlxr"])),
        ("flags_fetch_not", InOrder(&["ldaxr", "eor", "stlxr"])),
        ("ends8_low_test_and_set", Only(&["ldaxrb", "orr", "stlxrb", "cbnz", "and", "ret"])),
        (
            "ends8_top_test_and_set",
            Only(&["ldaxrb", "orr", "stlxrb", "cbnz", "sxtb", "ubfx", "ret"]),
        ),
        ("ends16_low_test_and_set", Only(&["ldaxrh", "orr", "stlxrh", "cbnz", "and", "ret"])),
        ("ends16_middle_test_and_set", Only(&["ldaxrh", "orr", "stlxrh", "cbnz", "ubfx", "ret"])),
        (
            "ends16_top_test_and_set",
            Only(&["ldaxrh", "orr", "stlxrh", "cbnz", "sxth", "ubfx", "ret"]),
        ),
        ("ends32_low_test_and_set", Only(&["ldaxr", "orr", "stlxr", "cbnz", "and", "ret"])),
        ("ends32_middle_test_and_set", Only(&["ldaxr", "orr", "stlxr", "cbnz", "ubfx", "ret"])),
        ("ends32_top_test_and_set", Only(&["ldaxr", "orr", "stlxr", "cbnz", "lsr", "ret"])),
        ("ends64_low_test_and_set", Only(&["ldaxr", "orr", "stlxr", "cbnz", "and", "ret"])),
        ("ends64_middle_test_and_set", Only(&["ldaxr", "orr", "stlxr", "cbnz", "ubfx", "ret"])),
        ("ends64_middle_test_and_clear", Only(&["ldaxr", "and", "stlxr", "cbnz", "ubfx", "ret"])),
        ("ends64_middle_test_and_toggle", Only(&["ldaxr", "eor", "stlxr", "cbnz", "ubfx", "ret"])),
        ("ends64_top_test_and_set", Only(&["ldaxr", "orr", "stlxr", "cbnz", "lsr", "ret"])),
        ("ends64_top_test_and_clear", Only(&["ldaxr", "and", "stlxr", "cbnz", "lsr", "ret"])),
        ("ends64_top_test_and_toggle", Only(&["ldaxr", "eor", "stlxr", "cbnz", "lsr", "ret"])),
        (
            "ends64_low_test_and_clear_in_some",
            Only(&["ldaxr", "and", "stlxr", "cbnz", "and", "ret"]),
        ),
        (
            "ends64_top_test_and_toggle_in_some",
            Only(&["ldaxr", "eor", "stlxr", "cbnz", "lsr", "ret"]),
        ),
        (
            "u64_bit_set",
            Only(&["mov", "lsl", "ldaxr", "orr", "stlxr", "cbnz", "tst", "cset", "ret"]),
        ),
        (
            "u64_bit_clear",
            Only(&["mov", "lsl", "ldaxr", "bic", "stlxr", "cbnz", "tst", "cset", "ret"]),
        ),
        (
            "u64_bit_toggle",
            Only(&["mov", "lsl", "ldaxr", "eor", "stlxr", "cbnz", "tst", "cset", "ret"]),
        ),
        ("u64_bit_set_0", Only(&["ldaxr", "orr", "stlxr", "cbnz", "and", "ret"])),
        ("u64_bit_set_5", Only(&["ldaxr", "orr", "stlxr", "cbnz", "ubfx", "ret"])),
        ("u64_bit_set_63", Only(&["ldaxr", "orr", "stlxr", "cbnz", "lsr", "ret"])),
        ("u64_bit_clear_63", Only(&["ldaxr", "and", "stlxr", "cbnz", "lsr", "ret"])),
        ("u64_bit_set_64", Only(&["ldaxr", "orr", "stlxr", "cbnz", "and", "ret"])),
        ("u64_bit_set_discarded", Only(&["mov", "lsl", "ldaxr", "orr", "stlxr", "cbnz", "ret"])),
        (
            "u64_bit_set_in_some",
            Only(&["mov", "lsl", "and", "ldaxr", "orr", "stlxr", "cbnz", "lsr", "and", "ret"]),
        ),
        (
            "u64_bit_clear_in_some",
            Only(&["mov", "lsl", "and", "ldaxr", "bic", "stlxr", "cbnz", "lsr", "and", "ret"]),
        ),
        (
            "u64_bit_toggle_in_some",
            Only(&["mov", "lsl", "and", "ldaxr", "eor", "stlxr", "cbnz", "lsr", "and", "ret"]),
        ),
        ("u64_bit_set_63_in_some", Only(&["ldaxr", "orr", "stlxr", "cbnz", "lsr", "ret"])),
        ("u64_bit_set_through_map", InOrder(&["ldaxr", "orr", "stlxr"])),
        (
            "u32_bit_set",
            Only(&["mov", "lsl", "ldaxr", "orr", "stlxr", "cbnz", "tst", "cset", "ret"]),
        ),
        ("u32_bit_set_31", Only(&["ldaxr", "orr", "stlxr", "cbnz", "lsr", "ret"])),
        ("u16_bit_set", InOrder(&["ldaxrh", "orr", "stlxrh"])),
        ("u16_bit_set_in_some", InOrder(&["ldaxrh", "orr", "stlxrh"])),
        (
            "i32_bit_set",
            Only(&["mov", "lsl", "ldaxr", "orr", "stlxr", "cbnz", "tst", "cset", "ret"]),
        ),
        (
            "i64_bit_set",
            Only(&["mov", "lsl", "ldaxr", "orr", "stlxr", "cbnz", "tst", "cset", "ret"]),
        ),
        (
            "isize_bit_set",
            Only(&["mov", "lsl", "ldaxr", "orr", "stlxr", "cbnz", "tst", "cset", "ret"]),
        ),
        ("u8_bit_set", InOrder(&["ldaxrb", "orr", "stlxrb"])),
        ("tag_set", Only(&["ldxr", "orr", "stlxr", "cbnz", "ret"])),
        ("tag_clear", Only(&["ldxr", "and", "stlxr", "cbnz", "ret"])),
        ("tag_toggle", Only(&["ldxr", "eor", "stlxr", "cbnz", "ret"])),
        ("bit0_test_and_set", Only(&["ldaxr", "orr", "stlxr", "cbnz", "and", "ret"])),
        ("bit0_test_and_set_in_some", Only(&["ldaxr", "orr", "stlxr", "cbnz", "and", "ret"])),
        ("bit0_test_and_clear", Only(&["ldaxr", "and", "stlxr", "cbnz", "and", "ret"])),
        ("bit0_test_and_toggle", Only(&["ldaxr", "eor", "stlxr", "cbnz", "and", "ret"])),
        ("bit1_test_and_set", Only(&["ldaxr", "orr", "stlxr", "cbnz", "ubfx", "ret"])),
        ("bit1_test_and_clear", Only(&["ldaxr", "and", "stlxr", "cbnz", "ubfx", "ret"])),
        ("bit1_test_and_toggle", Only(&["ldaxr", "eor", "stlxr", "cbnz", "ubfx", "ret"])),
        ("bit2_test_and_set", Only(&["ldaxr", "orr", "stlxr", "cbnz", "ubfx", "ret"])),
        ("bit2_test_and_clear", Only(&["ldaxr", "and", "stlxr", "cbnz", "ubfx", "ret"])),
        ("bit2_test_and_toggle", Only(&["ldaxr", "eor", "stlxr", "cbnz", "ubfx", "ret"])),
        ("tag_flags_or", Only(&["ubfiz", "ldxr", "orr", "stlxr", "cbnz", "ret"])),
        ("tag_flags_and", Only(&["lsl", "orr", "ldxr", "and", "stlxr", "cbnz", "ret"])),
        ("tag_flags_xor", Only(&["ubfiz", "ldxr", "eor", "stlxr", "cbnz", "ret"])),
        ("tag_flags_not", Only(&["ldxr", "eor", "stlxr", "cbnz", "ret"])),
        ("nested_tag_set", Only(&["ldxr", "orr", "stlxr", "cbnz", "ret"])),
        ("tag_fetch_or", Only(&["ldaxr", "orr", "stlxr", "cbnz", "and", "and", "ret"])),
        (
            "word_compare_exchange",
            Only(&[
                "stp", "mov", "ldr", "ldr", "orr", "tst", "b.ne", "ldrb", "ldrb", "ldrb", "ldrb",
                "orr", "orr", "add", "add", "ldaxr", "cmp", "b.ne", "stlxr", "cbnz", "mov", "ldp",
                "ret", "adrp", "add", "bl", "mov", "clrex", "ldp", "ret",
            ]),
        ),
        (
            "nested_compare_exchange",
            Only(&[
                "stp", "mov", "ldr", "ldr", "orr", "tst", "b.ne", "ldrb", "ldrb", "ldrb", "ldrb",
                "orr", "orr", "add", "add", "ldaxr", "cmp", "b.ne", "stlxr", "cbnz", "mov", "ldp",
                "ret", "adrp", "add", "bl", "mov", "clrex", "ldp", "ret",
            ]),
        ),
        ("u128_update", Retry(&["ldaxp", "cmp", "stxp", "stxp", "ldaxp", "cmp", "stlxp", "stlxp"])),
        ("pair_update", Retry(&["ldaxp", "cmp", "stxp", "stxp", "ldaxp", "cmp", "stlxp", "stlxp"])),
        ("quantity_update", Retry(&["ldar", "ldaxr", "cmp", "stlxr"])),
        ("tag_update", Retry(&["ldar", "ldaxr", "and", "stlxr"])),
        (
            "word_update",
            RetryOnly(&[
                "ldar", "ldaxr", "cmp", "b.ne", "and", "and", "eor", "orr", "stlxr", "cbnz", "mov",
                "mov", "cbz", "b", "mov", "clrex", "b", "mov", "b", "and", "and", "ubfx", "str",
                "strb", "strb", "ret",
            ]),
        ),
        (
            "word_pop",
            RetryOnly(&[
                "stp", "mov", "ldar", "mov", "ands", "b.eq", "ldr", "tst", "b.ne", "and", "ldaxr",
                "add", "cmp", "b.ne", "add", "and", "and", "add", "add", "stlxr", "cbnz", "mov",
                "cbnz", "ands", "mov", "b.ne", "b", "mov", "clrex", "b", "mov", "b", "ldp", "ret",
                "adrp", "add", "bl",
            ]),
        ),
    ];

    /// Each function of the fixture on `x86_64` with `cmpxchg16b`: at the `x86-64-v3` floor on
    /// Linux and macOS, and at Windows' default CPU.
    ///
    /// A field's bit test is one `lock bts`, `btr` or `btc` at every position: with the position an
    /// immediate, but the lowest bit's and the top bit's, which go through a register, and with
    /// LLVM's `setb`, shift and test where the bit is the upper half's lowest, at 8 of 16 bits or
    /// 32 of 64. An atomic's bit goes through a register at every position, after an `and`.
    const X86_64: &[(&str, Lowering)] = &[
        ("u64_load", Only(&["movq", "retq"])),
        ("u64_store", Only(&["movq", "retq"])),
        ("u64_swap", InOrder(&["xchgq"])),
        ("u64_fetch_add", InOrder(&["lock xaddq"])),
        ("u64_fetch_sub", InOrder(&["negq", "lock xaddq"])),
        ("u64_compare_exchange", InOrder(&["lock cmpxchgq"])),
        ("u64_fetch_add_discarded", InOrder(&["lock addq"])),
        ("u64_fetch_sub_discarded", InOrder(&["lock subq"])),
        ("u64_or", InOrder(&["lock orq"])),
        ("u64_and", InOrder(&["lock andq"])),
        ("u64_xor", InOrder(&["lock xorq"])),
        ("u64_not", InOrder(&["lock xorq"])),
        ("bool_or", InOrder(&["lock orb"])),
        ("ptr_fetch_byte_add_discarded", InOrder(&["lock addq"])),
        ("ptr_fetch_ptr_sub", InOrder(&["negq", "lock xaddq"])),
        ("char_load", Only(&["movl", "retq"])),
        ("option_load", Only(&["movq", "retq"])),
        ("ranged_load", Only(&["movq", "retq"])),
        ("u128_compare_exchange", InOrder(&["lock cmpxchg16b"])),
        ("u128_load_rmw", InOrder(&["lock cmpxchg16b"])),
        ("store_store_fence", Only(&["movq", "movq", "retq"])),
        ("seq_cst_compiler_fence", Only(&["retq"])),
        ("acquire_fence", Only(&["retq"])),
        ("release_fence", Only(&["retq"])),
        ("seq_cst_fence", Only(&["lock orl", "retq"])),
        ("ranged_below_min", Only(&["xorl", "retq"])),
        ("ranged_to_deranged", Only(&["movl", "retq"])),
        ("ranged_from_deranged", Only(&["movl", "retq"])),
        ("field_set", Only(&["movabsq", "lock orq", "retq"])),
        ("field_clear", Only(&["movabsq", "lock andq", "retq"])),
        ("field_toggle", Only(&["movabsq", "lock xorq", "retq"])),
        ("field_test_and_set", Only(&["lock btsq", "setb", "retq"])),
        ("field_test_and_set_in_some", Only(&["lock btsq", "setb", "retq"])),
        ("field_test_and_set_through_map", InOrder(&["lock btsq", "setb"])),
        ("field_load", Only(&["movq", "shrq", "andl", "retq"])),
        ("flags_load", Only(&["movq", "shrq", "retq"])),
        ("quantity_load", Only(&["movq", "retq"])),
        ("quantity_update", Retry(&["lock cmpxchgq"])),
        ("flags_or", Only(&["movzbl", "shlq", "lock orq", "retq"])),
        ("flags_xor", Only(&["movzbl", "shlq", "lock xorq", "retq"])),
        ("flags_not", Only(&["movabsq", "lock xorq", "retq"])),
        ("flags_or_constant", Only(&["movabsq", "lock orq", "retq"])),
        ("nested_clear", Only(&["lock andl", "retq"])),
        ("nested_flags_or", Only(&["movzbl", "shll", "lock orl", "retq"])),
        ("top_fetch_add", InOrder(&["shlq", "lock xaddq"])),
        ("top_fetch_sub", InOrder(&["shlq", "negq", "lock xaddq"])),
        ("top_fetch_add_discarded", Only(&["shlq", "lock addq", "retq"])),
        ("top_fetch_add_count", Only(&["movabsq", "lock xaddq", "shrq", "retq"])),
        ("u61_top_fetch_add_references", Only(&["movl", "lock xaddq", "shrq", "retq"])),
        ("ends16_low_test_and_set", Only(&["lock btsw", "setb", "retq"])),
        ("ends16_middle_test_and_set", Only(&["lock btsw", "setb", "retq"])),
        ("ends16_top_test_and_set", Only(&["lock btsw", "setb", "retq"])),
        ("ends32_low_test_and_set", Only(&["lock btsl", "setb", "retq"])),
        ("ends32_middle_test_and_set", Only(&["lock btsl", "setb", "retq"])),
        ("ends32_top_test_and_set", Only(&["lock btsl", "setb", "retq"])),
        ("ends64_low_test_and_set", Only(&["lock btsq", "setb", "retq"])),
        ("ends64_middle_test_and_set", Only(&["lock btsq", "setb", "retq"])),
        ("ends64_middle_test_and_clear", Only(&["lock btrq", "setb", "retq"])),
        ("ends64_middle_test_and_toggle", Only(&["lock btcq", "setb", "retq"])),
        ("ends64_top_test_and_set", Only(&["lock btsq", "setb", "retq"])),
        ("ends64_top_test_and_clear", Only(&["lock btrq", "setb", "retq"])),
        ("ends64_top_test_and_toggle", Only(&["lock btcq", "setb", "retq"])),
        ("ends64_low_test_and_clear_in_some", Only(&["lock btrq", "setb", "retq"])),
        ("ends64_top_test_and_toggle_in_some", Only(&["lock btcq", "setb", "retq"])),
        ("u64_bit_set", Only(&["andl", "lock btsq", "setb", "retq"])),
        ("u64_bit_clear", Only(&["andl", "lock btrq", "setb", "retq"])),
        ("u64_bit_toggle", Only(&["andl", "lock btcq", "setb", "retq"])),
        ("u64_bit_set_0", Only(&["xorl", "lock btsq", "setb", "retq"])),
        ("u64_bit_set_5", Only(&["movl", "lock btsq", "setb", "retq"])),
        ("u64_bit_set_63", Only(&["movl", "lock btsq", "setb", "retq"])),
        ("u64_bit_clear_63", Only(&["movl", "lock btrq", "setb", "retq"])),
        ("u64_bit_set_64", Only(&["xorl", "lock btsq", "setb", "retq"])),
        ("u64_bit_set_discarded", Only(&["andl", "lock btsq", "setb", "retq"])),
        ("u64_bit_set_in_some", Only(&["andl", "lock btsq", "setb", "retq"])),
        ("u64_bit_clear_in_some", Only(&["andl", "lock btrq", "setb", "retq"])),
        ("u64_bit_toggle_in_some", Only(&["andl", "lock btcq", "setb", "retq"])),
        ("u64_bit_set_63_in_some", Only(&["movl", "lock btsq", "setb", "retq"])),
        ("u64_bit_set_through_map", InOrder(&["andl", "lock btsq", "setb"])),
        ("u32_bit_set", Only(&["andl", "lock btsl", "setb", "retq"])),
        ("u32_bit_set_31", Only(&["movl", "lock btsl", "setb", "retq"])),
        ("u16_bit_set", Only(&["andl", "lock btsw", "setb", "retq"])),
        ("u16_bit_set_in_some", Only(&["andl", "lock btsw", "setb", "retq"])),
        ("i32_bit_set", Only(&["andl", "lock btsl", "setb", "retq"])),
        ("i64_bit_set", Only(&["andl", "lock btsq", "setb", "retq"])),
        ("isize_bit_set", Only(&["andl", "lock btsq", "setb", "retq"])),
        (
            "word_load",
            Only(&[
                "movq", "movq", "movq", "andq", "movl", "andb", "shrb", "andb", "movq", "movb",
                "movb", "retq",
            ]),
        ),
        ("word_load_top", Only(&["movq", "andq", "retq"])),
        (
            "option_word_load",
            Only(&["movq", "movq", "andq", "movl", "andl", "testq", "movl", "cmovnel", "retq"]),
        ),
        ("tag_set", Only(&["lock orq", "retq"])),
        ("tag_clear", Only(&["lock andq", "retq"])),
        ("tag_toggle", Only(&["lock xorq", "retq"])),
        ("tag_load", Only(&["movq", "andb", "retq"])),
        ("tag_update", Retry(&["lock cmpxchgq"])),
        ("bit0_test_and_set", Only(&["lock btsq", "setb", "retq"])),
        ("bit0_test_and_set_in_some", Only(&["lock btsq", "setb", "retq"])),
        ("bit0_test_and_clear", Only(&["lock btrq", "setb", "retq"])),
        ("bit0_test_and_toggle", Only(&["lock btcq", "setb", "retq"])),
        ("bit1_test_and_set", Only(&["lock btsq", "setb", "retq"])),
        ("bit1_test_and_clear", Only(&["lock btrq", "setb", "retq"])),
        ("bit1_test_and_toggle", Only(&["lock btcq", "setb", "retq"])),
        ("bit2_test_and_set", Only(&["lock btsq", "setb", "retq"])),
        ("bit2_test_and_clear", Only(&["lock btrq", "setb", "retq"])),
        ("bit2_test_and_toggle", Only(&["lock btcq", "setb", "retq"])),
        ("tag_flags_not", Only(&["lock xorq", "retq"])),
        ("nested_tag_set", Only(&["lock orq", "retq"])),
        ("pointer_place_load", Only(&["movq", "movl", "andl", "andq", "retq"])),
        (
            "word_update",
            RetryOnly(&[
                "movq",
                "movl",
                "movq",
                "andq",
                "andl",
                "xorq",
                "orq",
                "lock cmpxchgq",
                "jne",
                "movq",
                "andq",
                "movl",
                "andb",
                "shrb",
                "andb",
                "movq",
                "movb",
                "movb",
                "movq",
                "retq",
            ]),
        ),
    ];

    /// The rest on `x86_64` with AVX, at the `x86-64-v3` floor: a 128-bit load or store is
    /// `vmovdqa`, and an update's first read too.
    const X86_64_AVX: &[(&str, Lowering)] = &[
        ("u128_load", InOrder(&["vmovdqa"])),
        ("u128_load_relaxed", InOrder(&["vmovdqa"])),
        ("u128_load_seq_cst", InOrder(&["vmovdqa"])),
        ("u128_store", InOrder(&["vmovdqa"])),
        ("u128_store_seq_cst", InOrder(&["vmovdqa", "lock orl"])),
        ("u128_update", Retry(&["vmovdqa", "lock cmpxchg16b"])),
        ("pair_load", Only(&["vmovdqa", "vmovq", "vpextrq", "retq"])),
        ("pair_store", Only(&["vmovq", "vmovq", "vpunpcklqdq", "vmovdqa", "retq"])),
        ("pair_update", Retry(&["vmovdqa", "lock cmpxchg16b"])),
    ];

    /// The rest on `x86_64` Linux and macOS: the System V calling convention passes the arguments
    /// these move, and a tagged pointer's store or exchange reserves no stack on its fast path.
    const X86_64_SYSTEM_V: &[(&str, Lowering)] = &[
        (
            "pair_load_rmw",
            Only(&["pushq", "xorl", "xorl", "xorl", "xorl", "lock cmpxchg16b", "popq", "retq"]),
        ),
        (
            "pair_compare_exchange",
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
        (
            "slice_compare_exchange",
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
        ("field_store", InOrder(&["testl", "lock orq", "lock andq"])),
        ("flags_and", Only(&["shlq", "movabsq", "orq", "lock andq", "retq"])),
        ("u4_flags_or", Only(&["andl", "lock orw", "retq"])),
        ("tag_flags_or", Only(&["addl", "andl", "lock orq", "retq"])),
        ("tag_flags_and", Only(&["addl", "orq", "lock andq", "retq"])),
        ("tag_flags_xor", Only(&["addl", "andl", "lock xorq", "retq"])),
        ("word_store", Refuses(&["testb", "jne", "incq", "movq", "retq"])),
        ("nested_store", Refuses(&["testb", "jne", "addq", "movq", "retq"])),
        ("word_compare_exchange", Refuses(&["orl", "testb", "jne", "lock cmpxchgq", "retq"])),
        ("nested_compare_exchange", Refuses(&["orl", "testb", "jne", "lock cmpxchgq", "retq"])),
    ];

    /// The rest on `x86_64` Linux, where a function that calls aligns the stack with a push.
    const X86_64_LINUX_PUSH: &[(&str, Lowering)] = &[(
        "word_pop",
        RetryOnly(&[
            "pushq",
            "movq",
            "movq",
            "andq",
            "je",
            "movq",
            "testb",
            "jne",
            "movl",
            "andl",
            "leal",
            "andl",
            "addq",
            "addq",
            "andl",
            "addq",
            "lock cmpxchgq",
            "jne",
            "movq",
            "popq",
            "retq",
            "xorl",
            "movq",
            "popq",
            "retq",
            "leaq",
            "callq",
        ]),
    )];

    /// The rest on `x86_64` macOS, where every function's frame record aligns the stack.
    const X86_64_MACOS_FRAME: &[(&str, Lowering)] = &[(
        "word_pop",
        RetryOnly(&[
            "movq",
            "movq",
            "andq",
            "je",
            "movq",
            "testb",
            "jne",
            "movl",
            "andl",
            "leal",
            "andl",
            "addq",
            "addq",
            "andl",
            "addq",
            "lock cmpxchgq",
            "jne",
            "movq",
            "retq",
            "xorl",
            "movq",
            "retq",
            "leaq",
            "callq",
        ]),
    )];

    /// The rest on `x86_64` Windows at its default CPU, which has `cmpxchg16b` but no AVX: a
    /// 128-bit atomic has no load or store, and its update reads with a compare-exchange. A
    /// function that calls the refusal reserves its stack on the fast path, a `subq` and an `addq`.
    const X86_64_WINDOWS_DEFAULT: &[(&str, Lowering)] = &[
        ("u128_update", Retry(&["lock cmpxchg16b", "lock cmpxchg16b"])),
        ("pair_update", Retry(&["lock cmpxchg16b", "lock cmpxchg16b"])),
        (
            "pair_load_rmw",
            Only(&[
                "pushq",
                "movq",
                "xorl",
                "xorl",
                "xorl",
                "xorl",
                "lock cmpxchg16b",
                "popq",
                "retq",
            ]),
        ),
        (
            "pair_compare_exchange",
            Only(&[
                "pushq",
                "movq",
                "movq",
                "movq",
                "movq",
                "movq",
                "xorl",
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
        (
            "slice_compare_exchange",
            Only(&[
                "pushq",
                "movq",
                "movq",
                "movq",
                "movq",
                "movq",
                "xorl",
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
        ("field_store", InOrder(&["testb", "lock orq", "lock andq"])),
        ("flags_and", Only(&["movzbl", "shlq", "movabsq", "orq", "lock andq", "retq"])),
        ("u4_flags_or", Only(&["andb", "movzbl", "lock orw", "retq"])),
        ("tag_flags_or", Only(&["movzbl", "addl", "andl", "lock orq", "retq"])),
        ("tag_flags_and", Only(&["movzbl", "addl", "orq", "lock andq", "retq"])),
        ("tag_flags_xor", Only(&["movzbl", "addl", "andl", "lock xorq", "retq"])),
        ("word_store", Refuses(&["subq", "testb", "jne", "incq", "movq", "addq", "retq"])),
        ("nested_store", Refuses(&["subq", "testb", "jne", "addq", "movq", "addq", "retq"])),
        (
            "word_compare_exchange",
            Refuses(&["subq", "orl", "testb", "jne", "lock cmpxchgq", "addq", "retq"]),
        ),
        (
            "nested_compare_exchange",
            Refuses(&["subq", "orl", "testb", "jne", "lock cmpxchgq", "addq", "retq"]),
        ),
        (
            "word_pop",
            RetryOnly(&[
                "subq",
                "movq",
                "movq",
                "andq",
                "je",
                "movq",
                "testb",
                "jne",
                "movl",
                "andl",
                "leal",
                "andl",
                "addq",
                "addq",
                "andl",
                "addq",
                "lock cmpxchgq",
                "jne",
                "movq",
                "addq",
                "retq",
                "xorl",
                "movq",
                "addq",
                "retq",
                "leaq",
                "callq",
                "ud2",
            ]),
        ),
    ];

    #[test]
    fn aarch64_linux_lowers_each_operation_to_its_instruction() {
        let tables = [AARCH64, AARCH64_LSE, AARCH64_WITHOUT_LSE2, AARCH64_FLOOR];
        lowers_as_expected(AARCH64_LINUX, None, &tables);
    }

    #[test]
    fn aarch64_linux_with_lse2_lowers_each_operation_to_its_instruction() {
        let tables = [AARCH64, AARCH64_LSE, AARCH64_LSE2, AARCH64_LINUX_LSE2];
        lowers_as_expected(AARCH64_LINUX, Some("neoverse-v1"), &tables);
    }

    #[test]
    fn aarch64_macos_lowers_each_operation_to_its_instruction() {
        let tables = [AARCH64, AARCH64_LSE, AARCH64_LSE2, AARCH64_MACOS_LSE2];
        lowers_as_expected(AARCH64_MACOS, None, &tables);
    }

    // `generic`, the target's default CPU, replaces the floor only if `RUSTFLAGS` names it.
    #[test]
    fn aarch64_android_at_its_default_cpu_lowers_each_operation_to_its_instruction_or_ll_sc_loop() {
        let tables = [AARCH64, AARCH64_WITHOUT_LSE2, AARCH64_WITHOUT_LSE];
        lowers_as_expected(AARCH64_ANDROID, Some("generic"), &tables);
    }

    #[test]
    fn x86_64_linux_lowers_each_operation_to_its_instruction() {
        let tables = [X86_64, X86_64_AVX, X86_64_SYSTEM_V, X86_64_LINUX_PUSH];
        lowers_as_expected(X86_64_LINUX, None, &tables);
    }

    #[test]
    fn x86_64_macos_lowers_each_operation_to_its_instruction() {
        let tables = [X86_64, X86_64_AVX, X86_64_SYSTEM_V, X86_64_MACOS_FRAME];
        lowers_as_expected(X86_64_MACOS, None, &tables);
    }

    // `x86-64` is the target's default CPU, to which its spec adds `cmpxchg16b`.
    #[test]
    fn x86_64_windows_at_its_default_cpu_lowers_each_operation_to_its_instruction() {
        lowers_as_expected(X86_64_WINDOWS, Some("x86-64"), &[X86_64, X86_64_WINDOWS_DEFAULT]);
    }

    /// Checks that `target`, an `x86_64` one, refuses each probe only `aarch64` has, each with its
    /// capability's message.
    fn refuses_each_operation_only_aarch64_has(target: &str) {
        let stderr = refused(target, None, "aarch64-only");
        let errors: Vec<&str> = stderr.lines().filter(|line| line.starts_with("error[")).collect();
        let fetch_bitwise = "has no `fetch_and`, `fetch_or`, `fetch_xor` or `fetch_not` without a \
                             compare-exchange loop on this target";
        let min_max =
            "has no atomic maximum or minimum without a compare-exchange loop on this target";
        let bit_test = "has no bit test-and-set without a compare-exchange loop on this target";
        assert!(
            errors.iter().all(|error| error.starts_with("error[E0277]: `")
                && [fetch_bitwise, min_max, bit_test]
                    .iter()
                    .any(|message| error.ends_with(message))),
            "every error is `FetchBitwise`'s, `MinMax`'s or `BitTest`'s:\n{stderr}"
        );
        let x86_64 = [X86_64, X86_64_AVX, X86_64_SYSTEM_V, X86_64_LINUX_PUSH];
        let aarch64_only = [AARCH64, AARCH64_LSE, AARCH64_WITHOUT_LSE2, AARCH64_FLOOR]
            .into_iter()
            .flatten()
            .filter(|(name, _)| x86_64.into_iter().flatten().all(|(other, _)| other != name))
            .count();
        assert_eq!(errors.len(), aarch64_only, "one error per probe only aarch64 has:\n{stderr}");
        for line in [
            "= help: the trait `FetchBitwise` is not implemented for `u64`",
            "= note: x86_64's `lock and`, `lock or` and `lock xor` cannot return the value before",
            "= help: the trait `MinMax` is not implemented for `i64`",
            "= note: x86_64 has no atomic maximum or minimum",
            "= help: the trait `BitTest` is not implemented for `u8`",
            "= note: x86_64's `lock bts`, `btr` and `btc` take 16, 32 or 64 bits",
            "= note: for an 8-bit word, use a 16-bit one, `AtomicU16` or `#[atom(repr = u16)]`",
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

    /// The errors of the `aarch64-refused` probes where a 16-byte atomic has a compare-exchange
    /// alone: one per probe, a double word's `Load`, `Store` and `Swap`, and a `u128`'s and its
    /// `MinMax`.
    const WIDE_REFUSALS: [&str; 7] = [
        "error[E0277]: `DoubleWord<*mut double_words::Node, *mut double_words::Node>` has no \
         pure-read atomic load on this target",
        "error[E0277]: `DoubleWord<*mut double_words::Node, *mut double_words::Node>` has no \
         atomic store without a compare-exchange loop on this target",
        "error[E0277]: `DoubleWord<*mut double_words::Node, *mut double_words::Node>` has no \
         atomic exchange without a compare-exchange loop on this target",
        "error[E0277]: `u128` has no pure-read atomic load on this target",
        "error[E0277]: `u128` has no atomic store without a compare-exchange loop on this target",
        "error[E0277]: `u128` has no atomic exchange without a compare-exchange loop on this target",
        "error[E0277]: `u128` has no atomic maximum or minimum without a compare-exchange loop on \
         this target",
    ];

    /// Checks that `target`, built for `cpu` or the floor, refuses each `aarch64-refused` probe
    /// with its error, and that the diagnostics say each of `notes`.
    fn refuses_each_wide_capability(target: &str, cpu: Option<&str>, notes: &[&str]) {
        let stderr = refused(target, cpu, "aarch64-refused");
        let errors: Vec<&str> = stderr.lines().filter(|line| line.starts_with("error[")).collect();
        assert_eq!(errors, WIDE_REFUSALS, "one error per probe:\n{stderr}");
        for line in notes.iter().chain(&[
            "= note: to accept a load that writes the cache line, call `load_rmw`",
            "= note: to accept a compare-exchange loop, call `store_rmw`",
            "= note: atomix has no 128-bit exchange: call `update` with `|_| new`",
            "= note: to accept a compare-exchange loop, call `update`",
        ]) {
            assert!(stderr.contains(line), "the diagnostics say `{line}`:\n{stderr}");
        }
    }

    #[test]
    fn aarch64_linux_refuses_each_wide_capability() {
        refuses_each_wide_capability(
            AARCH64_LINUX,
            None,
            &[
                "= note: a 128-bit load is one instruction with FEAT_LSE2",
                "= note: a 128-bit store is one instruction with FEAT_LSE2",
                "= note: aarch64's atomic maximum and minimum take an integer of at most 64 bits",
            ],
        );
    }

    #[test]
    fn x86_64_windows_at_its_default_cpu_refuses_each_wide_capability() {
        refuses_each_wide_capability(
            X86_64_WINDOWS,
            Some("x86-64"),
            &[
                "or AVX (x86_64: `-C target-cpu=x86-64-v3`)",
                "= note: x86_64 has no atomic maximum or minimum",
            ],
        );
    }

    #[test]
    fn x86_64_without_cmpxchg16b_refuses_two_words_as_it_refuses_128_bits() {
        let stderr = refused(X86_64_LINUX, Some("x86-64"), "x86-64-refused");
        let refusals: Vec<&str> =
            stderr.lines().filter(|line| line.starts_with("error[E0277]")).collect();
        assert_eq!(
            refusals,
            [
                "error[E0277]: `(NonNull<without_cmpxchg16b::Node>, \
                 NonNull<without_cmpxchg16b::Node>)` cannot be stored in an atomic",
                "error[E0277]: `NonNull<[u64]>` cannot be stored in an atomic",
            ],
            "the pair and the slice's pointer, each refused as no `Atom`:\n{stderr}"
        );
        let advice = "= note: a 128-bit value needs `cmpxchg16b` on x86_64: build with `-C \
                      target-cpu=x86-64-v2` or newer";
        assert_eq!(stderr.matches(advice).count(), 2, "each names the CPU it needs:\n{stderr}");
        assert!(!stderr.contains("PointerMetadata"), "and no hidden trait:\n{stderr}");
    }

    #[test]
    fn aarch64_macos_refuses_the_wide_exchange_and_maximum() {
        let stderr = refused(AARCH64_MACOS, None, "aarch64-refused");
        let errors: Vec<&str> = stderr.lines().filter(|line| line.starts_with("error[")).collect();
        assert_eq!(
            errors,
            [
                "error[E0277]: `DoubleWord<*mut double_words::Node, *mut double_words::Node>` has \
                 no atomic exchange without a compare-exchange loop on this target",
                "error[E0277]: `u128` has no atomic exchange without a compare-exchange loop on \
                 this target",
                "error[E0277]: `u128` has no atomic maximum or minimum without a compare-exchange \
                 loop on this target",
            ],
            "LSE2 has the load and store, so only the `Swap` and `MinMax` probes fail:\n{stderr}"
        );
    }
}
