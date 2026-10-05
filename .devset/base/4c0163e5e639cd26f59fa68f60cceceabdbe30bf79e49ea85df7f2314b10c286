# Style: Voice, Compaction, and What to Cut

Read this before any summary line or body prose, before tightening a doc, and
before an example, a link or a list. Derived from the crates `exemplars.md`
adapts: `mem-init`, `mem-region` and `mem-allocators`. Where the
[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/documentation.html)
or the
[Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/guidelines/docs/index.html)
say otherwise, the exemplar crates win.

Contents: 1 Summary lines · 2 Body prose · 3 Compaction moves · 4 Cut list with
rewrites · 5 Examples · 6 Links · 7 Words and punctuation · 8 Vocabulary

## 1 Summary Lines

The first paragraph is what rustdoc shows in every listing. It is one sentence,
fits one line when it can (a second line is a clause of the same sentence or an
invariant, never a third), ends with a period, and carries information the name
does not. Then a blank `///` line, then the body.

| item            | form                                                                                   | write                                                                                 | not                                            |
| --------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ---------------------------------------------- |
| crate           | the pitch (the manifest `description`), or `Topic: pitch.`; never names itself a crate | Allocators over a byte range you own.                                                 | The one crate that provides allocators.        |
| module          | one line: the noun or gerund, naming the key item                                      | A bump cursor over one region.                                                        | This module contains the arena implementation. |
| fn / method     | verb first, present tense, implied subject                                             | Splits into the first `mid` bytes and the rest, or `None` when `mid` is past the end. | This function splits the region.               |
| getter          | name the value: `The …`, `Where …`, `Bytes …`, `How many …`                            | Bytes the span holds.                                                                 | Returns the length of the span.                |
| predicate       | `Whether …`                                                                            | Whether the span holds no bytes.                                                      | Returns true if empty.                         |
| `Option` result | the value, then ``, or `None` when …``                                                 | The next `layout`-shaped region, or `None` when what is left cannot hold one.         | Returns an Option containing the region.       |
| constructor     | the state it yields, or the door it is; never `Creates a new`                          | The range of `length` bytes at `start`.                                               | Creates a new `Span`.                          |
| struct / enum   | its role: noun phrase, or verb phrase for an active object                             | Drops the prefix a run wrote, unless the run completes.                               | A struct that represents a guard.              |
| adapter type    | ``What [`f`] builds.``                                                                 | What [`each`] builds.                                                                 | The return type of `each`.                     |
| trait           | `How to …` for a capability, the role otherwise                                        | How to build a run of `T` at a destination that already exists.                       | Trait for run initializers.                    |
| marker trait    | the property an impl asserts                                                           | Bytes returned to this allocator become available again.                              | Marker trait for reclaiming allocators.        |
| assoc. type     | what it is, per impl                                                                   | A stored location: a pointer for [`Local`], an [`Offset`] for [`Shared`].             | The stored type.                               |
| type alias      | the role, then why an alias                                                            | The record, and the bytes cut from it.                                                | Alias for `Resident<Header>`.                  |
| field           | noun phrase; invariant after a period or colon                                         | How many are initialized. Zero once disarmed.                                         | The length.                                    |
| `PhantomData`   | what it fixes and how                                                                  | Fixes the element and error types by owning them.                                     | Marker.                                        |
| error enum      | `Why …` (what could not happen)                                                        | Why an allocator could not make room.                                                 | Errors returned by the allocator.              |
| error variant   | the condition, as a fact                                                               | The region is shorter than the layout.                                                | Returned when too small.                       |
| other variant   | when it is produced, or what it carries                                                | The close frame is out; inbound drains until the peer acknowledges.                   | Variant `Closing`.                             |
| const / static  | what it fixes, and why that value                                                      | The smallest region an arena fits in, and the alignment every arena region must meet. | Minimum region size.                           |
| macro           | verb first, like a fn                                                                  | Builds a record straight into its destination.                                        | Macro for in-place init.                       |
| test helper     | its role in the proof                                                                  | Runs `source` over `N` fresh slots and hands back what it built.                      | Helper function.                               |
| test fixture    | one line on why it exists                                                              | A record, to prove `each` builds an element in place.                                 | Test struct.                                   |

