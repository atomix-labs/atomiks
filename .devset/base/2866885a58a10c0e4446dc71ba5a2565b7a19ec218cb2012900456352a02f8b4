# Attack Classes

Read this once the change's trust boundaries are mapped, a section at a time:
each class that a boundary needs, found by its heading with `grep -n '^## '`.
Each section is one class: the rule as its heading, why a path through it harms,
a bad example and a good one, and what holds it. A finding cites the heading of
the section it read, and its path comes from the change's own code, never from
an example here.

The examples are Rust; each rule holds in any language.

## A Path from Input Stays Under Its Base

`base.join(name)` keeps each `..` of the name, and a name that is absolute
replaces the base whole, so a tile set named `../../home/ada/.ssh/id_ed25519` or
`/etc/shadow` is read from outside the tile directory. A name from input is one
plain component, refused otherwise, before it is joined; a path that must have
several is resolved, with the base, and compared with it. An archive is the
same: each entry's name is a path its author chose.

```rust
use std::fs;
use std::io;
use std::path::Path;

// Bad: `..` and an absolute name both leave `sets`.
pub fn read_set(sets: &Path, name: &str) -> io::Result<Vec<u8>> {
    fs::read(sets.join(name))
}
```

```rust
use core::str::FromStr;
use std::fs;
use std::io;
use std::path::{Component, Path};

use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("parse set name error: want one path component, not a path")]
pub struct ParseSetNameError;

/// A tile set's name: one plain component, so it stays in the directory it joins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetName(String);

impl FromStr for SetName {
    type Err = ParseSetNameError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parts = Path::new(text).components();
        match (parts.next(), parts.next()) {
            (Some(Component::Normal(_)), None) => Ok(Self(text.to_owned())),
            _ => Err(ParseSetNameError),
        }
    }
}

pub fn read_set(sets: &Path, name: &SetName) -> io::Result<Vec<u8>> {
    fs::read(sets.join(&name.0))
}
```

In a base someone else wrote, a repository being built or an unpacked archive, a
plain name can still be a link that leads out. There, a name that is a link is
refused, through `symlink_metadata`, or opened with `O_NOFOLLOW` where they can
still write; or the path is resolved, with the base, and compared with it:

```rust
use std::fs;
use std::io;
use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReadSetError {
    #[error("read set error: the set leads outside the checkout")]
    OutsideCheckout,
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// A tile set from a checkout someone else wrote, links and all.
pub fn read_checked_out_set(checkout: &Path, name: &str) -> Result<Vec<u8>, ReadSetError> {
    let base = checkout.canonicalize()?;
    let path = base.join(name).canonicalize()?;
    if !path.starts_with(&base) {
        return Err(ReadSetError::OutsideCheckout);
    }
    Ok(fs::read(path)?)
}
```

A resolved path compared with its base holds where nobody writes under the base
while it is read; where someone still can, a link they swap in after the
comparison leads out again, which is a race on files. An archive's link entry
redirects each entry written through it after, so an extractor refuses a link
whose target leaves the target directory, and extraction goes through an API
that checks each entry, never a loop that joins the entries' names by hand.

A checked name held as a type is `writing-rust`'s "Check at the Boundary, and
Let the Type Carry the Proof", in
`.claude/skills/writing-rust/references/api-design.md`.

Held by review: no lint sees where a path came from.

## A Command Takes Input as Arguments, Never as Shell Text

`sh -c` parses its string as a script, so a sheet named `a.png; rm -rf ~` runs
both commands. `Command` with each value an `arg` runs no shell, and the program
gets each as one argument; `--` before a name from input ends the options, so a
name that starts with `-` is not read as one. Some programs run their options as
code, `git -c`, `ssh -o ProxyCommand=`, `find -exec`, so a value from input
never reaches one as an option, shell or no shell.

```rust
use std::io;
use std::process::{Command, ExitStatus};

// Bad: the shell reads the sheet's name as part of the script.
pub fn compress(sheet: &str) -> io::Result<ExitStatus> {
    Command::new("sh").arg("-c").arg(format!("gzip -k {sheet}")).status()
}
```

