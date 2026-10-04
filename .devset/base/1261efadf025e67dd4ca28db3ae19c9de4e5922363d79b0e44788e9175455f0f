---
name: writing-rustdoc
description: Use when writing, revising or reviewing rustdoc (`//!`, `///`), `// SAFETY:`, `// INVARIANT:` and `// ORDERING:` comments, `#[expect]` reasons, assertion and `expect` messages, the `//!` of a test, bench or example file, or a crate's `Cargo.toml` description; when a crate, module, type, trait or `unsafe fn` is added or changed and its docs must follow; when asked to document a crate, make it docs.rs ready, clean up its docs, or audit them for restatement, filler, widows or missing Safety, Errors and Examples sections. Covers summaries by item kind, body prose, sections, examples, links, comments, messages, file headers, docs.rs and the manifest's description. Not for Markdown outside Rust source, or what a safety proof must establish, which writing-unsafe-rust covers.
---

# Writing Rustdoc

How docs and comments are written in this workspace. The reader is a Rust
engineer with the code open beside the docs, so a doc says only what the code
cannot: why an item exists, what it promises, what it assumes, what it costs;
and it links the rest. Compact means no restatement, never less content. The
rules below are the whole of it, each with its reason; the references hold the
tables, templates and exemplars, whose examples are docs and so document every
item they show.

## Rules

### What a Doc Says

1. **Only what the code, its tests, its manifest or its history say**: a sign
   convention, a unit, an order, a cost or a promise the code does not state is
   a guess, however plausible, so the sentence is left out and asked for, never
   filled with a plausible reason, a link not opened or output not produced.
2. **Nothing the code already says**: no summary that paraphrases the signature,
   no `# Arguments` or `# Returns`, no module doc listing its items, no comment
   narrating the next line, no doc on a `mod x;` line; a sentence whose deletion
   loses nothing goes.
3. **Link, do not re-explain**: an item with a home is ``[`Name`]`` on its first
   use in a block, an external fact a reference definition at the block's
   bottom, since the target says it once. A crate speaks for itself: it links a
   dependency where a guarantee comes from it, and never names a dependent.
4. **A cost is stated with what shows it**, the code path, a listing or a
   measurement, since "allocates nothing" read off the types is a guess.
5. **The body says, only where true and in this order, why the item exists and
   when to use it, ``Prefer [`x`] for …, so …``; its contract, what it
   guarantees, in what order, what remains on failure; the reason for each
   choice that is not obvious, as a fact and its consequence; its cost**, since
   the code cannot show those. Three properties or more are bold-lead bullets.
6. **One word for one thing**: the crate's own nouns kept, a coined noun defined
   once where its type is and linked after.

### Summaries

1. **The summary is one sentence, on one line where it fits, in its kind's form,
   and says more than the name**, since rustdoc shows it in every listing;
   `references/style.md` §1 has every kind, and what not to write:

| item          | summary                                    | write                                            |
| ------------- | ------------------------------------------ | ------------------------------------------------ |
| crate         | the pitch, the manifest's `description`    | Boards of tiles, and the moves across them.      |
| module        | one line, naming its key item              | The moves a tile makes across a [`Board`].       |
| fn            | a verb first, present tense                | Slides toward `direction`, or `None` at an edge. |
| getter        | the value: `How many …`, `Whether …`       | How many squares hold a tile.                    |
| constructor   | the state it yields                        | A board of `columns` by `rows` empty squares.    |
| type          | its role; a verb phrase for an active one  | A square of a board, by column and row.          |
| trait         | `How to …` for a capability, else the role | How to paint a tile onto a square.               |
| field         | a noun phrase; its invariant after it      | Tiles on the board. At most one a square.        |
| error type    | `Why …`                                    | Why a column and row name no square of a board.  |
| error variant | the condition, as a fact                   | The column is past the board's last.             |

### Sections

1. **`# Safety`, `# Errors`, `# Panics`, then `# Examples`, after the prose,
   each heading after a blank `///`, its content on the next line**, so a reader
   learns once where each fact is. An `# Errors` entry names the variant, then
   the condition as a fact, a `- ` list for several, as below;
   `references/templates.md` §4 has the example whole.
