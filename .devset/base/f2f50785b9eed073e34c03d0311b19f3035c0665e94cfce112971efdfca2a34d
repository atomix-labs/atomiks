# Comments: Everything That Is Not Rustdoc

Read this before a `// SAFETY:`, `// INVARIANT:`, `// ORDERING:` or inline `//`
comment, an `#[expect]` reason, an assertion or `expect` message, a test, a
fixture, a bench or an example, a file's `//!`, or a manifest's `description`.
Same reader as the docs: the code is open, so a comment says only what the code
cannot.

What a `// SAFETY:`, an `// INVARIANT:` or an `// ORDERING:` must establish is
`writing-unsafe-rust`'s; this says how each is worded and where it sits.

Contents: 1 `// SAFETY:` · 2 `// INVARIANT:` · 3 `// ORDERING:` · 4 Inline `//`
· 5 `#[expect]` · 6 Messages · 7 Tests · 8 File headers · 9 Manifest

## 1 `// SAFETY:`

Above every `unsafe {}` block and every `unsafe impl`, and above the
`#[expect(unsafe_code, …)]` that goes with it, so the proof reads before the
permission. clippy's `undocumented_unsafe_blocks` accepts the comment above the
statement or above its attributes; the house writes it above them. One `unsafe`
operation per block, so each comment proves one operation's preconditions; never
a `// SAFETY:` on safe code.

Under `strict`, `undocumented_unsafe_blocks`, `multiple_unsafe_ops_per_block`
and `unnecessary_safety_comment` hold all three.

```text
// SAFETY: `mid` is at most the length, checked above, which is all `split_at_unchecked`
// needs.
#[expect(unsafe_code, reason = "a split the check above has bounded")]
Some(unsafe { squares.split_at_unchecked(mid) })
```

It names, for each precondition the operation's `# Safety` lists, the *fact*
that discharges it, in as many lines as that takes: one line where one fact
discharges them all, a sentence for each where they are several. Four sources of
fact:

| source                | pattern                                     | example                                                                                         |
| --------------------- | ------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| the caller's contract | `the caller upholds <item>'s contract[, …]` | ``// SAFETY: the caller upholds `__init`'s contract, including pinning unless `I` cancels it.`` |
| the preceding step    | `<what just happened>, so <permission>`     | `// SAFETY: the initializer reported success, so the guard may count it.`                       |
| a check in scope      | `<the bound>, checked above[, so …]`        | ``// SAFETY: `index` is below the length, checked above.``                                      |
| a field's invariant   | `by the field INVARIANT <the fact>`         | `// SAFETY: by the field INVARIANT the slot holds a tile that nothing else drops.`              |

A proof of several preconditions runs over the lines it needs:

```text
// SAFETY: the head lies inside the region, backing outlives `'a`, and `self` is consumed,
// so nothing else covers those bytes.
let head = unsafe { Self::from_raw_parts(self.base, mid) };
```

Chaining, when the fact was stated a few lines up: `// SAFETY: as above.`, ``//
SAFETY: as `base`.``, ``// SAFETY: as `read_at`, and nothing else reaches the
field while the lock is held.``

On an `unsafe impl`, state why every obligation of the trait holds, then the
`#[expect]`, then the impl: ``// SAFETY: one copy initializes every element, and
a `Copy` type cannot refuse or panic.``, `// SAFETY: the only mutable state is
the atomic cursor, so handles never alias.` (`Send`), then ``// SAFETY: see the
`Send` impl.`` (`Sync`).

Never: restate the operation (`// SAFETY: write to the pointer`), open with
`This is safe because`, say `trust me`, or explain what `unsafe` means. Name the
hazard the code rests on, not a proxy for it (*torn-read bit-pattern validity*,
not *reachability*).

Rationale that is not a safety argument goes on its own `//` line above,
separated by a bare `//`:

```text
// Built where it lands, so nothing is moved in; on `Err` it wrote nothing.
//
// SAFETY: a fresh unaliased slot for one `T`, and `Init` cancels the pinning duty.
unsafe { PinInit::__init(init, slot) }?;
```

