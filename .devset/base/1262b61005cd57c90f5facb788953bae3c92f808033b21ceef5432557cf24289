# Templates: Crate, Module, Item, File, Diagram, Table

Read this before a crate page, a module doc, an item's docs, an error type, the
`//!` of a test, bench or example file, a manifest's `description`, a diagram or
a table. Skeletons in the exemplar crates' shape. *Fixed* sections keep their
exact name across crates so a reader learns where to look once; *task* and
*concept* sections are named for what the reader does or the thing explained.
Drop any section with nothing to say; never pad one.

Contents: 1 Crate · 2 Module · 3 Items · 4 Errors · 5 Test, bench, example files
· 6 Manifest · 7 Diagrams · 8 Tables and bullets · 9 Section names

## 1 Crate (`lib.rs`)

Two shapes are in use. Both open with the pitch and close with the reference
links; both keep `lib.rs` to docs, `#![…]` attributes, `mod`, and `pub use`.

### 1.1 Problem-First (A Facade or a Mechanism with a Story)

````text
//! <Topic>: <pitch, the manifest `description`>.
//!
//! <The problem, shown before it is named: a short example with one-line comments naming the
//! cost, then one paragraph on what goes wrong and the external fact that says so, cited.>
//!
//! <The shape: what the crate is, then each departure from the obvious design or from a wrapped
//! crate, as fact + consequence, one sentence each.>
//!
//! # Examples                                        ← fixed, as an item's
//! ## <Task, e.g. Building One Value>                ← task, gerund
//! ```
//! <complete example of the primary path>
//! ```
//!
//! ## <Task, e.g. Building Many>
//! <One sentence naming and linking the entry points.>
//! ```
//! <example>
//! ```
//!
//! # <Concept, e.g. Pinned Types / Contention / Ordering>   ← concept, noun
//! <Prose; a diagram (§7) if there is a flow, handoff, or state.>
//!
//! # What It Compiles To                             ← fixed, only with real output in hand
//! <The exact call, the build (`release build, aarch64`), a `text` fence of the listing with
//! aligned `;` annotations, one paragraph on what it proves.>
//!
//! # Crate Features                                  ← fixed, required if any feature exists
//! <One sentence on the defaults.>
//!
//! | Feature   | Adds                                                       |
//! | --------- | ---------------------------------------------------------- |
//! | `alloc`   | `InPlaceInit`, to build a `Box`/`Arc` in place             |
//!
//! [label]: https://…                                ← reference links, order of first use
````

### 1.2 Shape-First (A Vocabulary Crate: Several Types That Fit Together)

````text
//! <Pitch.>
//!
//! <One paragraph: the central noun and what choosing it buys. A `text` shape diagram (§7.1) if
//! the parts relate.>
//!
//! # Types                                           ← fixed
//!
//! - **<Role.>** [`A`] is …; [`b`](A::b) narrows one, [`c`](A::c) divides one, and both spend
//!   what they take.
//! - **<Role.>** [`D`] is …; [`E`] is what a `b` yields.
//! - **Refusals.** [`AError`] when …; [`BError`] when ….
//!
//! # Examples
//! ```
//! <one example of the primary path>
//! ```
````

`# Types` groups by *role*, three to five bullets, bold lead, each naming its
types in one clause with a link. It is never one bullet per export, and never
restates an item's own docs.

Every example a crate page shows sits under `# Examples`, as an item's does, so
a reader finds them where every page keeps them: a page of one task has its
example straight under the heading, and a page of several gives each a `##`
named for the task.

### 1.3 Attributes That Follow

```text
#![no_std]
#![expect(
    unsafe_code,
    reason = "<the concrete unsafe this crate exists to do>"
)]
```

The crate-level `#![expect(unsafe_code)]` is only for a crate whose whole
purpose is unsafe; elsewhere it sits on the statement, item or `impl` that holds
the unsafe. A crate that needs a nightly feature of its own, an unstable API,
enables it here, with a `//` line saying why; no file writes one for a lint.

