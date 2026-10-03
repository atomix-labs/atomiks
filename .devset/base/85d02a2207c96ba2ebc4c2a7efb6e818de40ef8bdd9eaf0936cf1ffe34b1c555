---
name: review-rust
description: Use when a change to Rust code is ready, before calling it done or opening a pull request, or when asked to review Rust code, a diff, a branch or a pull request. Reviews the change against the workspace's Rust skills and reports each finding with its place, the rule it breaks and the fix.
argument-hint: "[<revision range or paths>] [-- <what the change is for>]"
context: fork
agent: Explore
model: inherit
background: false
---

# Review a Rust Change

This pass reviews a change to the workspace's Rust against its Rust guides, and
reports each finding with its line, the rule it breaks, why that matters in this
change, and the fix. It changes nothing: the agent that made the change knows
what it is for, and fixes each finding or says why one does not hold. The report
is all that agent sees, so it stands alone.

The guides are the rules. A finding is a line of the change that breaks one of
them, or that does not do what the change is for; a preference no guide holds,
or a fault a lint already reports, is none.

## Scope

The arguments, as given; an empty block is none:

```text
$ARGUMENTS
```

- **Before `--`**: a revision range, `main..HEAD` or `HEAD~2..`, or one commit,
  whose change is reviewed; or paths, whose change in the working tree and on
  the branch is reviewed, and a path with neither is reviewed whole, as it
  stands. A pull request is reviewed by its branch or its range, given here,
  since this pass cannot reach GitHub.
- **After `--`**: what the change is for, against which each finding's "why" is
  judged.
- **No range or paths**: the working tree against `HEAD`, and the branch against
  the default branch, both.
- **The default branch** is the first of `origin/HEAD`, `main` and `master` that
  `git log -1 --format=%H <name>` finds; with none, only the working tree is
  reviewed, and the report says so.
- **No purpose**: read it from the commits the change spans, `git log <range>`,
  and say so in the report. With no commits either, the purpose is unknown: the
  report says so, weighs nothing by it, and infers none from the change.

Find the change with git, each command run from the repository's root as it is
written here, since a fork cannot ask for an approval and a command written
another way, `git -C <dir> diff`, may need one; a shell loop or a `$VARIABLE` is
refused, so several files are read one command each:

- **The working tree**: `git diff -U20 HEAD`, staged or not, and `git status
  --short --untracked-files=all` for files git does not track yet, each read
  whole.
- **The branch**: `git diff -U20 <default>...HEAD`, from where it left the
  default branch.
- **A range**: `A..B` is the change from where `B` left `A`, so diff it as `git
  diff -U20 A...B`: `git diff A..B` compares the two tips, and shows what `A`
  gained since as undone. `git log A..B` lists its commits, and `git show -U20
  <commit>` shows one commit.
- **Paths**: `-- <paths>` after any of these narrows it.
- **A large change**: `git diff --stat` over the same range first, then `-U20`
  with `-- <file>`, a file at a time, wherever the whole diff would run long.

A range that ends anywhere but the working tree, as `HEAD~3..HEAD~1`, an older
commit or a branch not checked out, is read as it leaves each file, with `git
show <end>:<path>`, and each line number is taken from that.

Rust files, `build.rs` included, are in scope, and so is each `Cargo.toml`. Any
other file the change touches is named under Out of scope and not reviewed. With
neither in scope, the report is the line "No Rust in scope" and what was out of
scope, and the pass stops there, before the guides and the lints.

## Read the Guides

Read the body of each guide the workspace has, its SKILL.md, whole, before the
change: it states every rule in a sentence, and its References section says
which reference holds each rule's examples.

- `.claude/skills/writing-rust/SKILL.md`, `writing-rust`: errors, layout, names,
  API, ownership and lints.
- `.claude/skills/writing-unsafe-rust/SKILL.md`, `writing-unsafe-rust`: unsafe
  code, pointers, FFI and atomics.
- `.claude/skills/writing-rust-tests/SKILL.md`, `writing-rust-tests`: where
  tests go, their names and messages, fixtures, compile-fail tests and
  properties.
- `.claude/skills/tuning-rust-performance/SKILL.md`, `tuning-rust-performance`:
  measuring, allocation, code generation, the build profiles, data layout and
  threads.
