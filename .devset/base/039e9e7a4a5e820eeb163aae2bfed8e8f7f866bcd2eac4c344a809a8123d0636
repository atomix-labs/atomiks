# Properties, Tables and Fuzzing

Read this before a test that should hold for every input, a round trip, a parser
that must never panic, before a table of cases, and before a fuzz target or a
crash it found. Each widens a test past the inputs its author thought of: a
property generates them, a table lists them, a fuzzer searches for them.

## An Invariant Over Every Input Is a Property

What holds for every value, not for a few, is a proptest property: a round trip
through printing and parsing, a parser that refuses what it cannot read and
never panics, a fast function that agrees with a slow and obvious one. proptest
runs it on 256 generated inputs by default, and on a failure shrinks the input
to the smallest that still fails. A loop over hand-picked values tests the cases
the author already believed in.

```rust
use core::str::FromStr;

use derive_more::Display;
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("parse pos error: want `<col>,<row>`, like `3,4`")]
pub struct ParsePosError;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
#[display("{col},{row}")]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

impl FromStr for Pos {
    type Err = ParsePosError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (col, row) = text.split_once(',').ok_or(ParsePosError)?;
        let col = col.parse().map_err(|_not_digits| ParsePosError)?;
        let row = row.parse().map_err(|_not_digits| ParsePosError)?;
        Ok(Self { col, row })
    }
}

#[cfg(test)]
mod tests {
    use super::Pos;

    // Bad: the two positions the author was sure of.
    #[test]
    fn a_pos_reads_back_as_it_prints() {
        for pos in [Pos { col: 0, row: 0 }, Pos { col: 3, row: 4 }] {
            assert_eq!(pos.to_string().parse::<Pos>(), Ok(pos), "one spelling, both ways");
        }
    }
}
```

```rust
use core::str::FromStr;

use derive_more::Display;
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("parse pos error: want `<col>,<row>`, like `3,4`")]
pub struct ParsePosError;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
#[display("{col},{row}")]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

impl FromStr for Pos {
    type Err = ParsePosError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (col, row) = text.split_once(',').ok_or(ParsePosError)?;
        let col = col.parse().map_err(|_not_digits| ParsePosError)?;
        let row = row.parse().map_err(|_not_digits| ParsePosError)?;
        Ok(Self { col, row })
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{any, prop_assert_eq, proptest};

    use super::Pos;

    proptest! {
        #[test]
        fn a_pos_reads_back_as_it_prints(col in any::<u16>(), row in any::<u16>()) {
            let pos = Pos { col, row };
            prop_assert_eq!(pos.to_string().parse::<Pos>(), Ok(pos), "one spelling, both ways");
        }

        #[test]
        fn any_text_parses_or_is_refused(text in ".*") {
            let _parsed = text.parse::<Pos>();
        }
    }
}
```

Held by review. proptest is a `[dev-dependencies]` entry; `PROPTEST_CASES`
raises the count for a run by hand.

## A Property Generates Valid Inputs, and Discards Few

A property over squares on an 8 by 8 board generates columns and rows below 8,
from ranges or a strategy's `prop_map`. `prop_assume!` is for the sliver of
inputs a strategy cannot rule out cheaply, never for most of them: one that
generates any `u16` and throws away what is off the board rejects nearly every
input, and proptest gives up after 1,024 rejections, failing a property that
holds.

```rust
#[must_use]
pub fn index(col: u16, row: u16) -> Option<usize> {
    if col >= 8 || row >= 8 {
        return None;
    }
    usize::from(row).checked_mul(8)?.checked_add(usize::from(col))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{any, prop_assert, prop_assume, proptest};

    use super::index;

    proptest! {
        // Bad: keeps one input in 67 million, and fails with "Too many global rejects".
        #[test]
        fn every_square_on_the_board_has_an_index(col in any::<u16>(), row in any::<u16>()) {
            prop_assume!(col < 8 && row < 8);
            prop_assert!(index(col, row).is_some_and(|at| at < 64), "an index below 64");
        }
    }
}
```

```rust
#[must_use]
pub fn index(col: u16, row: u16) -> Option<usize> {
    if col >= 8 || row >= 8 {
        return None;
    }
    usize::from(row).checked_mul(8)?.checked_add(usize::from(col))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{prop_assert, proptest};

    use super::index;

    proptest! {
        #[test]
        fn every_square_on_the_board_has_an_index(col in 0..8_u16, row in 0..8_u16) {
            prop_assert!(index(col, row).is_some_and(|at| at < 64), "an index below 64");
        }
    }
}
```

