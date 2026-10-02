# Templates: Library, Command Line, Workspace, Crate

Read this before writing a README or any section of one. Each template below is
a README whole, in the order the body's rules give: take the one for what the
repository builds, keep its order, fill each section from the code and a run,
and drop a section with nothing to say. `<owner>` is the repository's owner on
GitHub.

Every link to a file of the repository is a relative path, since only GitHub
shows the README.

Contents: 1 Library · 2 Command Line · 3 Workspace · 4 Each Crate's Own README

## 1 Library

The header comes first, as the body's rules keep it; the pitch picks up where
the tagline, "Boards of tiles, and the moves across them.", stops; the example
is the crate page's first doctest, written out as a program.

````md
<!-- >>> devset: project >>> -->

(the header block, as the project profile renders it)

<!-- <<< devset: project <<< -->

A board of any size, each square named by its column and row, and a move off the
edge refused rather than wrapped. It draws nothing: a board is data, for a game
or a terminal to show.

## Install

```sh
cargo add tiles
```

`tiles` builds with Rust 1.98 or later.

## Quick Start

```rust
use tiles::{Board, Pos, PosError};

fn main() -> Result<(), PosError> {
    let board = Board { cols: 8, rows: 8 };
    assert_eq!(board.pos(3, 1)?, Pos { col: 3, row: 1 }, "a square of the board");
    assert_eq!(board.pos(8, 1), Err(PosError::Col { col: 8, cols: 8 }), "and not past it");
    Ok(())
}
```

## Features

| Feature | Adds                                                |
| ------- | --------------------------------------------------- |
| `serde` | `Serialize` and `Deserialize` for `Board` and `Pos` |

None is on by default: `cargo add tiles --features serde`.

## Documentation

- [The API on docs.rs][docs.rs], with an example for each type.
- [CHANGELOG.md][changelog]: what changed in each release.

## Contributing

Issues and pull requests are welcome: read [CONTRIBUTING.md][contributing]
first.

## License

Either [the MIT License][mit] or [the Apache License, Version 2.0][apache], at
your option.

[docs.rs]: https://docs.rs/tiles
[changelog]: CHANGELOG.md
[contributing]: CONTRIBUTING.md
[mit]: LICENSE-MIT
[apache]: LICENSE-APACHE
````

The minimum Rust is the workspace's `rust-version`, 1.98 where it is the
default; the feature table has the rows of the crate docs' `# Crate features`,
and a crate with no features has no such section.

Where the repository has a book, Documentation links it first, "The book: what
it teaches", to `https://<owner>.github.io/tiles/`, the site the header's book
badge links.

## 2 Command Line

The pitch says what the command does to what, and what it never does; Install
names the package, which differs from the binary; Quick Start is one command and
the output a run printed.

````md
<!-- >>> devset: project >>> -->

(the header block, as the project profile renders it)

<!-- <<< devset: project <<< -->

`tiles` shows a board of tiles in the terminal, and moves a tile across it. It
reads the board from a text file, one line a row, and writes the moved board to
standard output: it never changes the file.

## Install

```sh
cargo install --locked tiles-cli
```

The package is `tiles-cli`, and the binary `tiles`; `cargo install` builds it
with Rust 1.98 or later.

## Quick Start

```sh
tiles show board.txt
```

```text
  a b c d
1 . # . .
2 . . . #
```

## Usage

| Command                         | Does                                          |
| ------------------------------- | --------------------------------------------- |
| `tiles show <file>`             | draws the board                               |
| `tiles move <file> <from> <to>` | moves a tile, refusing a square off the board |

`tiles --help` lists every command and flag.

## Documentation

- [CHANGELOG.md][changelog]: what changed in each release.

## Contributing

Issues and pull requests are welcome: read [CONTRIBUTING.md][contributing]
first.

## License

Either [the MIT License][mit] or [the Apache License, Version 2.0][apache], at
your option.

[changelog]: CHANGELOG.md
[contributing]: CONTRIBUTING.md
[mit]: LICENSE-MIT
[apache]: LICENSE-APACHE
````

The output is pasted from a run of the build the README describes, never typed;
the table names the commands a reader reaches for first, and the help has the
rest.

## 3 Workspace

A library and the command line built on it: the root README is the project's
page, and names each crate; each crate's install line is its own.

````md
<!-- >>> devset: project >>> -->

(the header block, as the project profile renders it)

<!-- <<< devset: project <<< -->

A library and a command line: the library holds the board and refuses a move off
its edge; the command line reads a board from a file and shows it.

| Crate       | What it is                                                    |
| ----------- | ------------------------------------------------------------- |
| `tiles`     | the library: boards, squares and moves, [on docs.rs][docs.rs] |
| `tiles-cli` | the `tiles` command, which shows a board and moves its tiles  |

## Install

The library:

```sh
cargo add tiles
```

The command line, whose package is `tiles-cli` and whose binary is `tiles`:

```sh
cargo install --locked tiles-cli
```

Both build with Rust 1.98 or later.

## Quick Start

(the library's example, then the command line's, as in §1 and §2)

## Usage

(the command line's table, as in §2)

## Documentation

- [The library's API on docs.rs][docs.rs], with an example for each type.
- [CHANGELOG.md][changelog]: what changed in each release.

## Contributing

(as in §1)

## License

(as in §1)

[docs.rs]: https://docs.rs/tiles
[changelog]: CHANGELOG.md
````

The pitch is the project's, and the table says which crate is which, since a
reader looking for the library would otherwise install the command line. Only
the root README has the header.

## 4 Each Crate's Own README

One crate shows the root README, through `readme = "../../README.md"`: the one
the root README's first install line is for, `tiles`, the library, by default.
Every other published crate has a `README.md` beside its `Cargo.toml`, which
Cargo finds with no `readme` key: its name as the title, the crate's place in
the project in one paragraph, its install line and first use, its docs, and the
licence.

That crate is the one the `crate` variable names, whose crates.io and docs.rs
badges the header carries.

````md
# `tiles-cli`

The command line of [tiles][project]: the `tiles` command shows a board of tiles
in the terminal, and moves a tile across it. It is built on the `tiles` library.

## Install

```sh
cargo install --locked tiles-cli
```

The binary is `tiles`, and it builds with Rust 1.98 or later.

(the Quick Start of §2)

`tiles --help` lists every command and flag.

## License

Either [the MIT License][mit] or [the Apache License, Version 2.0][apache], at
your option.

[project]: ../../README.md
[mit]: ../../LICENSE-MIT
[apache]: ../../LICENSE-APACHE
````

A crate's README has no header and no badges: the root README carries them.