A `# Safety` section on an `unsafe fn` and the `// SAFETY:` inside it are
different sentences: the section is the caller's obligation, the comment is why
this body's `unsafe` op is discharged by it (``// SAFETY: the caller upholds
`raw_try_init`'s contract; the initializer cannot fail.``).

## 2 `// INVARIANT:`

On each field a `// SAFETY:` relies on, where the field is declared: above the
field, and above its `///` where it has one, so the doc says what the field is
and the invariant what every writer of it keeps. It states the fact, then names
the writers, since a safe method that writes the field wrong breaks the proof as
surely as an unsafe one; one comment covers the fields that keep a fact
together, naming each. Each `// SAFETY:` that rests on it cites it as "by the
field INVARIANT".

```text
pub struct Row<'a> {
    // INVARIANT: `base` and `length` are the pointer and length of one `&'a [u8]`; `new` is their
    // only writer.
    /// The row's first square.
    base: NonNull<u8>,
    /// Squares the row holds.
    length: usize,
    /// Borrows the squares for `'a`.
    squares: PhantomData<&'a [u8]>,
}
```

```text
// SAFETY: by the field INVARIANT these are the squares of one `&'a [u8]`: non-null, aligned,
// initialized, inside one allocation of at most `isize::MAX` bytes, and shared for `'a`, so no
// `&mut` overlaps them.
#[expect(unsafe_code, reason = "a view of the row `new` borrowed")]
unsafe { slice::from_raw_parts(self.base.as_ptr(), self.length) }
```

A field that no proof relies on needs none: its invariant, if it has one, is a
sentence of its `///`, `/// Squares that hold a tile. Never more than the board
has.`

## 3 `// ORDERING:`

Above every atomic operation, `SeqCst` included, naming its ordering and what it
pairs with, so a reader checks each pair from both ends and a change to one end
finds the other: a `Release` store names the `Acquire` loads that read it, and
the other way about; a `Relaxed` one says it publishes nothing but itself, or
what orders the rest.

```text
// ORDERING: Release, pairing with the Acquire load in `winner`, so a reader that sees the game
// over sees the winner stored before it.
self.finished.store(true, Release);
```

A function whose operations share one ordering says so once, at its top:

```text
// ORDERING: Relaxed throughout. The word hands out disjoint ranges and publishes nothing;
// whatever a caller lays in the bytes it took, it releases itself.
```

A repeat chains, as `// SAFETY:` does: ``// ORDERING: Relaxed, as in `stop`.`` A
`SeqCst` fence carries a paragraph: the store-load order the protocol rests on,
and why nothing weaker gives it.

```text
// ORDERING: Release, so a painter that sees this flag sees what this one did before; then a
// SeqCst fence, the store-load order the exclusion rests on. Both painters announce, then look,
// and the fences' one order puts one painter's look after the other's announcement.
own_flag.store(true, Release);
fence(SeqCst);
```

## 4 Inline `//`

Only where the code cannot say it: a reason, an invariant, a non-obvious
consequence, a deliberate absence. Capitalized, ends with a period, one fact per
comment.

| use                    | example                                                                                               |
| ---------------------- | ----------------------------------------------------------------------------------------------------- |
| a non-obvious choice   | ``// A mask, not a remainder: `align()` is a power of two, but only a divide would prove it.``        |
| a phase label          | ``// Commit: `chunk` shrinks to `required` and turns in use, in one store.``                          |
| a deliberate absence   | ``// No `Reclaiming`: a bump cursor never steps back, so a block returned here is not served again.`` |
| a deliberate omission  | ``// `Zeroable` … are deliberately absent: <reason as fact + consequence>.``                          |
| a `#![feature]` entry  | ``// `impl_restriction`: the settle is sealed by the payload it indexes, which a module cannot say.`` |
| a `cfg` that looks odd | `// Miri drives no compiler.`                                                                         |
| a debug tripwire       | `// A double-claim (unprovable in types) arrives free; a tripwire, coalescing can hide it.`           |

A `#![feature]` a crate enables for an unstable API of its own gets its line;
the lints' features are never written in a file.