`just check-rust-lints` turns on the features nightly's lints need through
`-Zcrate-attr`, so no template here opens with a `#![feature]`.

Ordering rule for the crate page: *why → what → how to use it → what it costs →
how to configure it*. Task sections go from the single, common case to the
plural or advanced one.

## 2 Module (`mod.rs` / `foo.rs`)

The default is one line:

```text
//! <Noun or gerund phrase, naming the key item>.
```

`//! A bump cursor over one region.` · `//! Why an allocator refused.` · `//!
Driving an initializer at a destination.` · ``//! `Arena`'s [`Allocator`] impl:
it carves, and never takes a cut back.``

A module that is itself a surface adds a contract paragraph and one example:

````text
//! <Gerund phrase>: <what it gives, naming the key item>.
//!
//! <Contract: what is guaranteed, in what order, what remains on failure; the general entry
//! point against the specialized ones, all linked.>
//!
//! ```
//! <one example of the primary path>
//! ```
````

A module whose content is one type takes the type's role as its line and leaves
the type's docs primary. Never a `///` on the `mod foo;` declaration in
`lib.rs`; the file's `//!` is the one home.

## 3 Items

Order inside an item: summary · prose · `# Safety` · `# Errors` · `# Panics` ·
`# Examples`. Blank `///` line before each heading, none after. `# Examples`
always plural.

### 3.1 Function or Method

````text
/// <Verb> <object> <at/into/from> <target>[, or `None` when <condition>].
///
/// <Why or when; the reason for a non-obvious choice; the cost.>
///
/// # Errors
/// <see §4>
///
/// # Examples
/// ```
/// <shortest example that shows the why>
/// ```
#[inline]                                     ← a small public fn that calls another
#[must_use]                                   ← a pure fn; never one returning a `Result`
pub fn …
````

Getters and constructors are one line with no example: `/// Bytes the span
holds.`, ``/// The range of `length` bytes at `start`.``, `/// Wraps a raw
address.`

### 3.2 Unsafe Function

````text
/// <Verb> ….
///
/// # Safety
/// `destination` is <precondition>, <precondition>, and <precondition>[; and, unless <case>,
/// <conditional precondition>].
///
/// # Errors
/// <…>
///
/// # Examples
/// ```
/// // SAFETY: <for each precondition, the fact that discharges it>.
/// unsafe { … }
/// ```
pub unsafe fn …
````

The infallible twin of a fallible fn writes `# Safety` as ``As
[`try_version`].`` and drops `# Errors`. A method users should not call directly
says so in the summary and links the front door: ``/// Writes them at `dst`.
Prefer [`raw_run_init`] / [`raw_try_run_init`] to calling this.``

### 3.3 Trait

```text
/// How to <capability> [at/for <context>].            ← or the role, for a shape trait
///
/// <Why a trait; what implementors promise beyond the signatures; the one reason for its shape.>
///
/// # Safety                                           ← unsafe trait: the impl's promise
/// `Ok` means <state>; `Err` means <state>, and <what may then be done with the bytes>.
pub unsafe trait Name<T> {
    /// <A stored location: a pointer for [`Local`], an [`Offset`] for [`Shared`].>   ← assoc type
    type Stored: Copy;

    /// <What it fixes, and why that value.>            ← assoc const
    const MIN_REGION: Layout;

    /// <How many … / Whether … / The …>                ← getters
    fn len(&self) -> usize;

    /// <Verb> ….
    ///
    /// # Errors
    /// [`BindError`], <condition>, <condition>, or <condition>.
    fn open(region: &Region<Self::Storage>) -> Result<Self::Bound<'_>, BindError>;
}
```

A marker trait states the property an impl asserts and, for `unsafe trait`, what
that licenses: `/// Bytes returned to this allocator become available again.`
then `# Safety` then, if needed, the one place the marker may gate and where it
must not.

A trait method whose behaviour in one impl needs saying is documented *in that
impl*: `/// Bytes the free-lists do not hold, **the heap's own terminal sentinel
included**: …`.