```rust
use std::io;
use std::path::Path;
use std::process::{Command, ExitStatus};

pub fn compress(sheet: &Path) -> io::Result<ExitStatus> {
    Command::new("gzip").arg("-k").arg("--").arg(sheet).status()
}
```

The program is a constant, never input, and input never names a variable of the
environment set for it, `PATH` or `LD_PRELOAD` among them.

Held by review.

## A Count from Input Is Bounded Before It Sizes an Allocation

A header that says how many tiles follow is a claim, and `Vec::with_capacity`
believes it before one tile has arrived: four bytes ask for gigabytes. Where the
allocator cannot give them, the process aborts, which no caller can catch, and a
capacity past `isize::MAX` bytes panics. A count sizes nothing until the input
holds what it claims, or it is bounded by a limit where the input is a stream;
or the collection grows as the items arrive.

```rust
/// A grid's tiles, as its file lays them out: a count, then a byte for each.
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<Vec<u8>> {
    let (head, body) = bytes.split_first_chunk::<4>()?;
    let count = usize::try_from(u32::from_le_bytes(*head)).ok()?;
    // Bad: four bytes ask for four gigabytes, whatever follows them.
    let mut tiles = Vec::with_capacity(count);
    tiles.extend(body.iter().take(count).copied());
    (tiles.len() == count).then_some(tiles)
}
```

```rust
/// A grid's tiles, as its file lays them out: a count, then a byte for each.
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<Vec<u8>> {
    let (head, body) = bytes.split_first_chunk::<4>()?;
    let count = usize::try_from(u32::from_le_bytes(*head)).ok()?;
    // The count sizes nothing until the bytes that back it are there.
    body.get(..count).map(<[u8]>::to_vec)
}
```

A `try_reserve` turns a failed allocation into an error, but still reserves what
the input claims where the allocator can give it. Under Linux's default, an
allocation the host cannot back usually still succeeds, and costs nothing until
it is touched: the harm is the abort where the allocator refuses, and the memory
the decoder fills after.

Held by review.

## A Decoder Reads to a Limit and Nests to a Depth

A reader read to its end holds as much as its sender sends, and a decoder that
recurses once for each nested group overflows its stack on input nested deep
enough, which aborts the process. Input from outside is read through
`Read::take` with a limit, and a recursive decoder counts its depth and refuses
past a bound. A format crate's limits count: serde_json stops at 128 levels
unless `disable_recursion_limit` is called, a method only its `unbounded_depth`
feature provides, and a format that takes a size limit is given one.

```rust
use std::io::{self, Read};

// Bad: holds whatever the sender sends, however much.
pub fn read_sheet<R: Read>(mut source: R) -> io::Result<Vec<u8>> {
    let mut sheet = Vec::new();
    source.read_to_end(&mut sheet)?;
    Ok(sheet)
}
```

```rust
use std::io::{self, Read};

use thiserror::Error;

/// A tile sheet is shorter than this, in bytes.
pub const SHEET_LIMIT: u64 = 1_048_576;

#[derive(Debug, Error)]
pub enum ReadSheetError {
    #[error("read sheet error: the sheet reaches the {SHEET_LIMIT} byte limit")]
    TooLong,
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub fn read_sheet<R: Read>(source: R) -> Result<Vec<u8>, ReadSheetError> {
    let mut sheet = Vec::new();
    let mut limited = source.take(SHEET_LIMIT);
    limited.read_to_end(&mut sheet)?;
    if limited.limit() == 0 {
        return Err(ReadSheetError::TooLong);
    }
    Ok(sheet)
}
```

A file on disk has a size to compare with the limit before it is read, but its
length can change as it is read, so the read is bounded too. A decompressor's
output is input as well: deflate expands up to a thousandfold, so the limit is
on what comes out, a `take` on the decoder, not on the bytes read in.

Held by review.

## A Secret Reaches No Log, Error or `Debug`