Held by review, and by proptest, which fails the property once it has rejected
too many.

## A Property Asserts with `prop_assert`

Inside `proptest!`, `prop_assert!` and `prop_assert_eq!` return the failure
rather than panic, so proptest shrinks the input quietly and reports the
smallest one once, with its message and the line. An `assert!` there shrinks
too, but prints a panic for each step it takes, and the smallest input is lost
among them.

```rust
#[must_use]
pub fn mirror(col: u16, cols: u16) -> Option<u16> {
    cols.checked_sub(1)?.checked_sub(col)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::proptest;

    use super::mirror;

    proptest! {
        // Bad: a failure prints a panic for each shrinking step.
        #[test]
        fn a_mirrored_column_mirrors_back(col in 0..8_u16) {
            let there = mirror(col, 8).expect("a column on the board has a mirror");
            assert_eq!(mirror(there, 8), Some(col), "and back");
        }
    }
}
```

```rust
#[must_use]
pub fn mirror(col: u16, cols: u16) -> Option<u16> {
    cols.checked_sub(1)?.checked_sub(col)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{prop_assert_eq, proptest};

    use super::mirror;

    proptest! {
        #[test]
        fn a_mirrored_column_mirrors_back(col in 0..8_u16) {
            let there = mirror(col, 8).expect("a column on the board has a mirror");
            prop_assert_eq!(mirror(there, 8), Some(col), "and back");
        }
    }
}
```

Held by review.

## A Property's Failures Are Committed

When a property fails, proptest writes the failing case's seed to
`proptest-regressions/`, in a file named for the source file, `src/pos.rs`'s in
`proptest-regressions/pos.txt`, and runs those seeds first on every run after.
The file is committed, so the case that failed once runs first everywhere, in CI
included, and the fix stays proven.

```text
# Bad: in .gitignore, so each run starts afresh, and a failure found once may not be found again.
proptest-regressions/
```

```text
# Seeds for failure cases proptest has generated in the past. It is
# automatically read and these particular cases re-run before any
# novel cases are generated.
#
# It is recommended to check this file in to source control so that
# everyone who runs the test benefits from these saved cases.
cc 6dfaaacf71ba74d7b4b2d2368e56a69e2194f0a4a3e99ab10cd4a8914b6ace51 # shrinks to col = 1000
```

Held by review.

## A Table of Cases Is an `rstest`, Each Case Named

A list of inputs and what each gives back, every way a parser refuses, is a
table: `#[rstest]` with a `#[case]` for each row, each a test of its own, so
every wrong row fails, not the first, and each is named in the run,
`a_pos_parses_as_its_table_says::case_2_a_column_past_u16`. A case whose input
does not say what it is for is named, `#[case::a_column_past_u16(…)]`. A loop
over the rows in one test stops at the first that fails.

```rust
use core::str::FromStr;

use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("parse pos error: want `<col>,<row>`, like `3,4`")]
pub struct ParsePosError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

impl FromStr for Pos {
    type Err = ParsePosError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (col, row) = text.split_once(',').ok_or(ParsePosError)?;
        let col = col.parse().map_err(|_not_digits| ParsePosError)?;
        let row = row.parse().map_err(|_not_digits| ParsePosError)?;
        Ok(Self { col, row })
    }
}

#[cfg(test)]
mod tests {
    use super::{ParsePosError, Pos};

    // Bad: the first wrong row hides every row after it.
    #[test]
    fn a_pos_parses_as_its_table_says() {
        let table = [
            ("3", Err(ParsePosError)),
            ("65536,0", Err(ParsePosError)),
            ("0,0", Ok(Pos { col: 0, row: 0 })),
        ];
        for (text, want) in table {
            assert_eq!(text.parse::<Pos>(), want, "the table's answer for {text:?}");
        }
    }
}
```

```rust
use core::str::FromStr;

use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("parse pos error: want `<col>,<row>`, like `3,4`")]
pub struct ParsePosError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

impl FromStr for Pos {
    type Err = ParsePosError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (col, row) = text.split_once(',').ok_or(ParsePosError)?;
        let col = col.parse().map_err(|_not_digits| ParsePosError)?;
        let row = row.parse().map_err(|_not_digits| ParsePosError)?;
        Ok(Self { col, row })
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::{ParsePosError, Pos};

    #[rstest]
    #[case::no_comma("3", Err(ParsePosError))]
    #[case::a_column_past_u16("65536,0", Err(ParsePosError))]
    #[case::the_first_square("0,0", Ok(Pos { col: 0, row: 0 }))]
    fn a_pos_parses_as_its_table_says(#[case] text: &str, #[case] want: Result<Pos, ParsePosError>) {
        assert_eq!(text.parse::<Pos>(), want, "the table's answer for {text:?}");
    }
}
```