2. **An example shows why the item is used, with a type of the domain and a real
   quantity, and ends in an assertion whose message continues the sentence**,
   since a reader knows how to call a function. It unwraps with an `expect` that
   says why the call cannot fail, never `unwrap()`; it closes `?` with a hidden
   `# Ok::<(), E>(())`; it is `no_run` for I/O, `text` for what is not Rust,
   never `ignore`. Every public type a user builds or drives, and every
   substantial function, has one; siblings, each clock, point or error, have one
   each or none; a trivial accessor has none.
3. **The crate page gives the pitch, then the problem or the shape, `# Examples`
   with a gerund `##` for each of several tasks, `# Crate Features` as a table
   where there are features, and the reference links last, every heading in
   title case**, so a newcomer can start there; `# Types` groups the exports by
   role, three to five bullets.

````text
/// The square at `column` and `row`, once both fall on the board.
///
/// # Errors
/// - [`PositionError::ColumnOffBoard`], `column` is past the last column.
/// - [`PositionError::RowOffBoard`], `row` is past the last row.
///
/// # Examples
/// ```
/// use tiles::{Board, Position, PositionError};
///
/// let board = Board { columns: 8, rows: 8 };
/// assert_eq!(board.position(3, 1)?, Position { column: 3, row: 1 }, "a square of the board");
/// # Ok::<(), PositionError>(())
/// ```
pub const fn position(self, column: u16, row: u16) -> Result<Position, PositionError>
````

A single variant is one line. An error passed up names the state it leaves,
"Whatever the source reports, having written nothing."; an identical contract is
``As [`open`](Self::open).`` and nothing more. An `unsafe fn`'s `# Safety` lists
the caller's obligations, comma-separated; an `unsafe trait`'s is the impl's
promise, what `Ok` and `Err` each mean.

### Comments

1. **A `// SAFETY:` sits above each `unsafe` block and `unsafe impl`, above the
   `#[expect]` that goes with it, and proves each precondition from a fact in
   scope**, the caller's contract, the step before or a field's invariant, in as
   many lines as the preconditions need; it never restates the operation or says
   "this is safe because". Reasoning that proves nothing sits above it, parted
   by a bare `//`.
2. **A field unsafe code relies on carries an `// INVARIANT:` where it is
   declared, above its `///`, saying what every writer keeps and naming the
   writers**, and each `// SAFETY:` that rests on it says "by the field
   INVARIANT", so a change to the field meets what it must keep.
3. **Every atomic operation has an `// ORDERING:` above it, `SeqCst` included,
   naming its ordering and what it pairs with**, "Release, pairing with the
   Acquire load in `winner`", or that it publishes nothing, so each pair is
   checked from both ends; one ordering throughout says "Relaxed throughout"
   once, atop the function, and a repeat chains, "Relaxed, as in `stop`".
4. **An `#[expect(lint, reason = "…")]` reason is its cause, one lowercase
   clause with no period, naming what the lint misreads**, never the consequence
   or the lint; it is the reason's one home, so no comment explains it.
5. **An inline `//` says only what the code cannot, a reason, an invariant, a
   deliberate absence, one fact a comment, capitalized, with a period**; a terse
   line that states its fact is finished.
6. **A message is a lowercase fragment with no period**: an assertion's says the
   property, continuing its test's sentence; an `expect`'s, in a test or an
   example, why the call cannot fail, and in library code the precondition; a
   `debug_assert!`'s the violation; an `#[error]`'s reads `"<type words> error:
   <fragment>"`, its fields inline.
7. **Every file opens with a `//!`**: `lib.rs` the crate page; a module its
   role, in one line; a file of `tests/` what it proves; a compile-fail fixture
   the unsoundness it prevents; a bench what the gap between its arms measures;
   an example what it shows and how to run it. A test's name is its property.

### Manifest and Words

1. **The manifest's `description` is the crate summary's pitch clause, its first
   letter lowercase, with a trailing period, never calling the crate a crate**,
   since crates.io shows it and refuses a crate without one; an acronym or a
   name that would open it moves further in.
