//! Names the conditions atomiks-core's items share, so each is written once.

use cfg_aliases::cfg_aliases;

/// Declares each alias, with the `check-cfg` that lets rustc know it.
fn main() {
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
        // `fence(StoreStore)` is `dmb ishst`, an `asm!` that neither loom nor Miri runs.
        dmb_ishst: { all(target_arch = "aarch64", on_hardware) },
    }
}