An article is fine (`A range of the address space…`, `The raw address.`). Never
article + the item's own kind (`A struct that…`, `A trait for…`, `A function
which…`). Never open with `This`, `Returns`, `Creates`, `Represents`, `Used`,
`Helper`, `Wrapper`, `Provides`, `Allows`.

## 2 Body Prose

State, in this order and only when true:

1. **Why it exists / when to use it**, and when not: ``Prefer [`x`] for …, so
   …``.
2. **The contract**: what is guaranteed, in what order, what remains on failure.
3. **The reason for each non-obvious choice**, as fact + consequence in one
   sentence: *"pin-init's derive emits `::pin_init::` absolute paths with no way
   to redirect them, which makes every consumer depend on a crate it never
   names."*
4. **The cost**: allocation, copy, lock, syscall, `compare_exchange`, "one
   `memcpy`".

Never: how it works (the code shows it), a parameter-by-parameter description,
the return type, the derive list, a repeat of the summary, what a linked item
does, who consumes this crate.

Enumerated properties of a type go in bullets with a bold lead: `-
**Move-only.** Laying a structure into a region consumes it, so …`. Three
bullets is the norm; two is a sentence.

Speak the crate's own domain and keep its nouns fixed: in `mem-init` a *slot* is
the destination, a *run* is `n` slots, a *source* is a `RunInit`, an initializer
*refuses* rather than fails. Define a coined noun once, where the type is, and
link every later use. A term that only makes sense to someone who already knows
the design is a defect.

Each crate speaks for itself: link a dependency's type where the guarantee comes
from it, never describe a dependent or how others use this crate, never
re-explain a substrate concept the defining crate documents.

## 3 Compaction Moves

Apply in order until the sentence fits.

| move                                      | before                                                                      | after                                                                |
| ----------------------------------------- | --------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| drop the subject                          | This function writes the value into `destination`.                          | Writes the value at `destination`.                                   |
| name the value, drop `Returns`            | Returns the number of elements that will be written.                        | How many it writes.                                                  |
| clause → participle                       | The prefix is dropped after it has been written.                            | …, having dropped the prefix it had written.                         |
| two sentences → semicolon or colon        | `Ok` means all are initialized. `Err` means none are.                       | `Ok` means all are initialized; `Err` means none are.                |
| explanation → link                        | See the pin-init docs at <url> for details on `PinInit`.                    | It produces a [`PinInit`].                                           |
| condition list → comma list               | `destination` must be aligned. It must be writable. It must not be aliased. | `destination` is aligned, writable, and unaliased.                   |
| `in order to` → `to`; `is used to` → verb | Used in order to build in place.                                            | Builds in place.                                                     |
| `which is` / `that is` → apposition       | A guard, which is a struct that drops the prefix.                           | A guard that drops the prefix.                                       |
| cut the hedge                             | This should generally be preferred for large types.                         | Prefer it for a large `T`.                                           |
| cut the metaphor's setup                  | Think of it as a factory: it hands out values.                              | Hands out values.                                                    |
| reason → fact + consequence               | We chose an alias because we felt a newtype was unnecessary here.           | An alias, not a newtype: every caller already means this exact word. |

## 4 Cut List with Rewrites

| cut                                                            | why                                      | write instead                                                      |
| -------------------------------------------------------------- | ---------------------------------------- | ------------------------------------------------------------------ |
| `This struct/fn/method …`                                      | the subject is the item                  | start with the verb or the role                                    |
| `Returns a new instance of X` / `Creates a new X`              | says nothing the signature does not      | the state it yields: `An empty tally.`                             |
| `A struct that holds …` / `Represents …`                       | the kind is visible                      | the role                                                           |
| `# Arguments` / `# Parameters` / `# Returns`                   | Rust convention is prose                 | fold names into the summary: ``Copies `source` to `destination`.`` |
| ``The `source` slice.`` on `source: &[T]`                      | restates the type                        | `What to copy.`                                                    |
| `// increment length` above `length += 1`                      | narrates the line                        | delete, or the invariant it maintains                              |
| `Note that …`, `It is important to …`, `Please`                | filler                                   | the fact                                                           |
| `simply`, `just`, `basically`, `essentially`                   | filler                                   | delete                                                             |
| `will`                                                         | tense drift                              | present tense                                                      |
| `can be used to`, `allows you to`, `provides`                  | indirection                              | the verb                                                           |
| `for more information see …`, `See also`                       | link ceremony                            | link the noun inline                                               |
| `powerful`, `simple`, `easy`, `efficient`, `zero-cost`         | unverifiable                             | the measured fact, or nothing                                      |
| `generic`, `reusable`                                          | editorializing; being in a lib proves it | what it does                                                       |
| `etc.`, `and so on`, `various`                                 | vague                                    | the full list, or the rule that generates it                       |
| `# Example` (singular)                                         | nonstandard                              | `# Examples`                                                       |
| blank line after `# Heading`                                   | house style                              | content on the next line                                           |
| a heading over one sentence                                    | structure without content                | a clause in the prose                                              |
| a bullet list of two                                           | a sentence with `and`                    | prose                                                              |
| `TODO`, `FIXME`, `(?)`, `should probably`                      | debris                                   | ask (SKILL.md), and omit                                           |
| `RC10 phase B`, `chunk 3`, `as agreed`, PR/issue talk          | dev-process noise                        | nothing; history lives in git                                      |
| `# HOT`, rule IDs (`STY-CMT-5`), `DEVIATION` tags              | belong to a suite this tree lacks        | nothing                                                            |
| the summary repeated as the first body sentence                | duplication                              | start the body with *why*                                          |
| a `///` on a `#[derive(Debug, Clone)]` or an obvious `Default` | nothing to say                           | nothing                                                            |
| a module doc that lists its items                              | the listing is the module page           | one line naming the role                                           |
| `Entry point: …` / `Entry points: …`                           | restates signatures                      | nothing; or the one-line contract                                  |
| a doc that explains an `#[expect]` beside it                   | the `reason` is its one home             | nothing                                                            |
| a comment naming the lint (`clippy wants …`)                   | lint machinery narration                 | the fact the code rests on                                         |

