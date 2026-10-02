# Messages

Read this before an error, log, panic or assertion message, a suppression's
reason, a CLI's help, a commit message, a pull request or a changelog entry. A
message is read away from the code that wrote it: by a user at a prompt, an
operator in a log, a developer at a failing test, a reviewer in a list of
commits. It has only its own words, so it carries the fact whole.

In Rust, `writing-rust` holds the form of an error type's message, when code may
panic, and how a binary fails, in its `references/errors.md`; its `#[expect]`
reasons are in its Lints rules.

`writing-rustdoc`'s `references/comments.md` holds how each Rust message is
worded, site by site.

A Rust test's assertion and `expect` messages follow `writing-rust-tests`.

## An Error Says What Was Wrong, with the Value and Where

The reader of an error has the message and nothing else: not the code, not the
input, often not the file. So it names what was refused, the value that broke
it, and where that value is, a file, a row, a key. "Invalid input" and
"something went wrong" are true of every failure, and so say nothing of this
one.

```text
Bad:  Error: Invalid input!
Bad:  error: failed to parse grid
Good: error: tiles.txt: row 3 has 7 squares, the first has 8
```

A script names itself first, writes to standard error, and exits nonzero, so a
caller can tell a failure from output.

```sh
# Bad: on standard output, with no name and no file.
echo "Error! File not found."
exit 1
```

```sh
echo "check-grids: $file: no such file" >&2
exit 1
```

Held by review.

## A Message Is a Lowercase Fragment, and Each Layer Adds Its Own Fact

A reporter prints each message of a chain after a prefix of its own, `error:`
before the first, so each message is a fragment that continues it: lowercase,
with no closing period. The Rust API Guidelines ask this of an error's
`Display`, and the GNU Coding Standards of a message after a program or file
name. Each layer of a chain adds only what it knows, the file around the row,
the row around the square, and never repeats the layer below: "failed to" at
every level says the same thing three times and the fact once.

```text
Bad:  Error: Failed to load board: Failed to parse grid: Failed to read row.
Good: error: board.txt: row 3 has 7 squares, the first has 8
```

Held by review.

The examples here belong to no one language. A Rust error type's message takes
the form of `writing-rust`'s `references/errors.md`, "A Message Is `<type words>
error: <fragment>`", which they leave out.

## Advice Only Where It Surely Helps

A message that apologizes, blames the user, exclaims or says "please" spends the
reader's attention on tone. Advice helps where there is one action that surely
fixes the failure, and then it names the action, on a line of its own where the
reporter has one; a guess, "check your input", "make sure the file exists", is
noise the reader has already thought of.

```text
Bad:  Oops! Something went wrong reading your grid. Please make sure the file
      is valid and try again.
Good: error: tiles.txt: row 3 holds `x`, not `.` or `#`
      help: a square is `.` when empty and `#` when it holds a tile
```

Held by review.

## A Message Is Read Where It Lands

What a reader sees is what reaches them, not what the code wrote: a runtime, a
reporter or an argument parser stands between, and may print a traceback, a
type's insides, a prefix of its own or lines of its own in place of the message.
Run each failure once, and read the words that land.

```text
Bad:  error: error: tiles.txt: row 3 has 7 squares, the first has 8
Good: error: tiles.txt: row 3 has 7 squares, the first has 8
```

The first is a message that carried its own `error:`, printed by a reporter that
adds another; only running it shows the two.

In Rust, a `main` that returns `Err` prints `Error: ` and the error's `Debug`,
not its message. For a command a person runs, that is the wrong line:

```text
Bad:  Error: GridError { row: 3, need: 8, have: 7 }
Good: error: grid error: row 3 has 7 squares, the first has 8
```

How a Rust binary reports a failure is `writing-rust`'s, in its
`references/errors.md`: "A Binary Fails with a Boxed Error" for one only an
operator reads, and "A Command a Person Runs Reports the Error's Message" for a
command a person runs.

Held by review: run the failure. A test that runs the command and compares its
standard error pins what lands.

## A Log Line Is a Stable Phrase, Its Values as Fields

A log is searched by its phrase and filtered by its fields, so a line of one
kind reads the same every time: a short lowercase phrase, and each value as a
field beside it, never spliced into the words. An error is logged once, where it
is handled, dropped or retried, not at every layer it passes. A secret, a token
or a user's data is never written, since a log is read by more people than the
code.

```text
Bad:  Saved 64 squares to /home/ana/boards/one.txt successfully!
Bad:  ERROR Failed to save board: Permission denied (token=…)
Good: board saved squares=64
Good: board save failed path=boards/one.txt error="permission denied"
```

Held by review.

## A Panic Names the Assumption That Broke

A panic fires only on a bug, and its reader is the one who fixes it, so its
message names the assumption that broke: the precondition a caller missed, or
the violation a check found. The fix then starts from the assumption, not from a
line number. "unreachable", "should not happen" and a bare `unwrap`'s message
name none.

```text
Bad:  thread 'main' panicked at src/draw.rs:40:22:
      called `Option::unwrap()` on a `None` value