### 3.4 Struct and Fields

````text
/// <Role: noun phrase, or verb phrase for an active object>[: <the pitch clause>].
///
/// - **<Property.>** <Consequence, one sentence.>        ← three bullets, or prose for fewer
/// - **<Property.>** <…>
/// - <A property with no name, as prose.>
///
/// <Where the safe door is, linked.>
///
/// # Examples
/// ```
/// <…>
/// ```
pub struct Name<S> {
    // INVARIANT: <what every writer keeps>; <its writers>.   ← a field unsafe code relies on
    /// <Noun phrase>.
    raw: NonNull<T>,
    /// <Noun phrase>.[ <Invariant.>]
    field: T,
    /// Fixes <the type parameters> by owning them.      ← PhantomData
    marker: PhantomData<S>,
}
````

Field docs are one line. An `// INVARIANT:` sits above the `///` of each field a
`// SAFETY:` relies on (`comments.md` §2). Adapter and return types: ``/// What
[`builder`] builds.`` and nothing more unless users construct them.

Under `strict`, private fields are documented too, since
`missing_docs_in_private_items` is denied.

### 3.5 Enum and Variants

```text
/// <Role>.
///
/// <If variants form a state machine or an ordering: the rule, or a diagram (§7.4).>
pub enum Name {
    /// <When it is produced, or what it carries.>
    Variant,
}
```

### 3.6 Type Alias and Constant

```text
/// <The role>.
///
/// An **alias**, not a newtype: <the fact that decides it>.
pub type Offset = NonZeroU64;

/// <What it fixes>, and <why that value>.
pub const MIN_ALIGN: usize = 16;
```

### 3.7 Macro

````text
/// <Verb> ….
///
/// <Grammar in one sentence: `name: value` writes a value, `name <- init` runs a nested
/// initializer, `name` takes the like-named field.> <Where the escape hatch is, linked.>
///
/// <A known wart, as fact + consequence, one sentence.>
///
/// ```
/// <…>
/// ```
#[macro_export]
macro_rules! name { … }
````

### 3.8 `#[doc(hidden)]`, Re-Exports, Deliberate Absences

```text
#[doc(hidden)]
pub use ::pin_init as __pin_init;       // no doc; the `__` says "not yours"

/// What this crate takes from `pin-init` unchanged, globbed so an explicit import can shadow one
/// of its names.
mod borrowed {
    // `Zeroable`, `MaybeZeroable`, … are deliberately absent: <reason as fact + consequence>.
    pub use pin_init::{…};
}

// No `Reclaiming`: a bump cursor never steps back, so a block returned here is not served again.
```

A hidden module that a dependent's tests use gets a `//!` saying why it is
public and hidden: ``//! Behind the `testing` feature: a plain `cfg(test)`
module is invisible across a crate boundary, and a dependent's tests count drops
the same way.``

## 4 Errors

`# Errors` names the variant, links it, and states the condition as a fact; the
destination's state when it matters. Four forms:

```text
/// # Errors
/// [`RootError::Conflict`], a mapping is already recorded.                 ← one variant

/// # Errors
/// - [`RegionError::TooSmall`], the region is shorter than `layout`.       ← several variants
/// - [`RegionError::Misaligned`], the base does not meet `layout`'s alignment.

/// # Errors
/// [`BindError`], the region does not fit one, nothing is published yet, or another build laid
/// it out.                                                                 ← a whole enum, its cases listed

/// # Errors
/// Whatever the source reports, having dropped the prefix it had written.  ← propagated, with the state

/// # Errors
/// As [`open`](Self::open).                                                ← delegated
```

The error enum itself:

```text
//! Why <the thing> did not work out[: <case>, or <case>].

/// Why <what could not happen>.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum FooError {
    /// <The condition, as a fact>[: <what to do about it>].
    #[error("foo error: <fragment with {fields} inline>")]
    Variant {
        /// <What the field carries.>
        required: usize,
    },
}
```

