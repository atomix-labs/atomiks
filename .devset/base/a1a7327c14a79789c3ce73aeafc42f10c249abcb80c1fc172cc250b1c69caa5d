---
name: review-names
description: Use when a change adds, renames or changes what a public item does, a type, trait, function, method, field, variant, constant, module, feature or flag, before calling it done or opening a pull request, or when asked to review the names in a change, a branch or a path. Reviews each name against what its definition, uses and tests show it does, and reports each that misleads, is unclear, clashes with the codebase or breaks the language's conventions, with its place, the rule, the fix, and whether a rename breaks its callers.
argument-hint: "[<revision range or paths>] [-- <what the change is for>]"
context: fork
agent: Explore
model: inherit
background: false
---

# Review the Names in a Change

This pass reads the names a change adds or renames, or every name under a path,
and reports each one that misleads, is unclear, clashes with the codebase or
breaks the language's conventions, with its place, the rule, why it matters
here, and the fix. It changes nothing: the agent that made the change fixes each
finding or says why one does not hold, and the report is all that agent sees, so
it stands alone.

A name is judged by what the thing does, so each is read first: its definition,
its uses and its tests. The guides are the rules, and a name that keeps them and
the codebase's way is sound, however else it might have been put. The names a
caller outside the crate sees come first, since a caller reads the name and not
the body, and a rename breaks them.

## Scope

The arguments, as given; an empty block is none:

```text
$ARGUMENTS
```

- **Before `--`**: a revision range, `main..HEAD` or `HEAD~2..`, or one commit,
  whose added and renamed names are reviewed; or paths, whose every name is
  reviewed as the files stand, not only what a change there touches. A pull
  request is given by its branch or range, since this pass cannot reach GitHub.
- **After `--`**: what the change is for, and whether it means to break its
  callers.
- **No range or paths**: the working tree against `HEAD`, and the branch against
  the default branch, both.
- **The default branch** is the first of `origin/HEAD`, `main` and `master` that
  `git log -1 --format=%H <name>` finds; with none, only the working tree is
  reviewed, and the report says so.
- **No purpose**: read it from the commits the change spans, `git log <range>`;
  with none, the purpose is unknown.

Find the change with git, each command run from the repository's root as it is
written here, never piped, since a fork cannot ask for an approval and a command
written another way may need one; a shell loop or a `$VARIABLE` is refused, so
several files are read one command each:

- **The working tree**: `git diff HEAD`, staged or not, and `git status --short
  --untracked-files=all` for files git does not track yet, each read whole.
- **The branch**: `git diff <default>...HEAD`.
- **A range**: `A..B` is the change from where `B` left `A`, so diff it as `git
  diff A...B`: `git diff A..B` compares the two tips, and shows what `A` gained
  since as undone. `git log A..B` lists its commits.
- **A large change**: `git diff --stat` over the same range first, then one file
  at a time, `-- <file>`.

A range that ends anywhere but the working tree is read as it leaves each file,
with `git show <end>:<path>`, and each line number is taken from that.

In a change, these names are in scope:

- **Added**: each name a `+` line declares, from a crate, a module or a file to
  a field, a parameter or a binding, and a program's flag, subcommand,
  configuration key or environment variable.
- **Renamed**: a declaration a `-` line drops that a `+` line brings back under
  another name, and a file `git diff --stat` shows as `old => new`.
- **Made false**: an existing name whose definition the change alters.

A name the change only calls is out of scope, and so is prose, which is
`writing-prose`'s.

## Order by Reach

List the names in scope, each with its reach, and read them in this order:

1. **Public**: what a caller outside the crate or package can name.
2. **Crate-wide**: what the crate's other modules can name, and no caller
   outside it.
3. **Local**: a private item, a parameter, a binding.

A name's reach is read from where it is declared and from what exports it. A
package is published where its releases go to an index; an unpublished one's
public names reach only the workspace, and are weighed as crate-wide. A binary's
flags, subcommands, configuration and environment reach its users once a release
has shipped them, whether or not the crate is published.

In Rust, a library's item is public where its root exports it, by a `pub use` or
a `pub mod`, and so are an exported type's public fields, variants and methods,
a `#[macro_export]` macro and a Cargo feature; `pub(crate)`, and `pub` in a
private module the root does not export, reach the crate. A parameter is local,
since no caller writes its name. A field or variant that clap or serde derives
into a flag, a key or a value is as public as that string, unless `#[arg(long =
…)]` or `#[serde(rename = "old")]` keeps the old spelling; serde's `alias` keeps
it only when reading. A crate is published unless its `publish`, its own or the
workspace's it inherits, is `false`.

In the shell, a script's path, its flags and the environment variables it reads
are public.

Under a path with more names than the fork can read to their uses, read each
public name whole and the rest by their definitions, and say which in the
report.

## Read the Guides