Good: thread 'main' panicked at src/draw.rs:40:22:
      install a tile set before drawing the first board
```

Held by review.

Which of the two each Rust site names, an `expect` or a `debug_assert!`, is in
`writing-rustdoc`'s `references/comments.md`, under Messages.

## A Suppression's Reason Says Why the Check Misreads This Code

A suppression tells a reader that a tool was overruled here; its reason says why
the tool is wrong about this code, as one lowercase clause naming the fact the
tool cannot see. "Needed", "otherwise the linter complains" and the rule's own
name say only that it was overruled.

```text
Bad:  reason = "needed"
Bad:  reason = "otherwise clippy complains about the cast"
Good: reason = "a square's index is below the grid's length, checked above"
```

ShellCheck takes a reason after the directive, on the same line:

```sh
# Bad: nothing says why the words split.
# shellcheck disable=SC2086
printf '%s\n' $squares
```

```sh
# shellcheck disable=SC2086 # each square is its own argument
printf '%s\n' $squares
```

Held by review.

## An Assertion's Message States the Property

A failing assertion prints its two values and its message, so the message says
what should have held, as a lowercase fragment that continues the test's name:
"the ragged row is refused". "Should be equal", "wrong value" and "test failed"
say what every failure says. An assertion whose expression says it all may leave
the message out.

```text
Bad:  assertion failed: values should be equal
Good: assertion `left == right` failed: the ragged row is refused
```

Held by review.

## A CLI's Help Is Read at the Prompt

Help is read a line at a time, by someone about to type. A command's line is a
verb phrase in the mood and case its siblings and the parser's own lines use,
with no closing period; a flag's says what it sets, its unit and its default. An
example in the help runs as written.

```text
Bad:  size         This command allows you to get the size of a grid.
      --width <N>  Width
Good: size         Print the width and height of the grid a file holds
      --width <N>  Columns each square is drawn across [default: 1]
```

In Rust, clap's own line is "Print help", capitalized. With its derive, a
one-paragraph doc comment is the help, and its closing period is dropped where
the help is shown, so the doc comment keeps its period.

The comment above a recipe is its line in `just --list`, the justfile's help: it
says what the recipe does, in the form its neighbours use.

```text
Bad:  # checks stuff
      check-grids:
Good: # Checks that every grid under grids/ reads, and names each that does not.
      check-grids:
```

Held by review.

## A Commit Subject Says What Changes, for Whoever Reads the Log

A subject's form is AGENTS.md's Commits section, which every session reads and
`just check-git-commits` holds; what follows is what the form leaves to the
writer.

The subject says what the change does for its reader, the outcome, not the files
or the functions it touched: someone scanning the log, or a release's notes,
learns from it what changed without opening the diff.

```text
Bad:  feat: Add parse_grid function and GridError enum.
Bad:  Updated core and cli
Good: feat(core): read a grid from text, refusing ragged rows
```

Held by review, and the form by `just check-git-commits`.

## A Commit Body Says Why, in Prose

The diff says what changed; the body says why, and what holds once the change is
in, in sentences a reader follows: the problem, the choice made and the one not
made where a reviewer would weigh it, what the change leaves for later. A bullet
for each file retells the diff. A commit is about change, so here, and in a pull
request or a changelog, "now" and "no longer" are the right words.

```text
Bad:  This commit adds grid parsing.

      - Added parse_grid to lib.rs
      - Added GridError
      - Updated main.rs to use it
Good: A grid file with a short row drew squares from the next row into it, so
      the board shifted. `read` now refuses a row whose width is not the
      first's, with the row's number, and the cli prints it after the file
      name.
```

Held by review.

## A Pull Request Describes the Change for Its Reviewer

A reviewer reads the description before the diff, in this order: what the change
does, then why, how it was checked, and what it leaves out, in prose. "This PR",
a heading over every bullet, a wall of ticked boxes the template does not ask
for, and chat, "Here's the change you asked for!", are tells a reviewer skims
past, and with them the one fact they needed.

```text
Bad:  ## Summary
      This PR introduces comprehensive grid parsing! 🎉

      ## Changes
      - **Parser:** Added a robust parser.
      - **CLI:** Updated the CLI.
Good: `read` refuses a row whose width is not the first's, and the cli prints
      the file, the row and both widths. Without the check, the cli drew a
      ragged grid with squares pulled up from the next row. Checked with a test
      for each refusal, and by running the cli on a ragged file.
```

Held by review.

The repository's `.github/pull_request_template.md` lists what a description
covers.

## A Changelog Line Is Written for a User

The changelog is written from the commit subjects at a release, grouped by type,
so each subject is also a line a user reads there, with its first letter raised.

A wrong line is fixed by rewording its commit, never by editing the changelog.

A line says what changed for the user, in the user's words, not the
implementation's: "Refuse a grid with a ragged row, naming the row", not
"Refactor the parse loop". "Various improvements" and "bug fixes" tell a user
nothing.

```text
Bad:  - Various improvements and bug fixes
      - Refactor parse loop
Good: - Refuse a grid with a ragged row, naming the row
```

Held by review.
