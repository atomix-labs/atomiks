---
name: writing-readmes
description: Use when writing, revising or reviewing a README, whether the repository's, a crate's or a command line's; when starting one or filling one in; when adding install lines, a first example, features, the minimum Rust or links to the docs; or when a README must read as well on crates.io as on GitHub. Covers the header, what each section holds and in what order, commands that run as shown, images and links that resolve wherever the README is shown, and what the README leaves to other pages. Not for the sentences themselves, which writing-prose covers.
---

# Writing READMEs

A README is the first page a stranger reads, and often the only one. It says
what the project is and why someone would use it, how to install it, and the
first thing to do with it; then it links everything else. The rules below are
how one is written here, each with its reason; `references/templates.md` holds a
library's, a command line's and a workspace's README whole.

Every sentence and heading in a README follows `writing-prose`: its Every
Sentence and Documents rules while writing, and its `references/tells.md` before
the README is final. This skill does not repeat them.

## Rules

### The Header

1. **Never edit inside the block between `<!-- >>> devset: project >>> -->` and
   `<!-- <<< devset: project <<< -->`**, since the project profile renders it
   and `devset status` reports any edit there as drift. It holds the title, or
   the logo the variable `logo` names; the tagline `description` gives; and the
   badges the applied profiles bring. They change through the variables and
   features that render them, `devset apply --var description="…"`, as
   `using-devset` says.
2. **The block opens the file, since it is the README's title, tagline and
   badges, and a page read from the top meets them first**: a README that was
   there before the profile gets the block appended at its end, so move the
   block, both markers and all between them, to the top, and delete the README's
   own title and tagline. devset finds the block by its markers, so a move is no
   drift.
3. **Nothing the block renders is written again below it**: no second title, no
   tagline, no badge of CI, crates.io, docs.rs or the book, since a copy by hand
   goes stale while the block's follows the project.

### What It Says, in Order

1. **The pitch comes first, in one paragraph that continues from the tagline
   without restating it: what the project does for its reader, and what it
   leaves out where a reader would guess wrong**, since the tagline, the
   manifests' `description`, has just said what it is, and a reader decides here
   whether to read on.
2. **Then Install: the command that installs it, as it runs, and the oldest
   toolchain that builds it**, since a reader who cannot install stops there.
3. **Then Quick Start: the fewest steps from nothing to the project at work,
   each a command or a complete example, with the output a run printed**, since
   the reader copies it; a library shows its first call with its imports, and a
   command line its first command.
4. **The project's own sections sit between Quick Start and Documentation, or
   before Quick Start where a reader needs one to choose**, each named for what
   it holds: a library's Cargo features, the ways a command line is used, how
   the project compares, why it exists; depth past that goes on the pages it
   links.
5. **Then Documentation, Contributing and License, last**, since a reader who
   reaches them has chosen the project and wants the next page, the way to take
   part, and the terms: every page the reader may want next, linked, one line
   each, or a table of what the reader wants to do and where to read it once
   there are more than a handful.
6. **Every command is one the reader can paste**, in a fenced block marked `sh`,
   with no prompt and no placeholder but the ones the reader must fill,
   `<file>`, since a pasted `$` or an invented name fails.

### Rust

1. **A library installs with `cargo add <package>`, and a command line with
   `cargo install --locked <package>`**, never a `[dependencies]` line with a
   version, which goes stale at the next release. `--locked` builds with the
   `Cargo.lock` the package was published with, which Cargo otherwise ignores.
2. **Name the package where it differs from the binary**, "The package is
   `tiles-cli`, and the binary `tiles`", since `cargo install tiles` finds
   another crate or none.
3. **Until crates.io has the crate, install it from git**, `cargo add --git
   https://github.com/<owner>/<repo> <package>`, or `cargo install --locked
   --git … <package>`, since the registry's command fails for a crate not yet
   published.
4. **State the minimum Rust once, in Install, from the workspace's
   `rust-version`**: "`tiles` builds with Rust 1.98 or later.", so a reader on
   an older toolchain knows before the build fails; raising `rust-version`
   changes the sentence in the same change.