A log is read by whoever runs, collects or is sent it, and an error by whoever
sees the output, a CI job's log included: a token printed in either is theirs. A
derived `Debug` prints every field, so `{server:?}` in a log line or a panic, or
an error that holds the configuration, prints the token with it. A secret is a
type whose `Debug` leaves it out and that has no `Display`, and a message names
the secret, never its value; a URL with a password in it is a secret too.

```rust
/// Where tiles are fetched from, and the token that lets the fetch in.
#[derive(Debug)]
pub struct TileServer {
    pub url: String,
    // Bad: `{server:?}` prints the token with the url.
    pub token: String,
}
```

```rust
use core::fmt;

/// A tile server's token, which its `Debug` never prints.
pub struct Token(String);

impl Token {
    #[must_use]
    pub const fn new(token: String) -> Self {
        Self(token)
    }

    /// The token itself, for the one header that sends it.
    #[must_use]
    pub fn get(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(..)")
    }
}

/// Where tiles are fetched from, and the token that lets the fetch in.
#[derive(Debug)]
pub struct TileServer {
    pub url: String,
    pub token: Token,
}
```

An error that refuses a secret still hands it back, as `writing-rust`'s "A
Refused Value Goes Back to the Caller" asks, in
`.claude/skills/writing-rust/references/errors.md`: the field is skipped in its
`Debug`, as that section shows, and its message never renders it.

A secret on a child's command line is readable by every local user, through `ps`
or `/proc`, and one in the environment reaches every child that inherits it: a
secret goes to a child on its stdin or in a file only it can read, and
`Command::env_remove` keeps it from a child that needs none.

That a log line holds no secret is `writing-prose`'s rule too, in "A Log Line Is
a Stable Phrase, Its Values as Fields", in
`.claude/skills/writing-prose/references/messages.md`; this class traces the
path by which one gets in.

Held by review: no lint knows which string is a secret.

## A File Is Checked Through the Handle That Uses It

A check on a path and a use of it are two lookups, and in a directory someone
else can write, a shared temporary directory, an upload area, a checkout of a
stranger's repository, they change what the path names between the two: a link
put where `exists()` saw nothing turns the write into one through the link, to a
file the process can write and they cannot. The check is made the use:
`create_new(true)` fails on anything at the path, a dangling link included; and
what was opened is checked through the file's own `metadata()`, not the path's.

```rust
use std::fs;
use std::io;
use std::path::Path;

pub fn export(grid: &[u8], destination: &Path) -> io::Result<()> {
    // Bad: a link made at `destination` after this check is written through.
    if destination.exists() {
        return Err(io::Error::from(io::ErrorKind::AlreadyExists));
    }
    fs::write(destination, grid)
}
```

```rust
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;

pub fn export(grid: &[u8], destination: &Path) -> io::Result<()> {
    OpenOptions::new().write(true).create_new(true).open(destination)?.write_all(grid)
}
```

Where only the program's user writes the directory, nobody else can swap the
path, and the race is none.

Held by review.

## A Value from Input Reaches a Log or a Terminal Escaped

A newline in a value from input starts a line of its own in a log, so a tile set
named `sea\nINFO admin signed in` forges an event others act on; and an escape
sequence that reaches a terminal moves the cursor, rewrites what is shown or
sets the window's title. A value from input goes into a log line or a terminal's
output escaped: `{name:?}` escapes every control character, and a structured log
keeps the value a field.

```rust
use std::io::{self, Write};

pub fn report_missing<W: Write>(out: &mut W, name: &str) -> io::Result<()> {
    // Bad: a newline in the name starts a line of the sender's.
    writeln!(out, "tile set error: no set named {name}")
}
```

```rust
use std::io::{self, Write};

pub fn report_missing<W: Write>(out: &mut W, name: &str) -> io::Result<()> {
    writeln!(out, "tile set error: no set named {name:?}")
}
```

How a log line and an error read is `writing-prose`'s "A Log Line Is a Stable
Phrase, Its Values as Fields" and "An Error Says What Was Wrong, with the Value
and Where", in `.claude/skills/writing-prose/references/messages.md`; this class
asks only whether input reaches them unescaped.

Held by review.

## A Decoder Never Panics on Its Input