- `.claude/skills/writing-rustdoc/SKILL.md`, `writing-rustdoc`: doc comments,
  `// SAFETY:`, `// INVARIANT:` and `// ORDERING:` comments, `#[expect]` reasons
  and assertion messages.
- `.claude/skills/editing-cargo-manifests/SKILL.md`, `editing-cargo-manifests`:
  every `Cargo.toml`.

## Lints First

Run `just check-rust-clippy` before reading the change. What it reports is the
lints' to hold: the report gives it one line, whether it passes and, where it
fails, in which crates and how many errors, and no finding restates a lint. A
change that does not build is said so in that line, and the rest is reviewed as
far as it reads. The lints run on the working tree: where the range ends
elsewhere, or the tree holds changes outside the range, the line says so.

## Read the Change

Read every changed hunk with its twenty lines of context, and further wherever
its meaning reaches past them: the whole function, type or `impl` it sits in;
the definition of what it calls, implements or changes; and, for a public item
whose signature or behaviour changes, its callers in the workspace. Go file by
file and keep count: the report says how many files and hunks were read. A file
too large to read whole is read hunk by hunk, never skipped.

## Read the References

Once the change is read, find the rules its hunks need in the guides'
references, each at `.claude/skills/<guide>/references/<name>.md` from the
repository's root, as the guide's References section names them: for an error
type, a panic or an `expect`, `writing-rust`'s `errors.md`; for a name a caller
sees, its `naming.md`; for an `unsafe` block, `writing-unsafe-rust`'s
`safety-comments.md`. List a reference's rules with `grep -n '^## '
.claude/skills/<guide>/references/<name>.md`, and read the section a hunk needs
with `Read` and an offset and a limit, from its heading to the next one, never
the whole file: a reference runs to a thousand lines, and the pass has only its
context. A finding cites the heading of the section it read.

## Check, in Order

Check every hunk for each category in turn, against the guide that holds it. A
hunk may break rules in several; each is its own finding.

1. **Errors**: `writing-rust`'s Errors rules and `errors.md`. A panic a caller
   can cause; a crate-wide or string error; a message out of its form; an error
   that renders its cause and exposes it; a `#[from]` whose lower error means
   more than one thing in the wrapper; a refused value not handed back; a
   `Result` alias; a command a person runs whose `main` returns the error; an
   error dropped without a name.
