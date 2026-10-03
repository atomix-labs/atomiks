//! What each operation lowers to, per target, read from the assembly of `tests/codegen`.
//!
//! The fixture builds with the repository's CPU floor and, on `aarch64`, with LSE2 too. Each
//! operation is the instructions and barriers its name promises, with no compare-exchange loop but
//! `update`'s; each target refuses each operation it lacks: `x86_64` those only `aarch64` has, and
//! `aarch64`'s floor the 128-bit ones it has no instruction for.

// Miri cannot run the compiler, and loom's atomics are not what ships.
#![cfg(on_hardware)]

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::path::Path;
    use std::process::{Command, Output};

    /// What a function of the fixture lowers to.
    enum Lowering {
        /// These instructions, in order, among others.
        InOrder(&'static [&'static str]),
        /// These instructions and no others.
        Only(&'static [&'static str]),
        /// As `InOrder`, plus one branch back to retry the compare-exchange: `update`'s loop.
        Retry(&'static [&'static str]),
    }

    use Lowering::{InOrder, Only, Retry};

    const AARCH64_LINUX: &str = "aarch64-unknown-linux-gnu";
    const X86_64_LINUX: &str = "x86_64-unknown-linux-gnu";

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
    ];

    /// The rest on `aarch64` with the `+lse` floor, which has no LSE2.
    ///
    /// With no 128-bit load, `update` reads with a compare-exchange. A `char` or an `Option` load,
    /// as every load, decodes without a check.
    const AARCH64_FLOOR: &[(&str, Lowering)] = &[
        ("u64_load", InOrder(&["ldar"])),
        ("char_load", Only(&["ldar", "ret"])),
        ("option_load", Only(&["ldar", "ret"])),
        ("u128_update", Retry(&["caspa", "caspal"])),
    ];

    /// The rest on `aarch64` with LSE2 (`neoverse-v1`).
    ///
    /// A 128-bit load or store is `ldp` or `stp` with the barriers its ordering needs, and an
    /// Acquire load is `ldapr`.
    const AARCH64_LSE2: &[(&str, Lowering)] = &[
        ("u64_load", InOrder(&["ldapr"])),
        ("char_load", Only(&["ldapr", "ret"])),
        ("option_load", Only(&["ldapr", "ret"])),
        ("u128_load", InOrder(&["ldp", "dmb ishld"])),
        ("u128_load_relaxed", InOrder(&["ldp"])),
        ("u128_load_seq_cst", InOrder(&["ldar", "ldp", "dmb ish"])),
        ("u128_store", InOrder(&["dmb ish", "stp"])),
        ("u128_store_seq_cst", InOrder(&["dmb ish", "stp", "dmb ish"])),
        ("u128_update", Retry(&["ldp", "dmb ishld", "caspal"])),
    ];

    /// Each function of the fixture on `x86_64` with the `x86-64-v3` floor (AVX).
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
    ];

    /// Cargo's `command` on the fixture for `target`, with the floor `.cargo/config.toml` sets.
    ///
    /// It runs from this crate, without the `RUSTFLAGS` that would replace the floor or the
    /// `CARGO_TARGET_<TRIPLE>_RUSTFLAGS` that would join it, and with its target directory and the
    /// fixture's lockfile in `out`.
    fn cargo(command: &str, target: &str, out: &Path) -> Command {
        let mut lockfile = OsString::from("resolver.lockfile-path='");
        lockfile.push(out.join("Cargo.lock"));
        lockfile.push("'");
        let mut cargo = Command::new(env!("CARGO"));
        cargo
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .env_remove("CARGO_BUILD_RUSTFLAGS")
            .env_remove(format!(
                "CARGO_TARGET_{}_RUSTFLAGS",
                target.to_uppercase().replace('-', "_")
            ))
            .args([command, "--target", target])
            .args(["--manifest-path", "tests/codegen/Cargo.toml", "--target-dir"])
            .arg(out)
            .arg("--config")
            .arg(lockfile);
        cargo
    }

    /// Runs `cargo` for `target`, and fails with the fix if the target is not installed.
    fn run(cargo: &mut Command, target: &str) -> Output {
        let output = cargo.output().expect("cargo is the one running this test");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("error[E0463]"),
            "target not installed: `rustup target add {target}`\n{stderr}"
        );
        output
    }

    /// The fixture's assembly for `target`, for the floor or, with `cpu`, for that CPU.
    ///
    /// Each build starts from an empty target directory: cargo keeps one build per configuration,
    /// but `--emit` writes each to the same file, and a deleted file is not rebuilt. Each probe
    /// keeps a body of its own: rustc would make one probe an alias of another with the same code.
    fn assembly(target: &str, cpu: Option<&str>) -> String {
        let name = cpu.map_or_else(|| target.to_owned(), |cpu| format!("{target}-{cpu}"));
        let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("codegen").join(name);
        if out.exists() {
            fs::remove_dir_all(&out).expect("the last run's build is removable");
        }
        let file = out.join("fixture.s");
        let mut emit = OsString::from("asm=");
        emit.push(&file);
        let mut build = cargo("rustc", target, &out);
        if let Some(cpu) = cpu {
            build
                .arg("--config")
                .arg(format!("target.{target}.rustflags=[\"-C\",\"target-cpu={cpu}\"]"));
        }
        build.args(["--release", "--", "-C", "codegen-units=1", "-Z", "merge-functions=disabled"]);
        let build = run(build.arg("--emit").arg(emit), target);
        assert!(
            build.status.success(),
            "the fixture builds for {target}:\n{}",
            String::from_utf8_lossy(&build.stderr)
        );
        fs::read_to_string(&file).expect("rustc wrote the assembly where `--emit` named")
    }

    /// One line of a function's assembly.
    enum Line {
        /// A branch target.
        Label(String),
        /// An instruction: its mnemonic, with a `lock` prefix or a barrier's domain kept, and where
        /// it branches to, if it is a branch.
        Instruction { mnemonic: String, branch: Option<String> },
    }

    /// The labels and instructions of the function `name` in `target`'s assembly.
    fn function(target: &str, assembly: &str, name: &str) -> Vec<Line> {
        // The assembler's comment: `#` in x86_64's syntax, `//` on aarch64, where `#` marks an
        // immediate.
        let comment = if target.starts_with("x86_64") { "#" } else { "//" };
        let mut lines = assembly
            .lines()
            .map(|line| line.split_once(comment).map_or(line, |(code, _)| code).trim());
        let start = format!("{name}:");
        assert!(lines.any(|line| line == start), "{target}: the assembly has `{name}`");
        let mut function = Vec::new();
        for line in lines.take_while(|line| !line.starts_with(".Lfunc_end")) {
            if let Some(label) = line.strip_suffix(':') {
                function.push(Line::Label(label.to_owned()));
                continue;
            }
            let mut tokens = line.split_whitespace();
            // An empty line, or a directive.
            let Some(head) = tokens.next().filter(|head| !head.starts_with('.')) else {
                continue;
            };
            let mnemonic = if ["lock", "dmb", "dsb"].contains(&head) {
                let Some(operand) = tokens.next() else {
                    panic!("{target}: `{name}` has a `{head}` alone on a line");
                };
                format!("{head} {operand}")
            } else {
                head.to_owned()
            };
            let is_branch = mnemonic.starts_with('j')
                || mnemonic.starts_with("b.")
                || ["b", "br", "cbz", "cbnz", "tbz", "tbnz"].contains(&mnemonic.as_str());
            let branch =
                line.rsplit([' ', '\t', ',']).next().filter(|_| is_branch).map(str::to_owned);
            function.push(Line::Instruction { mnemonic, branch });
        }
        function
    }

    /// Whether `mnemonic` is a compare-exchange: `cmpxchg` on `x86_64`, `cas` on `aarch64`.
    fn is_compare_exchange(mnemonic: &str) -> bool {
        mnemonic.contains("cmpxchg") || mnemonic.starts_with("cas")
    }

    /// Whether `mnemonic` is a barrier: `dmb`, `dsb` or `isb` on `aarch64`; on `x86_64`, a fence or
    /// a `lock`-prefixed instruction, which is how LLVM writes a full fence there.
    fn is_barrier(mnemonic: &str) -> bool {
        let head = mnemonic.split_once(' ').map_or(mnemonic, |(head, _)| head);
        ["dmb", "dsb", "isb", "mfence", "lfence", "sfence", "lock"].contains(&head)
    }

    /// Whether `mnemonic` is the load of a load-linked/store-conditional loop.
    fn is_load_linked(mnemonic: &str) -> bool {
        ["ldxr", "ldaxr", "ldxp", "ldaxp"].iter().any(|load| mnemonic.starts_with(load))
    }

    /// Checks that `name` costs nothing its `lowering` does not name.
    ///
    /// That is no call, no load-linked, no backward branch but, for a `Retry`, the one that
    /// retries its compare-exchange, and no compare-exchange or barrier beyond those the lowering
    /// names. A call or a jump out of the function counts, since what it reaches, such as an
    /// outline atomic, could loop.
    fn assert_no_unnamed_cost(target: &str, name: &str, lines: &[Line], lowering: &Lowering) {
        let (InOrder(wanted) | Only(wanted) | Retry(wanted)) = *lowering;
        let retries = matches!(lowering, Retry(_));
        let mut labels = Vec::new();
        let mut mnemonics = Vec::new();
        let mut branches_back = 0_usize;
        for line in lines {
            match line {
                Line::Label(label) => labels.push(label.as_str()),
                Line::Instruction { mnemonic, branch } => {
                    let leaves = branch.as_deref().is_some_and(|to| !to.starts_with(".L"));
                    assert!(
                        !leaves
                            && !mnemonic.starts_with("call")
                            && !["bl", "blr"].contains(&mnemonic.as_str()),
                        "{target}: `{name}` calls out: `{mnemonic}`"
                    );
                    assert!(
                        !is_load_linked(mnemonic),
                        "{target}: `{name}` has a load-linked loop: `{mnemonic}`"
                    );
                    let back = branch.as_deref().is_some_and(|to| labels.contains(&to));
                    assert!(retries || !back, "{target}: `{name}` branches back: `{mnemonic}`");
                    branches_back = branches_back.saturating_add(usize::from(back));
                    mnemonics.push(mnemonic.as_str());
                },
            }
        }
        assert!(
            !retries || branches_back == 1,
            "{target}: `{name}` branches back once, to retry its compare-exchange"
        );
        let count = |mnemonics: &[&str], is_kind: fn(&str) -> bool| {
            mnemonics.iter().filter(|mnemonic| is_kind(mnemonic)).count()
        };
        assert_eq!(
            count(&mnemonics, is_compare_exchange),
            count(wanted, is_compare_exchange),
            "{target}: `{name}` has only the compare-exchanges it names, among {mnemonics:?}"
        );
        assert_eq!(
            count(&mnemonics, is_barrier),
            count(wanted, is_barrier),
            "{target}: `{name}` has only the barriers it names, among {mnemonics:?}"
        );
    }

    /// Checks every function of `target`'s fixture, for the floor or `cpu`, against `expected`.
    ///
    /// The tables together list each function once.
    fn lowers_as_expected(target: &str, cpu: Option<&str>, expected: &[&[(&str, Lowering)]]) {
        let assembly = assembly(target, cpu);
        let mut found: Vec<&str> = assembly
            .lines()
            .filter_map(|line| line.trim().strip_prefix(".type")?.trim().strip_suffix(",@function"))
            .collect();
        found.sort_unstable();
        let mut named: Vec<&str> =
            expected.iter().copied().flatten().map(|(name, _)| *name).collect();
        named.sort_unstable();
        assert_eq!(found, named, "{target}: the fixture's functions are the ones listed");
        for (name, lowering) in expected.iter().copied().flatten() {
            let lines = function(target, &assembly, name);
            let mnemonics: Vec<&str> = lines
                .iter()
                .filter_map(|line| match line {
                    Line::Instruction { mnemonic, .. } => Some(mnemonic.as_str()),
                    Line::Label(_) => None,
                })
                .collect();
            match lowering {
                InOrder(wanted) | Retry(wanted) => {
                    let mut rest = mnemonics.iter();
                    assert!(
                        wanted.iter().all(|want| rest.any(|have| have == want)),
                        "{target}: `{name}` lowers to {wanted:?} in order, among {mnemonics:?}"
                    );
                },
                Only(wanted) => {
                    assert_eq!(&mnemonics, wanted, "{target}: `{name}` is these and nothing else");
                },
            }
            assert_no_unnamed_cost(target, name, &lines, lowering);
        }
    }

    /// The stderr of `cargo check` on the fixture for `target` with `feature`, which must fail.
    fn refused(target: &str, feature: &str) -> String {
        let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("codegen").join(feature);
        let check = run(cargo("check", target, &out).args(["--features", feature]), target);
        let stderr = String::from_utf8_lossy(&check.stderr).into_owned();
        assert!(!check.status.success(), "{target} refuses the `{feature}` probes:\n{stderr}");
        stderr
    }

    #[test]
    fn aarch64_lowers_each_operation_to_its_instruction() {
        lowers_as_expected(AARCH64_LINUX, None, &[AARCH64, AARCH64_FLOOR]);
    }

    #[test]
    fn aarch64_with_lse2_lowers_each_operation_to_its_instruction() {
        lowers_as_expected(AARCH64_LINUX, Some("neoverse-v1"), &[AARCH64, AARCH64_LSE2]);
    }

    #[test]
    fn x86_64_lowers_each_operation_to_its_instruction() {
        lowers_as_expected(X86_64_LINUX, None, &[X86_64]);
    }

    #[test]
    fn x86_64_refuses_each_operation_only_aarch64_has() {
        let stderr = refused(X86_64_LINUX, "aarch64-only");
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
    fn aarch64_floor_refuses_each_wide_capability() {
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
}
