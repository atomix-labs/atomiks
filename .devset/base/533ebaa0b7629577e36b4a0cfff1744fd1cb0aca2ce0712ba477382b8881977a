---
name: writing-the-book
description: Use when writing or restructuring a page of the project's book, adding a chapter, math, a diagram, a callout, an included listing or a link to the API documentation; when filling in a library's book before its first release, or judging whether the book is ready for one; or when `just check-mdbook` fails.
---

# Writing the Book

The book is the Markdown under the book's `src/` directory, which mdBook builds.
A page explains what the code does and why, for a reader who has not read it
yet; the API documentation holds the item-by-item detail.

## Rules

- **Every page is listed in `SUMMARY.md`**, which orders the book. A page it
  does not list is never published, and the docs lint fails on it. An entry
  whose page is missing is an error to fix, not a page to create empty.

- **Write only what the code and its tests show.** A behaviour the code does not
  have, or a promise it does not make, is not the book's to state: ask.

- **A library's book is whole before its first release**: after the introduction
  and Getting Started, a chapter for each concept a user must learn, then how to
  choose among its types, crates or features, performance where speed is a
  reason to use it, platforms and features, interop with the crates it converts
  to, testing code that uses it, and how it compares with the alternatives;
  `references/library-book.md` has the outline and what each chapter holds. A
  release whose book is still the scaffold's two pages is not ready, since the
  README's book badge sends every reader to it.

- **Callouts are mdBook's own**: a blockquote whose first line is `[!NOTE]`,
  `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]` or `[!CAUTION]`. Not a bold "Note:".

  ```markdown
  > [!WARNING]
  > Removing a tile renumbers every tile after it.
  ```

- **Math is TeX**, between `$` inline and `$$` for a display, rendered when the
  book builds. A literal dollar sign is `\$`.

- **Diagrams need the `mdbook` profile's `mermaid` feature.** Turn it on with
  devset, never by adding a preprocessor or a script to `book.toml` by hand:
  `devset add <source>/mdbook --features mermaid` where the profile comes with
  another, or the feature in its layer of `.devset/config.toml`.

- **Rust blocks are doctests**: `mdbook test` compiles and runs each. Mark one
  that must not run `no_run`, and what is not Rust `text`; never `ignore`.

- **Listings are included, not copied**, so they cannot drift from the code. Two
  comments in the source file mark the lines, and the page includes them by the
  name the comments give:

  ```text
  // ANCHOR: neighbours
  pub fn neighbours(&self, at: Point) -> impl Iterator<Item = Point> {
  // ANCHOR_END: neighbours
  ```

  ```markdown
  {{#include ../../crates/tiles/src/grid.rs:neighbours}}
  ```

- **A recipe a page names exists**: the docs lint fails on `just <recipe>` for
  one the justfile lacks.

- **An item of the API is linked by its path**, as in rustdoc: ``[`Grid`]`` for
  an item in scope, ``[`Grid`][tiles::grid::Grid]`` for any other. The book's
  build resolves it to the API at `api/`, and fails on a path that does not
  resolve, or that the name alone would reach.

## Steps

1. **Write the page** under `src/`, one file a page, its name in kebab case, its
   heading in title case.
2. **List it** in `SUMMARY.md`, where it belongs in the reading order.
3. **Preview it**: `mdbook serve` in the book's directory rebuilds on every save
   and serves the book at `http://localhost:3000`.
4. **Check it**: `just check-mdbook`, then `just check`.
5. **Before a library's first release**, hold `SUMMARY.md` to the outline in
   `references/library-book.md`: each chapter the library has is there, and none
   is a placeholder.

## Checks

`just check-mdbook` runs, in order:

- the docs lint: every page listed, every include and anchor there, every recipe
  a page names;
- the build, and every Rust block as a doctest;
- the API documentation, built into `api/`;
- every link of the built book, fragments included, offline.

No check sees whether the book holds what a reader needs: before a first
release, that is review's, against the outline.

## What Not to Do

- Do not add a preprocessor, a stylesheet or a script to `book.toml` by hand:
  turn on the feature that brings it.
- Do not create an empty page to satisfy `SUMMARY.md`.
- Do not call a library ready for its first release while its book is the
  scaffold's introduction and Getting Started.
- Do not draw a diagram as text art where mermaid is available.
- Do not copy code into a page that an include can show.
