//! How a crate's test reads its codegen fixture, `tests/codegen`: cargo builds it for a target,
//! and each function's assembly is checked against the instructions a table names for it, or a
//! build that must fail is read for its errors.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// What a function of the fixture lowers to.
pub(crate) enum Lowering {
    /// These instructions, in order, among others.
    InOrder(&'static [&'static str]),
    /// These instructions and no others.
    Only(&'static [&'static str]),
    /// As `InOrder`, plus branches back, each over a compare-exchange, which it retries:
    /// `update`'s loop.
    Retry(&'static [&'static str]),
    /// As `Only`, its branches back each over a compare-exchange, which it retries: a loop that
    /// tests, calls and saves nothing it does not name.
    RetryOnly(&'static [&'static str]),
    /// As `InOrder` up to the return, with no branch there but those named and nothing pushed or
    /// popped, `stp` and `ldp` included, then one cold block that calls [`REFUSAL`]: a tagged
    /// pointer's encode, whose fast path saves no frame record.
    Refuses(&'static [&'static str]),
}

use Lowering::{InOrder, Only, Refuses, Retry, RetryOnly};

/// The one function a lowering may call, by name: the cold refusal of a pointer misaligned for
/// its tags, which an encode of a tagged pointer calls off its fast path.
const REFUSAL: &str = "refuse_misaligned";

/// `aarch64` Linux, whose floor has LSE but not LSE2.
pub(crate) const AARCH64_LINUX: &str = "aarch64-unknown-linux-gnu";
/// `aarch64` macOS, whose floor, `apple-m1`, has LSE2.
pub(crate) const AARCH64_MACOS: &str = "aarch64-apple-darwin";
/// `x86_64` Linux, at the `x86-64-v3` floor.
pub(crate) const X86_64_LINUX: &str = "x86_64-unknown-linux-gnu";
/// `x86_64` macOS, at the same floor.
pub(crate) const X86_64_MACOS: &str = "x86_64-apple-darwin";

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
        .env_remove(format!("CARGO_TARGET_{}_RUSTFLAGS", target.to_uppercase().replace('-', "_")))
        .args([command, "--target", target])
        .args(["--manifest-path", "tests/codegen/Cargo.toml", "--target-dir"])
        .arg(out)
        .arg("--config")
        .arg(lockfile);
    cargo
}