2. **No process residue and no em dash**: no `TODO`, ticket, phase or rule id,
   no lint narration; a colon, semicolon, comma or parenthesis for a dash. Every
   identifier, path, literal, feature, lint and command is in backticks.

### Cut List

Delete on sight; `references/style.md` §4 gives each rewrite.

- **Openers**: `This function …`, `A struct that …`, `Represents …`, `Returns
  …`, `Creates a new …`, `Used to …`, `Allows …`, `Provides …`, `Helper`,
  `Wrapper`: an article and the item's kind is the fault, and `The raw address.`
  is fine.
- **Filler**: `simply`, `just`, `basically`, `note that`, `in order to`, `it is
  important`, `please`, `will`, `various`, `etc.`, `see also`.
- **Unverifiable**: `powerful`, `simple`, `easy`, `efficient`, `robust`,
  `zero-cost`, `generic`, and a cost with nothing behind it.
- **Restatement**: a parameter list, the return type, a field's type, the derive
  list, the summary again, a module doc listing its members, a comment
  explaining a neighbouring `#[expect]`.
- **Structure**: a blank line after `# Safety`, `# Errors`, `# Panics` or `#
  Examples`; `# Example`, `# Arguments`, `# Returns`, `# Overview`, `# Usage`,
  `# Notes`; a heading over one sentence, a list of two, a `# Types` bullet for
  each export.
- **Debris**: `TODO`, `FIXME`, `(?)`, `should probably`, a placeholder link, an
  invented issue, a number with no run behind it, phase and chunk labels.
- **Padding**: a clause added to a finished one-liner.

The lints hold the sections: `# Errors`, `# Panics` and `# Safety` on a public
function that needs them, and backticks on an identifier in prose.

Under `strict` they hold more: every item documented, private ones and fields
included, though not a test's helpers; a `// SAFETY:` on each unsafe block and
`unsafe impl`, and none on safe code; a first paragraph short enough for a
listing; a message on every assertion outside a test.

Under `strict`, `just check-rust-doc` runs the doc lint, `.just/rust-doc.py`: it
refuses a weak opener, filler, an unverifiable word, a first paragraph that is a
body, a misnamed section or a blank line after one, `# Errors` as prose,
`ignore` or `unwrap()` in an example, a module with no `//!`, a doc on a `mod`
line, a `// SAFETY:` that asserts, a process marker, an em dash, a crate named
that is neither this one, its family nor a dependency, and a `description` or
dependency table out of its form.

## Steps

Read each reference a step names, whole, before writing.

1. **Documenting or auditing a crate**: read it whole, its manifest, modules,
   `tests/`, `benches/` and `examples/`, and match the voice there;
   `.claude/skills/writing-rustdoc/scripts/doc-inventory.py <crate-dir>` lists
   each item and the sections it has; find the facts in comments, test names,
   `git log -p -- <path>` and design documents; ask for the rest (step 7); write
   top down, crate, modules, items, fields, comments, tests and examples, then
   the `description`. `references/style.md`, `references/templates.md`,
   `references/exemplars.md`, and `references/tooling.md` §7 for docs.rs.
2. **A new or changed item**: its summary, prose, sections, and an example where
   a user builds or drives it: `references/style.md`, and
   `references/templates.md` §3 and §4.
3. **A crate page or a module doc**: `references/templates.md` §1 and §2, §7 for
   a diagram, §8 for a table.
4. **An `unsafe` block, `unsafe fn` or `unsafe impl`, a field unsafe code relies
   on, or an atomic**: `references/comments.md` §1 to §3, and
   `writing-unsafe-rust` for what each proof must establish.
5. **A comment, an `#[expect]`, a message, a test, or a file of `tests/`,
   `benches/` or `examples/`**: `references/comments.md`; what a test pins, and
   how, is `writing-rust-tests`'s.
6. **The manifest**: `references/comments.md` §9 for its `description`; the rest
   is `editing-cargo-manifests`'s.