Delete any comment a rename would make redundant, and consider the rename. Never
narrate the next line, never explain what a lint wants, never cite a rule ID or
a phase.

An existing comment that states its fact in one terse line is finished.
Rewriting it longer, or into a figure of speech (`the type system catching up`),
is a regression even when it reads well: keep the fact, drop the flourish. ``//
`sign` computes into a stack `Tag`, no heap.`` beats two lines saying the same
thing more elegantly.

## 5 `#[expect(lint, reason = "…")]`

`expect`, never `allow`, so a suppression is reported once its cause is gone.
The reason is the *cause*, lowercase, no terminal period, one clause; it names
the concrete thing the lint misreads, never the consequence (`"otherwise clippy
complains"`) or the lint's name.

`unfulfilled_lint_expectations` is denied, so a stale `#[expect]` fails the
build.

Under `strict`, `allow_attributes` refuses an outer `#[allow]`, and
`allow_attributes_without_reason` an `#[expect]` with no reason.

The one `allow` is the `dead_code` atop `tests/testing/mod.rs`, which
`writing-rust-tests` explains: each test binary uses part of that module, so an
`#[expect]` there fails in the binary that uses it all.

```text
#[expect(clippy::mem_forget, reason = "pin-init disarms its field guards this way")]
#[expect(clippy::indexing_slicing, reason = "a `Bucket` is below `NUM_BUCKETS`, this array's length")]
#[expect(clippy::indexing_slicing, reason = "as `index`")]
```

`unsafe_code` is expected at the narrowest scope that holds the unsafe, the
statement, the item, the `impl` or the module, and on the crate only where
unsafe is its whole purpose; the reason names the concrete unsafe and why no
safe form serves:

```text
#![expect(
    unsafe_code,
    reason = "writing a value straight into raw destination bytes is this crate's whole purpose"
)]
```

A file whose role excuses a lint says so once at the top, in the same voice:
`#![expect(clippy::print_stdout, reason = "a demo binary reports its result on
stdout")]`. An integration test needs none: its tests sit in `#[cfg(test)] mod
tests`, as a unit test's do.

No comment ever explains an `#[expect]`; the reason is its one home. If the only
unsafe in a crate is a derived `unsafe impl`, the crate-level expect is
unfulfilled: delete it.

## 6 Messages

Every message is a lowercase fragment, no terminal period, saying what is true
or what went wrong, never what the code is doing.

| site                                    | form                                            | example                                                                       |
| --------------------------------------- | ----------------------------------------------- | ----------------------------------------------------------------------------- |
| `const _: () = assert!(cond, "…");`     | the property, as a claim                        | `assert!(size_of::<Position>() == 4, "a column and a row, and nothing else")` |
| `debug_assert!(cond, "…")`              | the violation, as a fact                        | `"a pointer outside the recorded mapping"`                                    |
| `.expect("…")` in tests and examples    | why it cannot fail here                         | `expect("the slack absorbs the pad")`, `expect("8 bytes, 8-aligned")`         |
| `assert_eq!(left, right, "…")` in tests | the property pinned, continuing the sentence    | `"the word survives a decode"`, then `"and the value an encode"`              |
| `#[error("…")]` on a variant            | `<type words> error: <fragment>`, fields inline | `"region error: needs {required} bytes, region holds {available}"`            |
| `#[must_use = "…"]` on a guard          | what stays held until it drops                  | `"the tile stays locked until the guard drops"`                               |
| `#[ignore = "…"]` on a test             | what the test needs to run                      | `ignore = "needs a tile server on localhost:7070"`                            |
| `panic!` in a test's impossible arm     | what was wanted, and what arrived               | `panic!("a one-byte layout fits, not {other:?}")`                             |

Assertion messages in a test read as one running argument: `"to itself"`, `"and
forwards"`, `"never backwards"`. Not every assertion needs one; add it where the
*why* is not the expression.

## 7 Tests

- A `#[test]` fn name is the property, as a snake_case sentence, its subject
  first: `a_slice_becomes_a_region_over_its_own_bytes`,
  `an_each_that_gives_up_drops_the_prefix_exactly_once`. It carries no doc
  comment, but for the compile-time property below.
