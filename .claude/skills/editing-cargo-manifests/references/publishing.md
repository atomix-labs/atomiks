# A Published Crate's Metadata

Read this before writing or reviewing the `[package]` of a crate crates.io
publishes, which is every crate without `publish = false`, and before its first
release. crates.io shows a crate's page from its manifest and its package alone,
as they were when that version was published, and a version never changes: a
field left out at 0.1.0 is missing from that page for good. Each section below
is one field, its rule, its reason, and what holds it.

Contents: 1 Where They Go · 2 `homepage` · 3 `readme` · 4 `keywords` · 5
`categories` · 6 Licence Files · 7 `exclude`

## 1 Where They Go

A published crate's `[package]` writes them after `repository`, in this order:
`homepage`, `readme`, `keywords`, `categories`, then `exclude`. A field every
crate shares lives in `[workspace.package]` and is inherited, `<key>.workspace =
true`, as `version` is; one that differs by crate is the crate's own.

```toml
[package]
name                   = "tiles-paint"
description            = "brushes that paint tiles onto a board, one square at a time."
version.workspace      = true
edition.workspace      = true
rust-version.workspace = true
license.workspace      = true
authors.workspace      = true
repository.workspace   = true
homepage.workspace     = true
keywords               = ["tile", "board", "brush", "paint"]
categories             = ["game-development", "graphics"]
exclude                = ["benches/results/"]
```

## 2 `homepage`

Where the project has a book, `homepage` is its URL, set once in
`[workspace.package]` and inherited by every crate: crates.io links the
repository from `repository` and the API from docs.rs by itself, and the book
from nothing else. `documentation` stays unset, as the body's Shape rules say,
since setting it replaces crates.io's link to docs.rs.

```toml
[workspace.package]
homepage = "https://<owner>.github.io/<repo>/"
```

A project without a book leaves `homepage` out rather than repeat `repository`.

## 3 `readme`

crates.io shows the README the package carries, and a crate under `crates/` has
none unless one is there:

- **The one crate the root README is for writes `readme = "../../README.md"`**,
  and Cargo copies the root README into its package. It is the crate the root
  README's first install line names, the facade or the library.
- **Every other crate has a `README.md` beside its `Cargo.toml`**, which Cargo
  finds with no `readme` key. Writing `readme = "README.md"` restates the
  default.
- **Every link and image in either is absolute**, since crates.io shows the
  README away from the repository: it never resolves a relative `srcset`, and it
  resolves a root README's relative links against the crate's own directory.

## 4 `keywords`

Up to five words a reader would search crates.io for, beyond the crate's name,
which the search matches already: what it is, `timestamp`, and what it is for,
`no-std`. crates.io refuses a sixth, and a keyword over 20 characters or with a
character other than an ASCII letter, a digit, `_`, `-` or `+`, the first a
letter or a digit; it stores each in lower case.

```toml
# Bad: the crate's name, a phrase, and two words crates.io refuses.
keywords = ["tiles", "a board of tiles", "board-game-engine-library", "c++"]
```

```toml
keywords = ["board", "tile", "grid", "game"]
```

## 5 `categories`

Up to five of crates.io's own slugs, from
[its list](https://crates.io/category_slugs), the most specific that fits: a
`no_std` crate adds `no-std`, and `no-std::no-alloc` where it never allocates. A
slug crates.io does not have is dropped with a warning at publish, so the crate
is listed under fewer categories than the manifest says.

```toml
# Bad: no slug of crates.io's, and so no category at all.
categories = ["games", "boards"]
```

```toml
categories = ["game-development", "data-structures", "no-std"]
```

## 6 Licence Files

Each licence `license` names asks its text to travel with every copy, and the
package is the copy crates.io serves. Cargo packages only what is under the
crate's directory, so a crate in `crates/<name>/` links each licence file from
the root, and Cargo packages the link as the file:

```sh
ln -s ../../LICENSE-MIT crates/tiles-paint/LICENSE-MIT
ln -s ../../LICENSE-APACHE crates/tiles-paint/LICENSE-APACHE
```

`MIT OR Apache-2.0` names `LICENSE-MIT` and `LICENSE-APACHE`; a single licence,
`LICENSE`. `license-file` is for a licence SPDX has no name for, never beside
`license`.

## 7 `exclude`

A user's download is the whole package, so what only the repository needs stays
out of it: committed benchmark transcripts under `benches/results/`, fixtures
too large for a test a user runs, recordings, design notes. `exclude` names
them, as paths inside the crate; `include` is for a crate that would rather list
what goes in.

```toml
exclude = ["benches/results/", "tests/fixtures/large/"]
```

A path a target needs stays in: a bench whose source is excluded no longer
builds from the package. Check what remains:

```sh
cargo package --list -p tiles-paint
```

Held by review.