2. **API and naming**: `writing-rust`'s Layout, Names and API rules, with
   `layout.md`, `naming.md` and `api-design.md`. A public item added, removed,
   renamed or changed is a change its callers see: the finding says whether it
   breaks them. Code written by hand where a neater or more concise way exists,
   in std, a crate, a derive or a helper the workspace has, is a finding
   (`api-design.md`, "Search First, and Take the Most Concise Form That Measures
   as Fast"), unless a line beside it names the run that measured that way
   slower; the finding names the way.
3. **Unsafe and atomics**: `writing-unsafe-rust`, for every `unsafe`, raw
   pointer, `unsafe impl` and atomic the change adds or touches. Check that each
   `// SAFETY:` proves its preconditions from facts in scope, not only that it
   is there, and that each `// ORDERING:` names the operation it pairs with, and
   that operation exists; or, for `Relaxed`, says that nothing pairs, and that
   holds, since nothing else is read through it.
4. **Tests**:
   - `writing-rust-tests`: what the change adds or alters is tested where the
     guide puts tests, with names, messages and fixtures as it writes them; a
     test the change edits still pins what it pinned before, unless the change
     is for altering that.
   - for unsafe code or an atomic, `writing-unsafe-rust`'s Verifying rules: "a
     test for each edge a proof names", "a loom model for each atomic protocol"
     and "a compile-fail test for each misuse the types refuse".
5. **Performance and ownership**: what only review holds of `writing-rust`'s
   Ownership rules, in `ownership.md`:
   - a clone that only quiets the borrow checker, where a shorter scope, a kept
     reference or a move would do ("Clone Only What Must Be Owned Twice").
   - an owned value returned where the input usually comes back unchanged
     ("Return `Cow` When the Input Usually Comes Back Unchanged").
   - a small `Copy` value taken by reference in a function the crate exports,
     which the lints do not see ("Pass a Small `Copy` Value by Value").
   - `tuning-rust-performance`, where the change touches a hot path, a benchmark
     or a build profile, or is for speed.
6. **Docs**: `writing-rustdoc`, for each doc comment, `#[expect]` reason,
   assertion message, and `// SAFETY:`, `// INVARIANT:` or `// ORDERING:`
   comment the change adds, and each one it leaves untrue.
7. **Manifests**: `editing-cargo-manifests`, for each `Cargo.toml` the change
   touches.

A hunk that does not do what the change is for, or breaks what it touches, is a
finding in any category, and its rule is the change's purpose.

## The Report

The report is Markdown in this form, and nothing else:

````text
## Review of <scope>: <what the change is for>

Purpose read from the commits.
Read <n> files and <n> hunks.
Lints: `just check-rust-clippy` passes, or fails in <crates> with <n> errors.

| #   | Where        | Rule                       | Weight   |
| --- | ------------ | -------------------------- | -------- |
| 1   | `<path>:<n>` | `<skill>`, "<its heading>" | blocking |

### 1. `<path>:<n>`

<Why it matters here, in a sentence or two.>

```rust
<the fix>
```

Checked, nothing found: <each category with no finding>.

Not run: <each command refused or failed, and what it would have checked>.

Out of scope: <each file not reviewed, each rule the untouched code around the
change breaks, and anything not read, with why>.
````

- **The purpose line** says "Purpose read from the commits" where it was, or
  "Purpose unknown" where nothing gave one, and is left out where the arguments
  gave it.
- **Where** is the path from the repository's root and the line in the file as
  the change leaves it. A fault repeated on several lines is one finding that
  names each.
- **Rule** is the skill and the heading of the rule, as the guide writes it: a
  reference's heading where one holds the rule, else the body's.
- **Each finding's section**, under its number and place, says what goes wrong
  in this change if the line stays, in a sentence or two, without restating the
  rule, which the table names, or the fix, which the fence holds; then the fix
  in a fenced block, `rust`, `toml` or `text`, as the code should read, short
  enough to apply as written.
- **Weight** is by what the line costs if it stays, and where two weights fit,
  the heavier holds:
  - **blocking** where it is unsound, wrong for the change's purpose, or breaks
    a caller the change does not mean to;
  - **important** where it breaks a guide's rule in what a caller or the next
    change depends on: an error type or message, a public name or signature, a
    `// SAFETY:` or `// ORDERING:` that does not prove its claim, a doc the
    change leaves untrue, or a test a guide requires for what the change adds,
    as the render test of a new error or the round trip of a `Display` and
    `FromStr` pair;
  - a **suggestion** where the rules hold and a guide shows a better form; where
    a guide recommends a test or a tool beyond what it requires of this change,
    a property test, fuzzing or a benchmark, unless the change is for what that
    tool checks; and where the rule is broken only in code no caller outside its
    module sees, unless it is a `// SAFETY:`, `// ORDERING:` or `// INVARIANT:`
    that does not prove its claim.
- **Not run** names each command refused or failed, and is left out where every
  command ran.

Findings go blocking first, then important, then suggestions, each by path and
line. With no finding, the table is the line "Nothing found."

## Keeping It Honest

- **Read every changed hunk.** None is sampled or skimmed; one that could not be
  read is named under Out of scope, with why.
- **No finding without the line it stands on**, read in the file, never inferred
  from the diff's shape or a name.
- **No finding without a rule**: a heading of a guide read here, or the change's
  purpose.
- **No "consider" without a reason**: each finding says what goes wrong if it
  stays.
- **Nothing a lint holds**: a fault that rustc or clippy reports, or that a lint
  in `writing-rust`'s `lints.md` would, is the lint's. Where the lints did not
  run, a fault they would catch is a finding, and Not run says why.
- **No preference the guides do not hold**, in style, names or layout.
- **Only the change**: a rule that the code around it breaks goes under Out of
  scope, once, not in the table. In a path reviewed whole, every line is the
  change.
- **Say "nothing found" rather than invent one**: a category with no finding is
  a result.
- **Change nothing**: read, run the lints, and report.
