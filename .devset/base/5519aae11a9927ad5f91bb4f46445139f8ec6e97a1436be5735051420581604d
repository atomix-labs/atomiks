# A Library's Book

Read this before a library's first release, and before adding a chapter to its
book. The scaffold's two pages, an introduction and a getting started page, are
where a book starts; a library's book is ready for a release when it holds the
chapters below, each with what the code and its tests show. A chapter the
library has nothing for is left out, never written empty: a crate with no reason
to be fast has no Performance chapter.

## The Outline

```markdown
# Summary

- [Introduction](introduction.md)
- [Getting Started](getting-started.md)
- [Boards](boards.md)
- [Squares and Moves](moves.md)
- [Choosing a Board](choosing.md)
- [Performance](performance.md)
- [Platforms and Features](platforms-and-features.md)
- [Interop](interop.md)
- [Testing](testing.md)
- [Compared with Others](comparison.md)
```

The concept chapters come after Getting Started, in the order a user meets the
concepts, and take the names the API gives them; the chapters after them take
these names.

## What Each Chapter Holds

| Chapter                | Holds                                                                                                                                                              |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Introduction           | what the library is, who it is for, the problem it solves and what it leaves out; a reader decides here whether to read on                                         |
| Getting Started        | the install line from crates.io, the minimum Rust, and the fewest steps to a first program that runs, with what it prints                                          |
| one for each concept   | what the concept is, why the library has it, the rules it keeps and the ones it refuses, a worked example included from tested code                                |
| Choosing               | where the library has several ways to one end, its types, crates or features, a table of which to take for which need, and why                                     |
| Performance            | what each operation a hot path takes costs, measured, with the run and the machine that showed it, and what to keep out of a hot path                              |
| Platforms and Features | the targets it builds and is tested on, `no_std` and allocation, the minimum Rust, and each Cargo feature, in the rows of the crate docs' `# Crate Features` table |
| Interop                | each crate it converts to and from, under which feature, what a conversion keeps and what it loses or refuses                                                      |
| Testing                | how a user tests code that uses the library: the doubles it ships, such as a clock set by hand, and how a test stays deterministic                                 |
| Compared with Others   | the crates a reader would weigh it against, at named versions, what each does that this one does not, and when to take another one instead                         |

## A Concept Chapter

A concept is something a user must hold in mind to use the API correctly: a type
and the rules between it and its neighbours, a timeline that two values must
share, an error the library refuses with. Its chapter answers, in order:

1. **What it is**, in one paragraph, linking its type in the API.
2. **Why the library has it**, the mistake it prevents or the cost it saves.
3. **What it keeps and what it refuses**, each refusal with the error a caller
   gets, since a reader learns the rules from the edges.
4. **A worked example**, included from a file the tests build, so it cannot
   drift from the code.

```markdown
# Squares and Moves

A [`Position`][tiles::Position] names one square of a board by its column and
row. A move from it is a [`Step`][tiles::Step], one of the eight directions, and
[`Board::step`][tiles::Board::step] refuses a step off the edge rather than
wrapping it, with
[`PositionError::ColumnOffBoard`][tiles::PositionError::ColumnOffBoard] or
[`PositionError::RowOffBoard`][tiles::PositionError::RowOffBoard].
```

## Compared with Others

A comparison states facts a reader can check, never a ranking: each other crate
at its version, what it has that this one lacks, and what this one has that it
lacks. It names when to take the other one, since a reader who takes the wrong
crate on the book's word stops trusting it. A claim of speed cites the run that
shows it, on the same machine for both.

| Crate           | Has that `tiles` has not          | Has not that `tiles` has     |
| --------------- | --------------------------------- | ---------------------------- |
| `ndarray` 0.16  | any number of dimensions, slicing | squares named by column, row |
| a `Vec<Vec<T>>` | nothing to learn                  | a width every row shares     |

## Before the First Release

- Every chapter of the outline the library has is in `SUMMARY.md`, and none is
  the scaffold's placeholder.
- The introduction and Getting Started say what the README says, and the install
  line is the one from crates.io.
- Every example is included from code the tests build, and `just check-mdbook`
  passes.
