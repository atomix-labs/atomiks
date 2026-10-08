//! Refuses a target atomix does not build for, and names the conditions atomix-core's items
//! share, so each is written once.

use std::env;

use cfg_aliases::cfg_aliases;

/// Refuses a target atomix does not build for, whatever its OS: another architecture, 32-bit
/// pointers, or big-endian; on a supported one, declares each alias, with the `check-cfg` that lets
/// rustc know it.
fn main() {
    let cfg = |name: &str| env::var(format!("CARGO_CFG_TARGET_{name}")).unwrap_or_default();
    let (arch, pointer_width, endian) = (cfg("ARCH"), cfg("POINTER_WIDTH"), cfg("ENDIAN"));
    // A big-endian `aarch64_be` target's architecture is `aarch64` too.
    if !matches!(arch.as_str(), "aarch64" | "x86_64") || pointer_width != "64" || endian != "little"
    {
        // One line: cargo ends an instruction at its newline.
        println!(
            "cargo::error=atomix builds for `aarch64` and `x86_64`, little-endian with 64-bit \
             pointers, not for `{}`: on another target, an operation it promises as one \
             instruction could be a compare-exchange loop, as a 64-bit add is on 32-bit x86",
            env::var("TARGET").unwrap_or_default()
        );
        return;
    }
    cfg_aliases! {
        // A 16-byte compare-exchange: aarch64's, or x86_64's `cmpxchg16b`.
        wide: { any(target_arch = "aarch64", all(target_arch = "x86_64", target_feature = "cmpxchg16b")) },
        // A 16-byte load and store with no compare-exchange: LSE2's, or AVX's.
        wide_load_store: { all(wide, any(target_feature = "lse2", target_feature = "avx")) },
        // x86_64 with no `cmpxchg16b`, where a refusal of a 128-bit value names the CPU it needs.
        x86_64_without_cmpxchg16b: { all(target_arch = "x86_64", not(target_feature = "cmpxchg16b")) },
        // The 128-bit cell is core's `AtomicU128`, which loom does not model.
        core_atomic_u128: { all(target_arch = "aarch64", not(loom)) },
        // Neither loom's model nor Miri's interpreter: the atomics are the CPU's, and a test can
        // run cargo.
        on_hardware: { not(any(loom, miri)) },
        // A double word's cell is a lock around plain copies of its words, which keeps each pointer's
        // provenance: Miri's model, as loom's is the index table of the 16-byte cell it shares.
        double_word_lock: { all(miri, not(loom)) },
        // `fence(StoreStore)` is `dmb ishst`, an `asm!` that neither loom nor Miri runs.
        dmb_ishst: { all(target_arch = "aarch64", on_hardware) },
        // A bit's test-and-set, -clear and -toggle are `lock bts`, `btr` and `btc` in an `asm!`,
        // whose test LLVM cannot widen into a shift, which would leave a compare-exchange loop
        // where the bit lands in an `Option`; neither loom nor Miri runs `asm!`, and
        // ThreadSanitizer sees no access inside one.
        lock_bit_test: { all(target_arch = "x86_64", on_hardware, not(sanitize = "thread")) },
    }
}
