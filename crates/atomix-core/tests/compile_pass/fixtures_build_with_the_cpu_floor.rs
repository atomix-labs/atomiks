//! Every fixture builds with `.cargo/config.toml`'s CPU floor, which the 128-bit atomics and the
//! refusals that name the floor depend on; a target directory outside the workspace loses it.

#[cfg(not(any(target_feature = "lse", target_feature = "avx")))]
compile_error!(
    "built without `.cargo/config.toml`'s CPU floor: keep the target directory \
     (`CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR`, `build.target-dir`) inside the workspace"
);

fn main() {}