7. **A gap no fact fills**: ask once, before writing, in one numbered list, each
   entry the item, the sentence with its gap marked, and the fact needed: a
   departure with no reason on record, a `# Safety` contract neither the callee
   nor the code states, a cost with no listing, a citation not found, an item
   that may be surface or plumbing, two docs that disagree. Without an answer,
   write the rest and leave those sentences out, never a placeholder.
8. **Before finishing**: `just fix`; then cut or rebalance each paragraph whose
   last line holds one to three words, since the formatter never joins lines;
   the checks below; the rendered page. Report what was written where, each
   command's result, and each question still open.

## Checks

- `just check`: every check, as CI runs them, after `just fix`.
- `just check-rust-doc`: rustdoc over every crate and feature, private items
  included, a warning failing it, then the doc lint.
- `.claude/skills/writing-rustdoc/scripts/doc-audit.sh <crate-dir>`: formatting,
  rustdoc, doctests, clippy and the doc lint, `.just/rust-doc.py`, over one
  crate, each clean before the next; `--lint-only --advisory` is the quick
  reread, with widows and summaries that wrap.

Then read the page the audit builds, `target/<host>/doc/<crate>/index.html`:
every ``[`Name`]`` a link, and no heading over an empty paragraph.

Done means every public item, field and variant has a summary that says more
than its name, and every file its `//!`; every unsafe site, field a proof relies
on and atomic its comment, and every section its place; every type a user
builds, every substantial function and their siblings, a runnable `# Examples`;
`description` set; no cut-list item left, nor a widow; the checks passing.

Under `strict`, every private item has its summary too.

## What Not to Do

| Thought                                          | Instead                                                 |
| ------------------------------------------------ | ------------------------------------------------------- |
| "A negative offset probably means earlier"       | The code does not say it: leave it out, and ask.        |
| "The crate is close; I'll fix the gaps I see"    | The inventory and the audit say what is left.           |
| "``[`PositionError`] if off the board`` works"   | ``[`Variant`], <condition>.``, a `- ` list for several. |
| "The three-line summary is one thought"          | One sentence, a blank `///`, then the rest.             |
| "`ignore` for now; the example needs setup"      | `no_run`, a `text` fence, or a smaller example.         |
| "This path allocates nothing; the types show it" | Say what shows it, a listing or a measurement, or ask.  |
| "A `// SAFETY:` is one line"                     | Each precondition and its fact, however many lines.     |
| "`// SAFETY:` between `#[expect]` and the code"  | Above the `#[expect]`, so the proof reads first.        |
| "`SeqCst` needs no `// ORDERING:`"               | Every atomic has one: say what the ordering is for.     |
| "A comment beside the `#[expect]` explains it"   | Its `reason` is its one home.                           |
| "My rewrite of that comment reads better"        | A terse line that states its fact is finished.          |
| "The description is out of scope for docs"       | It is the crate's pitch on crates.io: write it, or ask. |
| "A newtype needs an example, even a tautology"   | Not if a neighbour's shows it and no sibling has one.   |
| "This sibling crate's concept needs a paragraph" | Link the crate that defines it; write it there if not.  |
| "No time to read the tests and the history"      | A doc without its facts costs more later: read them.    |
| "The checks are slow; `cargo fmt` is enough"     | They take seconds on a warm cache: run them.            |

## References

Read every reference a task touches before writing, and read them again after
compaction: this body is the summary, and the tables and templates are there.

- `references/style.md`: before any summary or body: each kind's form, the
  prose, compaction, the cut list's rewrites, examples, links and words.
- `references/templates.md`: before a crate page, a module, an item, an error
  type, a test, bench or example file, a `description`, a diagram or a table.
- `references/comments.md`: before a `// SAFETY:`, `// INVARIANT:`, `//
  ORDERING:` or inline comment, an `#[expect]` reason, a message, a test, a file
  header or a manifest.
- `references/exemplars.md`: to see a rule on the page, in annotated excerpts
  from three exemplar crates.
- `references/tooling.md`: before the checks: commands, the lints that shape
  docs, cfgs, links rustdoc cannot resolve, doctest attributes, docs.rs.
- `references/sources.md`: before a doc makes an external claim, or when a
  rule's source is asked for.