An index past the end, a slice out of range, an `unwrap` of a failed parse, a
`split_at` past the length, a `copy_from_slice` of the wrong length and, in a
debug build, an overflow: each panics, and a panic on input stops the process
that decodes it, with every other request it serves. A decoder reaches each byte
through `get`, `split_first_chunk` or `split_at_checked`, and refuses what does
not fit with an error. A panic that stops a process others rely on weighs
medium; one in a CLI only its user runs is the user's.

That a caller's input is refused with an error, never a panic, is
`writing-rust`'s "Return an Error for Anything a Caller Can Cause", in
`.claude/skills/writing-rust/references/errors.md`; this class asks only whether
input reaches the panic.

```rust
/// The tile a record names: its first two bytes, little-endian.
#[must_use]
pub const fn tile(record: &[u8]) -> u16 {
    // Bad: a record shorter than two bytes panics.
    let (head, _rest) = record.split_at(2);
    let mut id = [0; 2];
    id.copy_from_slice(head);
    u16::from_le_bytes(id)
}
```

```rust
/// The tile a record names: its first two bytes, little-endian.
#[must_use]
pub fn tile(record: &[u8]) -> Option<u16> {
    let (id, _rest) = record.split_first_chunk::<2>()?;
    Some(u16::from_le_bytes(*id))
}
```

Held by `clippy::indexing_slicing`, `clippy::unwrap_used`,
`clippy::expect_used`, `clippy::panic` and `clippy::arithmetic_side_effects`
under `strict`, and by review for the rest: `split_at`, `copy_from_slice`,
`Vec::remove` and their kin, which no lint sees.

## Size Arithmetic on Input Is Checked

A grid of 65,536 rows by 65,536 columns is one tile more than a `u32` counts, so
the product wraps to none, a buffer sized by it holds nothing, and a bound
checked against it passes indexes the grid does not have: a panic in safe code,
a read or write out of bounds in `unsafe` code. A product, sum or difference
that sizes an allocation, a bound or an offset from input is `checked_`, and its
overflow is a refusal: `wrapping_` and `saturating_` both give a size that is
wrong.

```rust
#[must_use]
pub const fn grid_size(rows: u32, columns: u32) -> u32 {
    // Bad: 65,536 rows of 65,536 wrap to a grid of none.
    rows.wrapping_mul(columns)
}
```

```rust
#[must_use]
pub const fn grid_size(rows: u32, columns: u32) -> Option<u32> {
    rows.checked_mul(columns)
}
```

`rows * columns` wraps the same way wherever overflow checks are off, as they
are in a release build by default.

How each operation says how it overflows is `writing-rust`'s "Arithmetic Says
How It Overflows", in `.claude/skills/writing-rust/references/lints.md`; for a
size from input, the answer is `checked_`.

An `as` that narrows a length cuts it as the product wraps: `length as u32` of
five billion is 705,032,704. `clippy::cast_possible_truncation`, in the pedantic
group the lint table denies, refuses it; `u32::try_from` says how it fails.

Held by `clippy::arithmetic_side_effects`, which refuses the operator under
`strict`, and by review, which holds the method chosen in its place.

## Input Never Decides an Unsafe Precondition Unchecked

An `unsafe` operation's preconditions keep the process's memory whole, and a
value from input that reaches one, an index into `get_unchecked`, a length into
`from_raw_parts` or `set_len`, bytes into `from_utf8_unchecked`, lets its sender
choose what memory is read or written. The finding is the path: the input, the
operation it reaches, and the check missing on the way.

What a proof must establish is `writing-unsafe-rust`'s: "A Safe Function Is
Sound for Every Input" and "A `// SAFETY:` Proves Each Precondition from a Fact
in Scope", in
`.claude/skills/writing-unsafe-rust/references/safety-comments.md`. A proof that
cites a check the input can go around proves nothing.

```rust
#[derive(Debug)]
pub struct Palette {
    colours: Vec<u32>,
}

impl Palette {
    /// The colour a tile's byte names, from a sheet someone else wrote.
    #[expect(unsafe_code, reason = "the lookup is on the decoder's hot path")]
    #[must_use]
    pub fn colour(&self, tile: u8) -> u32 {
        // Bad: the byte is the sheet's, and nothing bounds it by the palette.
        // SAFETY: a tile names a colour of the palette.
        unsafe { *self.colours.get_unchecked(usize::from(tile)) }
    }
}
```