## 5 Examples

- The shape: `use core::…;` lines, a blank line, `use mem_…::{…};` lines, a
  blank line, the code; `rustfmt` orders them, so write them that way. A
  `#![feature(…)]` opens an example only for an unstable API it calls, never for
  a lint.
- Show *why* the item is used, not that it can be called. A small domain type
  (`Leg`, `Order`, `Tag`) beats `Foo`; a real quantity (`4096`, `64 * 1024`)
  beats `42`.
- End with an `assert!`/`assert_eq!` that proves the point. Its message is a
  fragment continuing the sentence: `assert_eq!(first.base().addr().get() % 64,
  0, "carved at the alignment asked for");`.
- Unwrap with `.expect("<why it cannot fail>")`, a fragment: `expect("8 bytes,
  8-aligned")`, `expect("the slack absorbs the pad")`. Never bare `unwrap()`.
- `?` for a real error path, closed by a trailing hidden line, not a wrapper fn:
  `# Ok::<(), core::alloc::LayoutError>(())` or `# Ok::<(), Box<dyn
  core::error::Error>>(())`.
- A `// SAFETY:` above each `unsafe {}`, proving each precondition, as in code
  (`comments.md` §1).
- One `//` line per non-obvious step, naming the cost or the fact, never the
  operation: ``// A `Vec` is byte-aligned, so over-allocate and let the cut find
  the record's alignment.``
- Hide with `# ` only setup that would obscure the point (`#
  install(OwnerId::new(NonZeroU64::MIN))?;`, `# let _ = boxed;`); keep every
  `use` visible.
- Infallible `Result<(), Infallible>` is consumed with `let Ok(()) = …;`, not
  `expect`.
- `no_run` for I/O or a process; `text` for anything that is not Rust; `ignore`
  never.
- Crate examples are the long ones; an item example is the shortest that still
  shows the why. Two items sharing a situation share one example and a link from
  the other.
- Substantive public fns, and every public type a user constructs or drives,
  carry `# Examples`; trivial accessors do not, and a small type whose whole use
  already appears in a neighbour's example (a `Cut` inside `Span::cut`'s) needs
  none of its own.
- Siblings are documented alike: the members of one family, each clock, each
  point type, each error, all have an example or none has, since a reader who
  finds one on a sibling looks for it on the next. Where this and the two rules
  above pull apart, siblings win: a small type whose sibling has an example has
  one of its own, though a neighbour's already shows its use.