One `errors.rs` per crate. The message leads with the type's words (`region
error:`, `reserve error:`, `sign error:`), then a fragment; no trailing period;
fields by name. Variant docs are one line stating what it *is*; no value trivia.

The example `SKILL.md` shows, whole: the error enum, the types it speaks of, and
the method whose `# Errors` names each variant.

````rust
use thiserror::Error;

/// Why a column and row name no square of a board.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PositionError {
    /// The column is past the board's last.
    #[error("position error: column {index} is past a board of {length} columns")]
    ColumnOffBoard {
        /// The column asked for.
        index: u16,
        /// Columns the board has.
        length: u16,
    },
    /// The row is past the board's last.
    #[error("position error: row {index} is past a board of {length} rows")]
    RowOffBoard {
        /// The row asked for.
        index: u16,
        /// Rows the board has.
        length: u16,
    },
}

/// A square of a board, by column and row from the top left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Columns from the left edge.
    pub column: u16,
    /// Rows from the top edge.
    pub row: u16,
}

/// A board of tiles, `columns` wide and `rows` deep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Board {
    /// Columns the board has.
    pub columns: u16,
    /// Rows the board has.
    pub rows: u16,
}

impl Board {
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
    /// assert_eq!(
    ///     board.position(8, 1),
    ///     Err(PositionError::ColumnOffBoard { index: 8, length: 8 }),
    ///     "and not past it"
    /// );
    /// # Ok::<(), PositionError>(())
    /// ```
    pub const fn position(self, column: u16, row: u16) -> Result<Position, PositionError> {
        if column >= self.columns {
            return Err(PositionError::ColumnOffBoard { index: column, length: self.columns });
        }
        if row >= self.rows {
            return Err(PositionError::RowOffBoard { index: row, length: self.rows });
        }
        Ok(Position { column, row })
    }
}
````

## 5 Test, Bench, Example Files

```text
//! <The proof, as a claim>: <what two parties share and what crosses between them>.
//!
//! <One paragraph on the one datum that crosses, or the shape the test exercises.>

#[cfg(test)]
mod tests {
    #[test]
    fn <the_property_as_a_sentence>() { … }
}
```

The compile-fail harness, `tests/trybuild.rs`, is the one test file that may
carry a `cfg` of its own, since it runs a compiler; its test says what it pins,
`each_misuse_fails_to_compile`, and each fixture's `//!` says what that one
refuses:

```text
//! The misuses the types refuse, each a fixture that must not compile. Regenerate a message
//! with `TRYBUILD=overwrite cargo nextest run -p <crate> --test trybuild`, and read it before
//! it is committed.

#[cfg(test)]
mod tests {
    #[test]
    fn each_misuse_fails_to_compile() {
        let cases = trybuild::TestCases::new();
        cases.compile_fail("tests/compile_fail/*.rs");
    }
}
```

In a crate with loom models, it carries `#![cfg(not(loom))]`, since a loom model
drives no compiler.

```text
//! <The property, and the unsoundness forgetting it would allow, in one or two lines.>   ← compile_fail fixture
```

```text
//! What <the thing> costs: <arm one>, against <arm two>.
//!
//! <What the gap between the arms measures, and what the figures do and do not say.>

/// The isolated core the measuring thread runs on.
const CORE: u32 = 6;
```

```text
//! <What the walk-through shows: one sentence.>
//!
//! Run with `cargo run -p <crate> --example <name>`.

#![expect(clippy::print_stdout, reason = "a demo binary reports its result on stdout")]
```

## 6 Manifest

Only the `description` is this skill's; the rest of the shape is
`editing-cargo-manifests`.

```toml
[package]
name        = "tiles-paint"
description = "<the crate summary's pitch clause verbatim, first letter lowercased, trailing period; never `the crate that …`>."

[dependencies]
# external
thiserror = { workspace = true }
# internal
tiles-geometry = { workspace = true }

[dev-dependencies]
# external
trybuild = { workspace = true }
```

`# external` comes first, then `# internal`, and they are the manifest's only
comments.