/// The target directory of the fixture's build `name`, apart from every other crate's, whose tests
/// share `CARGO_TARGET_TMPDIR`.
fn target_directory(name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join("codegen").join(env!("CARGO_PKG_NAME")).join(name)
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
    let out = target_directory(&name);
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

/// How a target's assembly is written: Mach-O on macOS, ELF on Linux.
struct Syntax {
    /// What starts a comment: `#` on `x86_64`; on `aarch64`, where `#` marks an immediate, `;`
    /// in Mach-O and `//` in ELF.
    comment: &'static str,
    /// What each symbol's name starts with: `_` in Mach-O.
    symbol_prefix: &'static str,
    /// Whether each function keeps a frame record, as macOS's `x86_64` ABI asks: no cost of
    /// the operation, so not counted.
    frame_record: bool,
}

/// The syntax of `target`'s assembly.
fn syntax(target: &str) -> Syntax {
    let macos = target.ends_with("-apple-darwin");
    let x86_64 = target.starts_with("x86_64");
    Syntax {
        comment: if x86_64 {
            "#"
        } else if macos {
            ";"
        } else {
            "//"
        },
        symbol_prefix: if macos { "_" } else { "" },
        frame_record: macos && x86_64,
    }
}

/// One line of a function's assembly.
enum Line {
    /// A branch target.
    Label(String),
    /// An instruction: its mnemonic, with a `lock` prefix or a barrier's domain kept, and where
    /// it branches to, if it is a branch, or what it calls, if it is a call.
    Instruction { mnemonic: String, branch: Option<String>, callee: Option<String> },
}

/// The labels and instructions of the function `name` in `target`'s assembly.
///
/// A function ends at its `.cfi_endproc`, in ELF and Mach-O alike.
fn function(target: &str, assembly: &str, name: &str) -> Vec<Line> {
    let syntax = syntax(target);
    let mut lines = assembly
        .lines()
        .map(|line| line.split_once(syntax.comment).map_or(line, |(code, _)| code).trim());
    let start = format!("{}{name}:", syntax.symbol_prefix);
    assert!(lines.any(|line| line == start), "{target}: the assembly has `{name}`");
    let mut function = Vec::new();
    for line in lines.take_while(|line| *line != ".cfi_endproc") {
        let words: Vec<&str> = line.split_whitespace().collect();
        if syntax.frame_record
            && [["pushq", "%rbp"].as_slice(), &["movq", "%rsp,", "%rbp"], &["popq", "%rbp"]]
                .contains(&words.as_slice())
        {
            continue;
        }
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
        let operand = line.rsplit([' ', '\t', ',']).next().map(str::to_owned);
        let branch = operand.clone().filter(|_| is_branch(&mnemonic));
        let callee = operand.filter(|_| is_call(&mnemonic));
        function.push(Line::Instruction { mnemonic, branch, callee });
    }
    function
}

/// Whether `mnemonic` is a branch: a jump on `x86_64`, `b` and its kin on `aarch64`.
fn is_branch(mnemonic: &str) -> bool {
    mnemonic.starts_with('j')
        || mnemonic.starts_with("b.")
        || ["b", "br", "cbz", "cbnz", "tbz", "tbnz"].contains(&mnemonic)
}

/// Whether `mnemonic` is a call: `call` on `x86_64`, `bl` or `blr` on `aarch64`.
fn is_call(mnemonic: &str) -> bool {
    mnemonic.starts_with("call") || ["bl", "blr"].contains(&mnemonic)
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
/// That is no call but, for a `Refuses` or a lowering that names a call, the one to [`REFUSAL`], no
/// load-linked, no backward branch but, for a `Retry` or a `RetryOnly`, at least one, each from
/// past a compare-exchange to before it, and no compare-exchange or barrier beyond those the
/// lowering names. A call or a jump out of the function counts, since what it reaches, such as an
/// outline atomic, could loop.
fn assert_no_unnamed_cost(target: &str, name: &str, lines: &[Line], lowering: &Lowering) {
    let (InOrder(wanted) | Only(wanted) | Retry(wanted) | RetryOnly(wanted) | Refuses(wanted)) =
        *lowering;
    let retries = matches!(lowering, Retry(_) | RetryOnly(_));
    let refuses = matches!(lowering, Refuses(_)) || wanted.iter().any(|mnemonic| is_call(mnemonic));
    let mut refusals = 0_usize;
    let local: Vec<&str> = lines
        .iter()
        .filter_map(|line| match line {
            Line::Label(label) => Some(label.as_str()),
            Line::Instruction { .. } => None,
        })
        .collect();
    // Each label above the line, with how many compare-exchanges lie above it.
    let mut labels = Vec::new();
    let mut compare_exchanges = 0_usize;
    let mut mnemonics = Vec::new();
    let mut branches_back = 0_usize;
    for line in lines {
        match line {
            Line::Label(label) => labels.push((label.as_str(), compare_exchanges)),
            Line::Instruction { mnemonic, branch, callee } => {
                let leaves = branch.as_deref().is_some_and(|to| !local.contains(&to));
                let refusal = refuses && callee.as_deref().is_some_and(|to| to.contains(REFUSAL));
                refusals = refusals.saturating_add(usize::from(refusal));
                assert!(
                    !leaves && (!is_call(mnemonic) || refusal),
                    "{target}: `{name}` calls out: `{mnemonic}`"
                );
                assert!(
                    !is_load_linked(mnemonic),
                    "{target}: `{name}` has a load-linked loop: `{mnemonic}`"
                );
                // Where the branch is back, whether a compare-exchange lies between its label and
                // it.
                let back = branch.as_deref().and_then(|to| {
                    let (_, above) = labels.iter().find(|(label, _)| *label == to)?;
                    Some(compare_exchanges > *above)
                });
                assert!(
                    retries || back.is_none(),
                    "{target}: `{name}` branches back: `{mnemonic}`"
                );
                assert!(
                    back != Some(false),
                    "{target}: `{name}` branches back over no compare-exchange: `{mnemonic}`"
                );
                branches_back = branches_back.saturating_add(usize::from(back.is_some()));
                compare_exchanges =
                    compare_exchanges.saturating_add(usize::from(is_compare_exchange(mnemonic)));
                mnemonics.push(mnemonic.as_str());
            },
        }
    }
    assert!(
        !retries || branches_back > 0,
        "{target}: `{name}` branches back to retry its compare-exchange"
    );
    assert!(
        !refuses || refusals == 1,
        "{target}: `{name}` calls the refusal of a misaligned pointer once, off its fast path"
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
pub(crate) fn lowers_as_expected(
    target: &str, cpu: Option<&str>, expected: &[&[(&str, Lowering)]],
) {
    let assembly = assembly(target, cpu);
    let symbol_prefix = syntax(target).symbol_prefix;
    // Each refusal a probe calls is a symbol too, but no probe.
    let mut found: Vec<&str> = assembly
        .lines()
        .filter_map(|line| line.trim().strip_prefix(".globl")?.trim().strip_prefix(symbol_prefix))
        .filter(|symbol| !symbol.contains(REFUSAL))
        .collect();
    found.sort_unstable();
    let mut named: Vec<&str> = expected.iter().copied().flatten().map(|(name, _)| *name).collect();
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
            Only(wanted) | RetryOnly(wanted) => {
                assert_eq!(&mnemonics, wanted, "{target}: `{name}` is these and nothing else");
            },
            Refuses(wanted) => {
                let end = mnemonics.iter().position(|mnemonic| mnemonic.starts_with("ret"));
                let fast_path =
                    &mnemonics[..end.map_or(mnemonics.len(), |end| end.saturating_add(1))];
                let mut rest = fast_path.iter();
                assert!(
                    wanted.iter().all(|expected| rest.any(|mnemonic| mnemonic == expected)),
                    "{target}: `{name}`'s fast path is {wanted:?} in order, among {fast_path:?}"
                );
                let branches = |mnemonics: &[&str]| {
                    mnemonics.iter().filter(|mnemonic| is_branch(mnemonic)).count()
                };
                assert_eq!(
                    branches(fast_path),
                    branches(wanted),
                    "{target}: `{name}`'s fast path branches only as {wanted:?}, among {fast_path:?}"
                );
                assert!(
                    !fast_path.iter().any(|mnemonic| {
                        ["push", "pop", "stp", "ldp"]
                            .iter()
                            .any(|stack| mnemonic.starts_with(stack))
                    }),
                    "{target}: `{name}`'s fast path saves no frame record, among {fast_path:?}"
                );
            },
        }
        assert_no_unnamed_cost(target, name, &lines, lowering);
    }
}

/// The stderr of `cargo check` on the fixture for `target` with `feature`, which must fail.
pub(crate) fn refused(target: &str, feature: &str) -> String {
    let out = target_directory(&format!("{target}-{feature}"));
    let check = run(cargo("check", target, &out).args(["--features", feature]), target);
    let stderr = String::from_utf8_lossy(&check.stderr).into_owned();
    assert!(!check.status.success(), "{target} refuses the `{feature}` probes:\n{stderr}");
    stderr
}