- A crate page's example sits under `# Examples`, as an item's does, a task each
  under a `##` where it shows several.

## 6 Links

- Every item mentioned is an intra-doc link on first use per doc block:
  ``[`RunInit`]``, ``[`len`](Self::len)``, ``[`cut`](Region::cut)``,
  ``[`pin_init!`](crate::pin_init!)``. Backticks inside the brackets always;
  ``[`Foo`](Foo)`` is a denied redundancy.
- A path a short name cannot reach goes in the target:
  ``[`Layout`](core::alloc::Layout)``, ``[`Shared`](crate::Shared)``, or as a
  reference definition at the bottom of the block: ``[`Shared`]:
  crate::Shared``.
- Reference definitions sit at the end of the doc block, in order of first use,
  with short stable labels: `[rust#125632]`, `[goal]`, `[pin-init]`, `[loom]`.
- A dependency's item is linked by the crate's path,
  ``[`Layout`](core::alloc::Layout)``, ``[`PinInit`](pin_init::PinInit)``; only
  a normal dependency resolves. A crate that is not one is not linked, and not
  named: a relative HTML path to its pages breaks on docs.rs and wherever the
  item is inlined, and nothing checks it. A published crate outside the
  workspace may be linked by its docs.rs URL.
- An unstable, cfg-gated, or foreign item rustdoc cannot resolve is linked by
  URL reference definition, never demoted to a bare code span:
  ``[`AtomicPrimitive`]:
  https://doc.rust-lang.org/std/sync/atomic/trait.AtomicPrimitive.html``,
  `[loom]: https://docs.rs/loom`.
- A variant gated on `cfg(target_os)` is named in plain backticks on shared
  surfaces; a link to it breaks on the other target.
- Link the noun, never `here`/`docs`/`see`. Never a bare URL (`bare_urls` is
  denied).
- A link replaces an explanation: if the target says it, this doc does not.

## 7 Words and Punctuation

- Clauses hang off a colon or semicolon, or become a participle. No em dash
  (`—`) anywhere, and no `--` standing in for one: a colon, a semicolon, a comma
  or a parenthesis, by what follows.
- Backticks on every identifier, path, literal, feature name, lint name, and
  shell command.
- **Bold** for the one load-bearing claim in a block (`**Plain, not atomic, and
  that is the point**`, `Aligns the **address**, not the offset`); *italics* for
  a coined term where it is defined (*initializer*, *place*) or a stressed word
  (`whose contract *is* reclamation`).
- Title case for headings and table headers, `# Crate Features`, `## Building
  One Value`, `| Feature | Adds |`; no terminal period on headings, table
  headers, or fragment bullets.
- Numbers as words below ten in prose (``one `T` ``, `two flags`); digits for
  measurements and code (`7 instructions against 6`, `64 GiB`, `4 KiB`).
- Present tense, indicative. `Prefer …` and `Use …` for guidance, never
  `should`/`must` in prose; `must` belongs in a `# Safety` precondition only
  when the indicative reads wrong.
- One space after a period. No exclamation marks, no emoji, no `e.g.`/`i.e.`
  where `like`/`that is` reads better.

## 8 Vocabulary

The crate's own nouns and verbs come first, and the workspace's after them;
where neither has a word, these are the ones the exemplar crates use. Reuse one
before coining another.

| word                 | means                                                          |
| -------------------- | -------------------------------------------------------------- |
| door                 | a constructor or entry point (`the safe door`, `the raw door`) |
| spend / spent        | consumed by value, so it cannot be used twice                  |
| refuse / refusal     | an `Err` that is a decision, not a fault                       |
| lay / laid           | initialize a structure into a region                           |
| hand out / hand back | allocate / free                                                |
| carve / cut          | take a sub-range from a range                                  |
| name                 | point to (``a pointer that names a live `B` ``)                |
| cover / hold         | span / contain bytes                                           |
| peer                 | another process over the same bytes                            |
| publish / verify     | make visible with a release store / check before trusting      |
| record / tail / body | a header, the bytes it governs, both together                  |
| fresh                | uninitialized and unaliased                                    |