## 7 Diagrams

A diagram earns its place when it shows a relation prose would have to
serialize: what is made of what, who talks to whom, where bytes live, which
state follows which. Never for a single call, a list, or a two-node arrow.
Inside a ` ```text ` fence; ≤ 12 lines; ≤ 90 columns after the `//! ` prefix;
annotations in an aligned right-hand column introduced by `──`; box-drawing
characters `─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ▶ ▼`.

### 7.1 Shape (What a Crate Is Made Of)

```text
bytes you own ── a buffer, or a mapped segment
│
▼
Region        ── a span of bytes, yours to write
├──▶ Arena    ── bump a cursor, never frees
└──▶ Heap     ── segregated fit, coalescing
```

### 7.2 Layout (Bytes and Fields)

```text
dst ──▶ ┌──────────┬────────────────────┬──────────────────┐
        │ id: u64  │ near: Leg          │ tag: [u8; 32]    │
        └──────────┴────────────────────┴──────────────────┘
        +0         +8                   +24                +56
```

### 7.3 Sequence (Participants as Columns, Time Downward)

```text
writer                 Rcu<H>                 reader
   │                      │                      │
   │                      │◀──── read_with ─────▶│  borrow the published value
   │── store(new) ───────▶│                      │
   │◀──── Retired(old) ───│                      │  displaced, not yet freed
```

### 7.4 State (Boxes and Labeled Transitions)

```text
┌─────────┐  grew()  ┌─────────────┐  disarm()  ┌──────────┐
│ armed 0 │─────────▶│ armed n     │───────────▶│ disarmed │
└─────────┘          └──────┬──────┘            └──────────┘
                            │ drop on `Err` or panic
                            ▼
                    drop_in_place(data[..n])
```

### 7.5 Listing (Asm, Output)

```text
movi v0.2d, #0      ; zero the tag
stp  x1, x2, [x0]   ; id, near.px  -> dst, dst+8
```

Real output only, with the exact call and build named above the fence.

## 8 Tables and Bullets

| use      | when                                                                |
| -------- | ------------------------------------------------------------------- |
| table    | ≥ 3 rows sharing ≥ 2 attributes: features, variants, sources, modes |
| bullets  | ≥ 3 parallel fragments that are not a comparison and not a sequence |
| numbered | a sequence the reader performs, or an ordered protocol              |
| prose    | everything else, including any list of two                          |

Tables: header cells in title case, as headings are; pipes aligned by hand
(`rustfmt` does not touch markdown); code in backticks; no terminal periods; one
sentence before the table and no restatement after it.

```text
| Feature   | Adds                                       |   ← # Crate Features
| --------- | ------------------------------------------ |

| Target    | Value | Source                          |    ← a fact per platform
| --------- | ----- | ------------------------------- |

| Source    | Element Built | Cost          | May Refuse |   ← ## Choosing a <Noun>
| --------- | ------------- | ------------- | ---------- |
```

Bullets: fragments carry no period; full sentences do. Parallel grammar across
items. A bold lead (`**Move-only.**`) when each bullet names a property. No
nested bullets deeper than one level.

## 9 Section Names

Every heading is in title case, as the repository's Markdown headings are, so a
crate page and its README read alike.

- *Task*: gerund, a `##` under `# Examples`, what the reader is doing: `Building
  One Value`, `Building Many`, `Choosing a Source`, `Allocating Through the
  Allocator Trait`.
- *Concept*: the noun: `Pinned Types`, `Contention`, `Ordering`, `Lifecycle`,
  `Safety Model`.
- *Fixed*, exact spelling: `Types`, `Examples`, `Crate Features`, `What It
  Compiles To`, and the item sections `Safety`, `Errors`, `Panics`, `Examples`.
- Never: `Overview`, `Introduction`, `Usage`, `Getting Started`, `Notes`,
  `Miscellaneous`, `Implementation Details`, `Arguments`, `Parameters`,
  `Returns`, `Example`, `HOT`.