Held by review. rstest is a `[dev-dependencies]` entry.

## Input from Outside Is Fuzzed, in a Workspace of Its Own

A parser or decoder of what another program or a person wrote is fuzzed:
cargo-fuzz runs a target on inputs libFuzzer mutates towards new paths, under a
sanitizer, for as long as it is given, and finds the input a property's strategy
never generates. The targets live in the crate's `fuzz/`, whose manifest holds
an empty `[workspace]` table, so it is a workspace of its own: cargo-fuzz builds
it with flags and a profile of its own, `fuzz_target!` expands to code the lint
table refuses, and no `--workspace` recipe reaches it. A target over raw bytes
checks that nothing panics; one over an `Arbitrary` type spends its time on the
grammar.

```text
# Bad: fuzz/Cargo.toml with no [workspace] table, so Cargo refuses to build it:
# "current package believes it's in a workspace when it's not".
[package]
name    = "tiles-fuzz"
publish = false
```

```text
# crates/tiles/fuzz/Cargo.toml
[package]
name    = "tiles-fuzz"
version = "0.0.0"
edition = "2024"
publish = false

[package.metadata]
cargo-fuzz = true

[workspace]

[dependencies]
libfuzzer-sys = "0.4"
tiles         = { path = ".." }

[[bin]]
name  = "parse_pos"
path  = "fuzz_targets/parse_pos.rs"
test  = false
doc   = false
bench = false

// crates/tiles/fuzz/fuzz_targets/parse_pos.rs
//! `Pos::from_str` over any UTF-8, which it refuses or reads and never panics on.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = core::str::from_utf8(data) {
        let _parsed = text.parse::<tiles::Pos>();
    }
});
```

Held by review. `cargo fuzz run parse_pos` fuzzes, on a nightly toolchain, since
it needs nightly's sanitizer flags; no recipe runs it, and no profile pins
cargo-fuzz, which `cargo install cargo-fuzz` installs.

## A Fuzz Target's Corpus Is Committed and Replayed

A target's corpus, `fuzz/corpus/<target>/`, is the inputs it has found that
reach new code, and a fresh checkout that starts from it fuzzes from real
coverage. `cargo fuzz init` writes a `fuzz/.gitignore` that ignores it, so the
line is taken out and the seeds committed, kept small with `cargo fuzz cmin`.
`cargo fuzz run <target> -- -runs=0` replays every seed and fuzzes nothing,
which is how a change is checked against them.

```text
# Bad: fuzz/.gitignore as `cargo fuzz init` writes it, the corpus included.
target
corpus
artifacts
coverage
```

```text
# fuzz/.gitignore
target
artifacts
coverage

# Replays the seeds, once each.
cargo fuzz run parse_pos -- -runs=0
```

Held by review.

## A Crash the Fuzzer Finds Becomes a Unit Test

A crash lands in `fuzz/artifacts/<target>/crash-<hash>`, which is ignored, and
cargo-fuzz prints the commands that reproduce and shrink it: `cargo fuzz run
<target> <file>`, then `cargo fuzz tmin <target> <file>`. The smallest input
becomes a unit test in the crate, named for what should have held, so the fix is
pinned in every check, and not only when someone fuzzes again.

```text
# Bad: the crash reproduced and fixed, and nothing pins it.
cargo fuzz run parse_pos fuzz/artifacts/parse_pos/crash-11f6ad8e…
```

```rust
use core::str::FromStr;

use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("parse pos error: want `<col>,<row>`, like `3,4`")]
pub struct ParsePosError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

impl FromStr for Pos {
    type Err = ParsePosError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (col, row) = text.split_once(',').ok_or(ParsePosError)?;
        let col = col.parse().map_err(|_not_digits| ParsePosError)?;
        let row = row.parse().map_err(|_not_digits| ParsePosError)?;
        Ok(Self { col, row })
    }
}

#[cfg(test)]
mod tests {
    use super::{ParsePosError, Pos};

    // Found by the `parse_pos` fuzz target, shrunk to a comma and no row.
    #[test]
    fn a_pos_with_no_row_is_refused() {
        assert_eq!("3,".parse::<Pos>(), Err(ParsePosError), "a column alone");
    }
}
```

Held by review.
