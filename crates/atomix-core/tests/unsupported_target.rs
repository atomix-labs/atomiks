//! What a build for a target atomix does not support sees: one error, from the build script,
//! before rustc compiles a line, so the target needs no `core` installed; and a build for a
//! supported architecture on another OS does not.

// Miri cannot run cargo, and loom changes nothing the build script reads.
#![cfg(on_hardware)]

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::Command;

    /// The lines of the errors a check of atomix-core for `target` prints.
    fn errors_checking_for(target: &str) -> Vec<String> {
        let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("unsupported-target");
        let output = Command::new(env!("CARGO"))
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env_remove("RUSTFLAGS")
            .args(["check", "--offline", "--target", target, "--target-dir"])
            .arg(&out)
            .output()
            .expect("cargo is the one running this test");
        let stderr = String::from_utf8_lossy(&output.stderr);
        stderr.lines().filter(|line| line.starts_with("error")).map(str::to_owned).collect()
    }

    /// The build script's one error for `target`, and cargo's line after it.
    fn refusal(target: &str) -> [String; 2] {
        [
            format!(
                "error: atomix-core@{}: atomix builds for `aarch64` and `x86_64`, little-endian \
                 with 64-bit pointers, not for `{target}`: on another target, an operation it \
                 promises as one instruction could be a compare-exchange loop, as a 64-bit add is \
                 on 32-bit x86",
                env!("CARGO_PKG_VERSION")
            ),
            "error: build script logged errors".to_owned(),
        ]
    }

    #[test]
    fn a_32_bit_target_is_refused_with_one_error_naming_the_supported_ones() {
        let target = "i686-unknown-linux-gnu";
        assert_eq!(errors_checking_for(target), refusal(target), "32-bit x86");
    }

    #[test]
    fn a_target_of_a_supported_architecture_with_32_bit_pointers_is_refused() {
        let target = "x86_64-unknown-linux-gnux32";
        assert_eq!(errors_checking_for(target), refusal(target), "x32, whose pointers are 32 bits");
    }

    #[test]
    fn a_big_endian_target_of_a_supported_architecture_is_refused() {
        let target = "aarch64_be-unknown-linux-gnu";
        assert_eq!(errors_checking_for(target), refusal(target), "aarch64, big-endian");
    }

    #[test]
    fn a_64_bit_target_of_another_architecture_is_refused() {
        let target = "riscv64gc-unknown-linux-gnu";
        assert_eq!(errors_checking_for(target), refusal(target), "riscv64");
    }

    #[test]
    fn arm64ec_is_refused_though_it_runs_aarch64_code() {
        let target = "arm64ec-pc-windows-msvc";
        assert_eq!(errors_checking_for(target), refusal(target), "arm64ec, which is not aarch64");
    }

    #[test]
    fn a_target_of_a_supported_architecture_on_another_os_is_not_refused() {
        let target = "x86_64-unknown-freebsd";
        let errors = errors_checking_for(target);
        assert!(
            !errors.iter().any(|line| line.starts_with("error: atomix-core@")),
            "FreeBSD on x86_64, whose check fails, if at all, for want of its `core`: {errors:?}"
        );
    }
}