Read `.claude/skills/writing-readable-code/SKILL.md`, `writing-readable-code`,
whole, before any name: each name is judged by its Names rules, and its Steps
say how a name is found. Each rule's why and example are in
`.claude/skills/writing-readable-code/references/naming.md`: list its headings
with `grep -n '^## '` and its path, and read the section a name needs with
`Read` and an offset and a limit, never the whole file.

For a Rust name, also read `.claude/skills/writing-rust/SKILL.md`'s Names and
Layout rules, `writing-rust`'s, and, the same way, the section of
`.claude/skills/writing-rust/references/naming.md` that a name needs, and "A
Module Is Named for What It Holds" in
`.claude/skills/writing-rust/references/layout.md`, for a module or a file.

For a Cargo feature, also read the Features rules of
`.claude/skills/editing-cargo-manifests/SKILL.md`, `editing-cargo-manifests`'s:
a feature names the thing, `std`, not the switch, `use-std`, and an optional
dependency is `dep:`, so no crate's name is a public feature.

For a shell name, also read the "In the Shell" section of
`writing-readable-code`'s `references/naming.md`.

## Read Each Name

For each name, in order of reach, read before judging:

- **Its definition**, whole: a function's body, a type's fields and `impl`
  blocks, a module's items, the code that reads a flag; and its doc comment.
- **Its uses**: search for the name as a whole word, `grep -rnw <name> <dir>`
  for each directory, never `target/`, and read each call site, for what callers
  expect of it. Search its kebab-case, SCREAMING_CASE and quoted spellings too,
  in docs and Markdown as well as source, and a renamed name's old spelling: a
  use still on it is a caller the change broke.
- **Its tests**: those that call it, in its file, the crate's `tests/` and its
  doc examples, whose names and assertions say what it is held to.

Then say in one sentence, in the codebase's words, what the thing is or does, as
`writing-readable-code`'s Steps ask: the name should be that sentence's noun or
verb, and is judged against it, never against what the name alone suggests.
Where the lines read do not show what the thing is for, the finding says so, and
proposes no name.

## Judge Each Name

Check each name for each kind of finding in turn; a name of several kinds is a
finding for each.

1. **Misleading**: the name says something false about what the thing does: a
   question, `is_`, `has_` or a bare noun, that writes, waits or allocates; a
   prefix that misstates a call's cost or how it refuses; a plural for one
   thing; a type named for a role it does not play; a name the change's new body
   made false. The rule is `writing-readable-code`'s "A Function's Name Says All
   It Does", or the family the name claims. The fix is never a name joined with
   "and", `check_and_book`, but two functions, one that asks and one that acts,
   or one named for what the two make together.
2. **Unclear**: the name does not say what the thing is: a word that fits
   anything, `data`, `info`, `handle`, `process`, `manager` or `util`; an
   abbreviation the domain does not write; a fragment, `at` or `held`, for a
   value; an error's variant or type that names part of its condition, `Held` or
   `HeldError` for `HeldByAnotherEditor`; a name shorter than its reach; a
   negated condition; a name that repeats its module, its receiver or its type.
3. **Inconsistent**: the name clashes with the codebase: a second word for what
   the codebase already names one way, `cell` beside `square`; a type whose last
   word breaks the family its siblings share; an affix its neighbours do not
   use, `fetch_` where they `load_`. The finding cites the names it clashes
   with, by `path:line`, each read; with none to cite, there is none.
4. **Against the language's conventions**, as the guides above give them for the
   language the name is in.
   - In Rust, each heading of `writing-rust`'s `references/naming.md`, and its
     `references/layout.md` for a module.
   - In the shell, a name's case is this pass's, since no lint checks it.

## Leave to the Lints

A Rust name that `just check-rust-clippy` refuses, its casing included, is the
lints', never a finding. The "Held by" line of each rule in the naming
references, and `writing-readable-code`'s Checks, say which lint holds it, and
on which names. Where a lint skips a name, the fault is this pass's: an exported
name, which `wrong_self_convention`, `enum_variant_names`, `struct_field_names`
and `upper_case_acronyms` pass; a private acronym not wholly in capitals,
`TileID`; a name that repeats its module, since `module_name_repetitions` is
off.

## A Public Rename Breaks Its Callers

A rename breaks callers outside the workspace only where a release has shipped
the name to them, as Order by Reach says: the last tag, `git log -1
--format='%(describe:tags,abbrev=0)'`, holds it, as `git show <tag>:<path>`
shows. Otherwise the change moves every caller in the workspace and keeps no
alias, which nothing would call.

Where a rename breaks callers, the finding says so, names each caller in the
repository still on the old name, which the change moves, and keeps the old name
in the fix, deprecated, beside the new: callers get a warning and a release to
move. A change announces a break with `!` after its commit's type or a `BREAKING
CHANGE:` footer; an announced break of an item that allows no alias is named
under Checked as `breaking`, not a finding.

In Rust, a method keeps its old name as a deprecated method that calls the new;
a unit variant, as a deprecated associated constant; a function, a type or a
constant, as a deprecated function, type alias or constant beside the new, which
the root re-exports under an `#[expect]`, since a re-export is a use of what it
names, which rustc warns of. `since` is the version the change sets in
`Cargo.toml` or the changelog, and is left out where it sets none:

```rust
mod grid {
    #[derive(Debug)]
    pub struct Grid {
        columns: u16,
    }

    impl Grid {
        #[must_use]
        pub const fn columns(&self) -> u16 {
            self.columns
        }

        #[deprecated(since = "0.4.0", note = "renamed to `columns`")]
        #[must_use]
        pub const fn get_columns(&self) -> u16 {
            self.columns()
        }
    }

    #[deprecated(since = "0.4.0", note = "renamed to `Grid`")]
    pub type TileGrid = Grid;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Clash {
        Refuse,
        Replace,
    }

    impl Clash {
        #[deprecated(since = "0.4.0", note = "renamed to `Refuse`")]
        #[expect(non_upper_case_globals, reason = "it stands in for a variant, so it is cased as one")]
        pub const Reject: Self = Self::Refuse;
    }
}

pub use crate::grid::{Clash, Grid};
#[expect(deprecated, reason = "the old name stays, deprecated, until the next breaking release")]
pub use crate::grid::TileGrid;
```

`just check-rust-clippy` denies that warning.

The constant is not imported by `use Clash::*`, and a tuple struct's
constructor, `TileId(3)`, does not work through a type alias: E0423.
`#[deprecated]` on a `pub use` compiles and warns no caller, so a tuple or
struct variant, a trait, a module or a field has no alias that warns: its rename
waits for a breaking release, or the old name stays. A Cargo feature keeps its
old name as one that turns on the new, `use-std = ["std"]`, which warns no one.

In the shell, a renamed script's old path stays as one that warns on stderr and
runs the new with `exec` and `"$@"`.

## The Report

The report is Markdown in this form, and nothing else:

````text
## Names in <scope>: <what the change is for>

Purpose read from the commits.
Read <n> names: <n> public, <n> crate-wide and <n> local.

| #   | Where        | Name     | Kind       | Rule                       | Weight   |
| --- | ------------ | -------- | ---------- | -------------------------- | -------- |
| 1   | `<path>:<n>` | `<name>` | misleading | `<skill>`, "<its heading>" | blocking |

### 1. `<name>`, `<path>:<n>`

Read: the definition at `<path>:<n>`; <n> uses, at `<path>:<n>`, …; the tests
at `<path>:<n>`, or none.

<What the name says, what the thing does, and what goes wrong if it stays.>

```<language>
<the fix>
```

Checked, nothing found: <each name read with no finding>.

Not run: <each command refused or failed, and what it would have read>.

Out of scope: <each name not read to its uses, and each fault in a name the
change does not touch>.
````

- **The purpose line** is "Purpose read from the commits" or "Purpose unknown",
  and left out where the arguments gave it.
- **Where** is the path from the root and the line that declares the name, as
  the change leaves it.
- **Kind** is `misleading`, `unclear`, `inconsistent` or `convention`, and
  `breaking`, beside one or alone, where a public rename, the change's or the
  fix's, breaks callers.
- **Rule** is the skill and the heading of the rule, a reference's where one
  holds it; for `breaking` alone, this pass's "A Public Rename Breaks Its
  Callers".
- **Each finding's section** gives the Read line, what the name says against
  what the lines show and what goes wrong if it stays, then the fix, fenced in
  the file's language.
- **Weight** is by what the name costs if it stays, and where two weights fit,
  the heavier holds:
  - **blocking** where a caller outside the crate is led wrong or broken without
    warning: a public name that misleads, since a caller who trusts it writes a
    bug; a released public name renamed with no deprecated alias, where the
    change does not announce the break.
  - **important** where it costs a breaking rename later, or misleads the next
    change: a public name that is unclear, inconsistent or against the
    conventions; a crate-wide or local name that misleads.
  - a **suggestion** where only the crate sees it: a crate-wide or local name
    that is unclear, inconsistent or against the conventions; and an announced
    break whose item allows an alias.
- **Checked, nothing found** names each name with no finding, or counts them
  past twenty; **Not run** is left out where every command ran.

Findings go blocking first, then important, then suggestions, each public before
crate-wide before local, then by path and line. With no finding, the table is
the line "Nothing found."

## Keeping It Honest

- **Every finding stands on lines read and a rule the guides hold.**
- **Only the change.** A fault in a name the change does not add, rename or make
  false goes under Out of scope, once. In a path reviewed whole, every name is
  the change.
- **Counts are counted**, never estimated.
- **Say "nothing found" rather than invent one**, and **change nothing**: read,
  and report.

The kinds of name read, and of finding as a start, come from softaworks'
agent-toolkit `naming-analyzer` (MIT), which judged a name without its body,
endorsed names joined with "and", and knew no public API;
`writing-readable-code`'s `references/sources.md` keeps its notice.