```rust
#[derive(Debug)]
pub struct Palette {
    colours: Vec<u32>,
}

impl Palette {
    /// The colour a tile's byte names, from a sheet someone else wrote.
    #[must_use]
    pub fn colour(&self, tile: u8) -> Option<u32> {
        self.colours.get(usize::from(tile)).copied()
    }
}
```

Held by review: under `strict`, `clippy::undocumented_unsafe_blocks` asks for a
`// SAFETY:`, and never whether it is true.

## A Dependency Carries No Advisory Its Code Reaches

A dependency runs with the program's authority, and a published advisory is an
attack anyone can read. `just check-cargo-deny` checks each crate in the lock
file against the RustSec database, and refuses a yanked or unmaintained one:
what it reports is its own. Review holds what it cannot: an exception the change
adds to `[advisories] ignore` is the maintainer's choice, and its reason says
why the advisory's code is not reached here.

```toml
[advisories]
ignore = ["RUSTSEC-2020-0071"]
```

```toml
[advisories]
ignore = [
    { id = "RUSTSEC-2020-0071", reason = "nothing calls `now_local`, `local_offset_at` or `time::now`" },
]
```

A finding against a dependency is still a path: the advisory's function, and the
call in this change that reaches it with input.

Held by `just check-cargo-deny`, and by review for each exception.

## A Shell Script Runs No Input as Code

`eval`, `sh -c` and `bash -c` parse their string as a script, so a name from
input with `;` or `$(…)` in it runs as a command; and a value that starts with
`-` is read as an option by the command it reaches. Input is passed as an
argument, quoted, after `--`, and a name that must be one component is refused
by a `case` when it is not.

```bash
sheet=$1
# Bad: a sheet named `x; rm -rf ~` runs both commands.
sh -c "gzip -k tiles/$sheet.png"
```

```bash
sheet=$1
case $sheet in
  '' | */* | .* | -*)
    echo "compress error: want a sheet's name, not a path" >&2
    exit 1
    ;;
esac
gzip -k -- "tiles/$sheet.png"
```

Held by `just check-shell`, whose ShellCheck refuses an unquoted expansion
(SC2086), and by review: ShellCheck passes `eval "…$sheet…"` and `sh -c
"…$sheet…"` both.

## A Recipe Passes Its Arguments as Words

just pastes `{{ sheet }}` into the recipe's line before its shell reads it, so
the argument is text of the script, and `x; rm -rf ~` runs both commands. That
matters where someone else chooses the argument: a CI step that passes a branch
name, a title or a file name from the repository, or a script that passes on
what it was given. The argument is passed as a word: `[positional-arguments]`
and `"$1"`, or `{{ quote(sheet) }}`.

```just
# Bad: compresses one tile sheet, pasting its name into the script.
compress sheet:
    gzip -k tiles/{{ sheet }}.png
```

```just
# Compresses one tile sheet.
[positional-arguments]
compress sheet:
    gzip -k -- "tiles/$1.png"
```

Held by review: no check lints a recipe's lines as a shell script.

## A Workflow Passes an Event's Text Through the Environment

GitHub pastes `${{ github.event.pull_request.title }}` into a `run:` step's
script before its shell reads it, so whoever opens a pull request writes part of
the script, and a title with `"; curl … | sh; "` in it runs in the job, with its
token and whatever secrets the job holds. An event's text goes to the step as a
variable of its environment, and the script reads the variable, quoted.

```yaml
- name: Name the tile set
  # Bad: the title is pasted into the script.
  run: echo "tile set ${{ github.event.pull_request.title }}"
```

```yaml
- name: Name the tile set
  env:
    TITLE: ${{ github.event.pull_request.title }}
  run: echo "tile set $TITLE"
```

Held by `just check-github-workflow-lint`, whose actionlint and zizmor each
report the bad step, and by review.
