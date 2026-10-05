# Comments

Read this before writing or changing a comment or a doc comment in any language,
before a `TODO`, and before keeping or deleting an old comment. A comment is
read beside the code by someone who has the code and not the conversation that
produced it, often a year on: it says only what the code cannot, and it stays
true while the code does.

In Rust, `writing-rustdoc` holds how `///` and `//!` read, and its
`references/comments.md` how an inline `//`, an `#[expect]` reason and a message
are worded; the rules below agree with it, and hold for every other language and
file.

What a `// SAFETY:`, an `// INVARIANT:` or an `// ORDERING:` proves, and where
each goes, is `writing-unsafe-rust`'s; none sits on code that needs no proof.

## A Comment Says Why, Not What

The reader has the line; what they lack is the reason for it: a choice between
two ways, an invariant the code keeps, a consequence that is not in sight, a
case left out on purpose. A comment that narrates the line below it says what
the reader has just read, and doubles the text to keep true.

```rust
pub fn rows(text: &str) -> impl Iterator<Item = &str> {
    // Bad: says what the call does, which its name says already.
    // Split the text into lines.
    text.lines()
}
```

```rust
pub fn rows(text: &str) -> impl Iterator<Item = &str> {
    // `lines`, not `split('\n')`: a grid saved on Windows ends each row in
    // `\r\n`, and `lines` takes the `\r` off with the `\n`.
    text.lines()
}
```

Held by review.

## A Name or a Shape, Not a Comment, Says What a Thing Is

A comment that says what a variable holds, or labels the steps of a long
function, is a name or a function that is missing. Rename, or split the
function, as `writing-readable-code` says, and the comment goes: a name is read
at every use, and a comment only where it sits.

```rust
#[must_use]
pub fn fits(text: &str, n: usize) -> bool {
    // Bad: the comment carries what the name should.
    // `n` is the widest row the terminal can draw.
    text.lines().all(|row| row.len() <= n)
}
```

```rust
#[must_use]
pub fn fits(text: &str, widest_row: usize) -> bool {
    text.lines().all(|row| row.len() <= widest_row)
}
```

Held by review.

## A Comment Holds No Process Residue

"Added", "fixed", "changed to", "now", "new", "no longer", "instead of the old",
"as discussed", "per review", a phase, a ticket, a review round: each tells the
story of the change, which the reader of the code does not have and does not
need, and each is false once the change is old. The story belongs in the commit.
What is left is the reason the code is as it is, or nothing. Commented-out code
goes the same way: git keeps it.

```rust
#[must_use]
pub const fn is_square(byte: u8) -> bool {
    // Bad: the history of the line, which git holds.
    // Changed to a match instead of the regex, per review (phase 2).
    matches!(byte, b'.' | b'#')
}
```

```rust
#[must_use]
pub const fn is_square(byte: u8) -> bool {
    matches!(byte, b'.' | b'#')
}
```

Held by review.

In Rust, `just check-rust-doc` runs the doc lint, which refuses `TODO`, `FIXME`,
a phase and a chunk in a comment.

## A Comment Names No Rule and Narrates No Lint

"Clippy wants this", "to make the linter happy", a rule's ID from a style guide
the repository does not ship: the reader learns what a tool asked, not why the
code is right. Where the code is right and the tool misreads it, the
suppression's reason says so, in its one home; where the tool is right, the code
changes and the comment goes.

```rust
#[must_use]
pub fn squares(columns: u16, rows: u16) -> u32 {
    // Bad: narrates the lint instead of saying anything of the code.
    // Clippy complains about `as`, so use `from` (STY-CAST-2).
    u32::from(columns).saturating_mul(u32::from(rows))
}
```

```rust
#[must_use]
pub fn squares(columns: u16, rows: u16) -> u32 {
    u32::from(columns).saturating_mul(u32::from(rows))
}
```

Held by review; `references/messages.md` says how a suppression's reason reads.

## A `TODO` Names Its Issue

A gap in the code that nobody tracks is never closed, and a `TODO` with no issue
is a note to a reader who cannot act on it. A known gap is an issue in the
tracker; where the code must mark it, the comment names the issue and states
what is missing as a fact. `FIXME`, `XXX` and "clean this up later" say that
something is wrong without saying what.

```text
Bad:  # TODO: handle wide grids
Bad:  # FIXME: hacky, clean up later
Good: # TODO(#214): a row wider than 64 squares is refused, not wrapped.
```

Held by review.

Rust source carries no `TODO`, as `writing-rustdoc` holds: the item's doc states
the limit as a fact, "Refuses a row wider than 64 squares.", and the issue holds
the work.

## A Comment Is a Sentence, Above What It Explains

A comment sits on its own line above the code it explains, so it reads before
the code, and has the room a sentence needs; a comment at the end of a line is
read after the code, and is cut short to fit. It is a sentence, capitalized,
with a period, one fact a comment. The same holds in a configuration file, where
the comment says why the value is what it is. A suppression's reason is the
exception: it sits where its tool reads it, on the directive's line, as
`references/messages.md` shows.

```toml
[grid]
# Bad: the name again, at the end of the line.
max_columns = 64 # max columns
```

```toml
[grid]
# A wider row wraps in an 80-column terminal, where the grid is drawn.
max_columns = 64
```

Held by review.

## A Comment Changes with Its Code, or Goes

A comment the code contradicts is worse than none: the reader trusts one of the
two, and cannot tell which. A change to the code changes the comment above it in
the same commit. A comment that says what a name, a type or a test says as well
goes, since two homes for one fact drift.

```rust
#[must_use]
pub fn rows(text: &str) -> usize {
    // Bad: the code skips blank lines; the comment says it counts them.
    // A blank line counts as an empty row.
    text.lines().filter(|row| !row.is_empty()).count()
}
```

```rust
#[must_use]
pub fn rows(text: &str) -> usize {
    // A blank line is no row: a file may end in one, and a grid may be split
    // into parts by one.
    text.lines().filter(|row| !row.is_empty()).count()
}
```

Held by review: nothing compares a comment with its code.

## A Terse Comment That States Its Fact Is Finished

A comment that states its fact in one short line is done. Rewriting it longer,
or into a figure of speech, reads better to its writer and worse to the next
reader, who has to dig the fact back out.

```text
Bad:  // Here we deliberately operate on the level of individual bytes rather
      // than Unicode scalar values, because every square in a well-formed grid
      // is guaranteed to be a single ASCII character.
Good: // Bytes, not chars: every square is `.` or `#`.
```

Held by review.

## A Script Says What It Does, and How It Is Run

A shell script is read by someone about to run it: it opens with a comment that
says what it does, its usage line, and what it needs from where it runs. A
function's comment sits above it and says what it does, not how.

```sh
#!/usr/bin/env bash
# Bad: says it is a script, and nothing of what it does or how it runs.
# Script to check grids.
set -euo pipefail
```

```sh
#!/usr/bin/env bash
# Checks that every grid under grids/ reads, and names each that does not.
#
# Usage: scripts/check-grids.sh [<dir>]
#
# Run in the repository's root, with the cli built.
set -euo pipefail
```

Held by review; ShellCheck holds the script, not its comments.
