//! What a user crate sees: what compiles with no feature gate, and each misuse refused with its
//! message.

// Loom's cells exist only inside a model, and Miri cannot run the compiler.
#![cfg(on_hardware)]

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::{self, Path};

    #[test]
    fn user_crates_compile_or_are_refused_with_the_message() {
        // trybuild builds each fixture in `<target dir>/tests/trybuild`, and cargo finds
        // `.cargo/config.toml`, and its CPU floor, only when that is inside the workspace.
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("the crate is `crates/atomix-core`");
        if let Some(target_dir) = env::var_os("CARGO_TARGET_DIR") {
            let target_dir = path::absolute(target_dir).expect("the current directory exists");
            assert!(
                target_dir.starts_with(workspace),
                "CARGO_TARGET_DIR, {}, is outside the workspace, so trybuild's builds would miss \
                 `.cargo/config.toml`'s CPU floor: unset it, or point it inside {}",
                target_dir.display(),
                workspace.display()
            );
        }
        let cases = trybuild::TestCases::new();
        // With a pass case, trybuild builds every fixture rather than checking it, so the
        // post-monomorphization errors show.
        cases.pass("tests/compile_pass/*.rs");
        cases.compile_fail("tests/compile_fail/*.rs");
        if cfg!(feature = "arbitrary-int") {
            cases.compile_fail("tests/compile_fail/arbitrary_int/*.rs");
        }
        if cfg!(feature = "deranged-05") {
            cases.compile_fail("tests/compile_fail/deranged/*.rs");
        }
        if cfg!(feature = "zerocopy-08") {
            cases.compile_fail("tests/compile_fail/zerocopy/*.rs");
        }
        if cfg!(feature = "bytemuck") {
            cases.compile_fail("tests/compile_fail/bytemuck/*.rs");
        }
        // The refusals that LSE2's 16-byte load and store lift, which only macOS's floor has, run
        // where this test is built without them. trybuild removes `RUSTFLAGS`, so it builds these
        // with the floor while this test is built at the CPU `RUSTFLAGS` names: a leg that adds
        // LSE2 through it skips the batch, and one that took LSE2 from macOS's M1 would fail it. A
        // `CARGO_TARGET_<TRIPLE>_RUSTFLAGS` adds to the floor for both.
        if cfg!(all(aarch64_code, not(wide_load_store))) {
            cases.compile_fail("tests/compile_fail/aarch64_without_lse2/*.rs");
            if cfg!(feature = "serde") {
                cases.compile_fail("tests/compile_fail/aarch64_without_lse2/serde/*.rs");
            }
        }
    }
}