5. **A crate with Cargo features lists those a user turns on, what each adds,
   which are on by default, and the command, `cargo add tiles --features
   serde`**, in the same rows as the crate docs' `# Crate features` table, which
   changes with it.
6. **The README and the crate page are two pages, never one**: the crate page is
   `//!` in `lib.rs`, written for someone using the API, never the README pulled
   in with `include_str!`, since a newcomer and a user of the API need different
   pages, and one file serving both needs its links defined twice and its header
   hidden from rustdoc. The README takes its pitch from the crate page's opening
   paragraph, past its summary, and its first example from the crate page, and
   links the crate page on docs.rs for the rest.
7. **A README's code runs nowhere by itself**, so its example is the crate
   page's first doctest, or a cut of it, and the two change together; a command
   line's output is pasted from a run of the build the README describes.

The crate page, its examples and each manifest's `description` are
`writing-rustdoc`'s.

### Links and Images

1. **A long URL is a reference definition at the bottom, `[docs.rs]:
   https://docs.rs/tiles`**, so a paragraph reads as prose and each URL is
   written once.
2. **An image says in its alt text what it shows, and a picture drawn for a
   theme is a `<picture>` with a dark `<source>` and a light `<img>`**, since a
   reader on the other theme would see the wrong one.
3. **Every link to a file of the repository is a relative path, since only
   GitHub shows the README**: GitHub resolves it against the branch it is read
   on, as its documentation recommends, and it holds in a clone, so `just
   check-lychee`, which runs offline, can check it.

## Steps

Read `references/templates.md` whole before writing a README or a section.

1. **A README written or rewritten**: take the template for what the repository
   builds, write each section in order under the header, and run every command
   it shows. First read what it describes: CONTRIBUTING.md, each `Cargo.toml`
   for `name`, `description`, `rust-version`, `[features]` and `[[bin]]`, the
   crate page's `//!`, and the command line's usage.
2. **A section added or changed**: find every place the README states the fact,
   the install line, the minimum Rust, a feature, a binary's name, and change
   them together with what changed.
3. **Reviewing a README**: check each claim against the manifest and a run:
   every command pasted into a fresh directory, every output against what it
   printed, every link opened, every version against the current one.

Before finishing, read the page as it renders, on GitHub.

Then run `just fix`, and the checks below.

## Checks

- By hand: each install line and Quick Start step, pasted into a fresh
  directory, with `--git` or `--path` in place of the registry until the first
  release.
- `just check`: every check, as CI runs them, after `just fix`.
- `just check-markdown`: rumdl, headings in title case among its rules.
- `just check-devset`: every file and block devset manages, as the profiles
  render them.
- `just check-lychee`: every link into the repository, offline; `just
  nightly-lychee` checks the absolute ones too, against the web.

## What Not to Do

| Thought                                       | Instead                                                      |
| --------------------------------------------- | ------------------------------------------------------------ |
| "The block's title wants a capital"           | The block is devset's: change the variable it renders.       |
| "The old title can stay above the block"      | The block goes to the top, whole; the old title goes.        |
| "A tagline and badges under the block"        | The block has both.                                          |
| "An absolute URL, to be safe"                 | A relative path: only GitHub shows it, on any branch.        |
| "`cargo install tiles`"                       | The package, `cargo install --locked tiles-cli`.             |
| "`tiles = "0.3"` under `[dependencies]`"      | `cargo add tiles`, which takes the newest.                   |
| "`#![doc = include_str!("../README.md")]`"    | Two pages for two readers: the crate page is its own `//!`.  |
| "Supports Rust 1.70+, probably"               | The workspace's `rust-version`, as it is.                    |
| "A section for each part of the API"          | The API's own docs have them: link those.                    |
| "The whole of `--help` belongs in the README" | The first command and its output; the help is the command's. |
| "The output reads right"                      | The output a run printed, pasted.                            |

## References

Read every reference a task touches before writing, and read them again after
compaction: this body is the summary, and the templates are there.

- `references/templates.md`: before a README or any section of one: a library's,
  a command line's and a workspace's, whole, each crate's own README.
- `references/sources.md`: before citing a source for a rule, or adapting text
  from one.