- A `//` comment above a test says what it pins and why, when the name cannot:
  `// The bug this closes aligned the offset, so it was correct only when the
  base already was.`
- Helpers, fixtures, and test-local constants get one summary line stating their
  role in the proof: `/// A layout that is valid by construction.`, `/// Slots
  the run writes into, and the one that refuses.`; a constant that pairs with
  another links it: ``/// See [`SLOTS`].`` No lint asks for these lines: the
  house writes them, and review holds them.
- A test module opens with `//!` only for setup notes (how to regenerate, why a
  `cfg` excludes a runner). In a crate with loom models, the other tests sit
  under `#[cfg(test)] #[cfg(not(loom))]`, and a model is `#[cfg(test)]
  #[cfg(loom)] mod model`, with a `// Run with …` comment and one paragraph on
  what the model can and cannot see.
- Inline comments in tests explain a non-obvious *setup*, never the assertion.
- `#[deny(unused_unsafe)]` on a test turns "this door is `unsafe`" into a
  compile-time property; say so in the test's `///`.
- A test-only `unsafe fn` helper carries `# Safety` like any other.

What a test pins, where it goes and how it runs is `writing-rust-tests`'s; this
says how its names, comments and messages read.

## 8 File Headers

Every non-`lib.rs` file opens with `//!`. One line by default; a contract
paragraph and one example only for a module that is itself a surface.

| file                      | `//!` says                                                                              |
| ------------------------- | --------------------------------------------------------------------------------------- |
| module                    | the role, naming the key item                                                           |
| `errors.rs`               | `Why <the thing> did not work out[: <the two cases>].`                                  |
| `testing.rs`              | `What the in-crate tests share.` or the fixture's role                                  |
| `tests/<name>.rs`         | the proof, as a claim, and the one datum that crosses                                   |
| `tests/trybuild.rs`       | `The misuses the types refuse, each a fixture that must not compile.`                   |
| `tests/compile_fail/*.rs` | one or two lines: the unsoundness the compile failure prevents, nothing of harness      |
| `benches/<name>.rs`       | what is measured and what the gap between arms means; every `const` documented          |
| `examples/<name>.rs`      | what the walk-through shows, then ``Run with `cargo run -p <crate> --example <name>`.`` |
| a bin's `main.rs`         | the operator's view: what it does, how it is driven, a `text` fence of invocations      |

The test in `tests/trybuild.rs` is named for what it pins,
`each_misuse_fails_to_compile`, and carries no doc: each fixture's `//!` says
what it prevents. A fixture's message is written with `TRYBUILD=overwrite cargo
nextest run -p <crate> --test trybuild`, and read before it is committed.

In a crate with loom models, `tests/trybuild.rs` alone of the test files carries
a `cfg` of its own, `#![cfg(not(loom))]`, since a loom model drives no compiler.

Under `strict`, an example that prints opens with
`#![expect(clippy::print_stdout, reason = "a demo binary reports its result on
stdout")]`.

## 9 Manifest

- `description` is the crate summary's pitch clause verbatim, first letter
  lowercased, trailing period, an imperative or noun phrase that never names the
  crate itself (`the one crate that …`, `a library for …`): `description =
  "build a value where it will live, never moved there."`, `description =
  "allocators over a byte range you own."` An acronym or a name keeps its case
  inside the clause and never opens it: `"clients for the REST API a tile server
  speaks."`, not `"REST clients for …"`, since a capital first letter reads as a
  sentence.
- Dependencies sit under `# external` / `# internal` comments, `# external`
  first (taplo sorts within each group). Those two are the *only* comments a
  manifest carries: no trailing gloss on a dependency, no paragraph above a
  table. A dependency whose purpose is not its name is named in the crate docs
  where its guarantee is used, not beside the entry.
- A feature that is not self-explanatory is explained once, in the crate docs'
  `# Crate Features` table, never in the manifest.
- Everything else about a manifest (shape, inheritance, features, targets, the
  gates) is `editing-cargo-manifests`.
