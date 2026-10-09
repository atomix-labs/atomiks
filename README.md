<!-- >>> devset: project >>> -->
<!-- dprint-ignore-start -->

<h1 align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/atomix-labs/atomix/main/docs/src/media/logo-dark.svg">
    <img alt="atomix" src="https://raw.githubusercontent.com/atomix-labs/atomix/main/docs/src/media/logo-light.svg" height="56">
  </picture>
</h1>

<p align="center">Typed atomics for any value that fits one atomic, and the locks built on them.</p>

<p align="center">
  <a href="https://github.com/atomix-labs/atomix/actions/workflows/check.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/atomix-labs/atomix/check.yml?branch=main&amp;style=flat-square&amp;label=check"></a>
  <a href="https://atomix-labs.github.io/atomix/"><img alt="Book" src="https://img.shields.io/badge/book-read-blue?style=flat-square"></a>
  <a href="https://github.com/atomix-labs/devset"><img alt="managed with devset" src="https://img.shields.io/badge/managed_with-devset-0969da?style=flat-square&amp;logo=data:image/svg%2bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHRpdGxlPmRldnNldDwvdGl0bGU+PHBhdGggZmlsbD0iI2YwZjZmYyIgZD0ibTE2IDMgMTMgNi41TDE2IDE2IDMgOS41WiIvPjxwYXRoIGZpbGw9Im5vbmUiIHN0cm9rZT0iI2YwZjZmYyIgc3Ryb2tlLWxpbmVjYXA9InJvdW5kIiBzdHJva2UtbGluZWpvaW49InJvdW5kIiBzdHJva2Utd2lkdGg9IjIuNSIgZD0ibTMgMTYgMTMgNi41TDI5IDE2TTMgMjIuNSAxNiAyOWwxMy02LjUiLz48L3N2Zz4K"></a>
</p>

<!-- dprint-ignore-end -->
<!-- <<< devset: project <<< -->

atomix gives each value that fits one atomic an atomic of its own type:
integers, plain or held to a range, `NonZero`s, `char`s, floats, pointers,
`Option`s that spend a spare bit pattern on `None`, and the structs and enums
`#[derive(Atom)]` packs into one word or two. Its orderings are types, checked
as it compiles, and each operation is the instruction its name promises: where a
target has none, the operation does not exist, and a compare-exchange loop is
`update`, by name. The locks built on these atomics are yet to come, in
`atomix-lock`.

| Crate           | What it holds                                                                    |
| --------------- | -------------------------------------------------------------------------------- |
| `atomix-rs`     | everything: `atomix-core` at its root, and `atomix-derive` with `derive`         |
| `atomix-core`   | the typed atomic, the values it holds, and the orderings and primitives under it |
| `atomix-derive` | `#[derive(Atom)]`, and the read-modify-writes a newtype takes from its field     |

## Install

```sh
cargo add --git https://github.com/atomix-labs/atomix atomix-rs --features derive
```

The package is `atomix-rs`, since crates.io's `atomix` is an unrelated
placeholder, and the library it adds is `atomix`: `use atomix::Atomic;`. atomix
needs a nightly Rust, `nightly-2026-09-28` or newer, and builds for `aarch64`
and `x86_64`, little-endian with 64-bit pointers, on any OS, and for Windows'
`arm64ec`; CI tests it on Linux and macOS. It is `no_std`. Some operations need
a CPU feature a target's default lacks, such as `cmpxchg16b` for two words on
`x86_64` Linux, which a flag adds: [Platforms][platforms] lists each.

## Quick Start

```rust
use atomix::ordering::{Acquire, Release};
use atomix::{Atom, Atomic};

/// The side of the book an order rests on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
enum Side {
    Bid,
    Ask,
}

/// A resting quote: 32 bits of price, 16 of quantity, then a bit of side, in a `u64`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
struct Quote {
    price: u32,
    quantity: u16,
    side: Side,
}

// `None` takes a bit pattern no quote has, so the `Option` does not widen the word.
static BEST_BID: Atomic<Option<Quote>> = Atomic::new(None);

fn main() {
    BEST_BID.store(Some(Quote { price: 10_050, quantity: 300, side: Side::Bid }), Release);
    assert_eq!(BEST_BID.load(Acquire).map(|quote| quote.quantity), Some(300), "the quantity bid");
    assert_eq!(size_of_val(&BEST_BID), 8, "the quote in one `u64`, `None` too");
}
```

## A Short Tour

### Orderings Are Types

An ordering is accepted only where it means something, so a load that promises
to publish does not compile:

```text
error[E0277]: `atomix::ordering::Release` is not a load ordering
   --> src/main.rs:7:23
    |
  7 |     let _ = NEXT.load(Release);
    |                  ---- ^^^^^^^ expected `Relaxed`, `Acquire` or `SeqCst` from `atomix::ordering`
    |                  |
    |                  required by a bound introduced by this call
    |
    = help: the trait `LoadOrdering` is not implemented for `atomix::ordering::Release`
    = note: `Release` and `AcqRel` order a store, and a load has none
```

### No Hidden Loops

An operation exists only where the target runs it without a compare-exchange
loop. `x86_64` has no instruction that returns the value an OR or a maximum
found, so there `fetch_or` and `fetch_max` do not compile. `or`, which returns
nothing, needs no loop on any target in an optimized build, and a maximum is a
loop, written as `update`:

```rust
use atomix::AtomicU64;
use atomix::ordering::{AcqRel, Acquire, Release};

/// The flags a connection has raised.
static FLAGS: AtomicU64 = AtomicU64::new(0);
/// The highest sequence number a thread has seen.
static HIGHEST: AtomicU64 = AtomicU64::new(0);

fn main() {
    // `lock or`, or LSE's `ldset`.
    FLAGS.or(0b10, Release);
    // `fetch_max` would be LSE's `ldumax` on `aarch64`; on `x86_64` the loop is named.
    assert_eq!(HIGHEST.update(AcqRel, Acquire, |highest| highest.max(42)), 0, "the highest before");
}
```

On `x86_64`, `fetch_max` names the loop it would be:

```text
error[E0277]: `u64` has no atomic maximum or minimum without a compare-exchange loop on this target
  --> src/main.rs:7:13
   |
 7 |     HIGHEST.fetch_max(42, AcqRel);
   |             ^^^^^^^^^ this would be a compare-exchange loop
   |
   = help: the trait `MinMax` is not implemented for `u64`
   = note: x86_64 has no atomic maximum or minimum
   = note: to accept a compare-exchange loop, call `update`
```

### One Field, One Instruction

An atomic packed struct lends each field as a place of its own, which one
instruction on the whole word changes alone:

```rust
use atomix::ordering::{AcqRel, Acquire, Release};
use atomix::{Atom, Atomic};

/// A resting quote: 32 bits of quantity, whether it may fill, then a byte of flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
struct Quote {
    quantity: u32,
    live: bool,
    flags: u8,
}

static QUOTE: Atomic<Quote> = Atomic::new(Quote { quantity: 300, live: false, flags: 0 });

fn main() {
    // `lock or`, or `ldset`: the bit `live` alone.
    QUOTE.fields().live.set(Release);
    // `lock btr`, or `ldclral`: the one thread that finds it live takes it off the book.
    assert!(QUOTE.fields().live.test_and_clear(AcqRel), "this thread took it");
    assert_eq!(QUOTE.load(Acquire).quantity, 300, "the quantity as it was");
}
```

### Tags in a Pointer

A pointer keeps small fields in the low bits its pointee's alignment leaves
clear, and keeps its provenance through every operation:

```rust
use core::ptr::NonNull;

use atomix::ordering::AcqRel;
use atomix::{Atom, Atomic, RangedU8};

/// A node of a stack, aligned to 8, so a pointer to one leaves three low bits clear.
#[repr(align(8))]
struct Node {
    value: u64,
}

/// A stack's head: the top node, then in its pointer's clear bits a version and whether the
/// stack is closed, in one word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
struct Head {
    top: Option<NonNull<Node>>,
    version: RangedU8<0, 3>,
    closed: bool,
}

static HEAD: Atomic<Head> = Atomic::new(Head { top: None, version: RangedU8::MIN, closed: false });

fn main() {
    // `lock bts`, or `ldsetal`, on the pointer: the one thread that finds the stack open closes it.
    assert!(!HEAD.fields().closed.test_and_set(AcqRel), "this thread closed the stack");
    assert_eq!(size_of_val(&HEAD), 8, "the top, its version and the flag in one pointer");
}
```

### Two Words in One Atomic

Two pointers, or one beside a counter or a slice's length, share a 16-byte
atomic, whose compare-exchange is `lock cmpxchg16b`, or `caspal`:

```rust
use core::ptr::NonNull;

use atomix::ordering::{AcqRel, Acquire};
use atomix::{Atom, Atomic};

/// A node of a stack.
struct Node {
    value: u64,
}

/// A stack's head: the top node, and a version that tells a top popped and pushed again from the
/// one read. No alignment leaves 64 bits clear, so the version takes a word of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
struct Head {
    top: Option<NonNull<Node>>,
    version: u64,
}

static HEAD: Atomic<Head> = Atomic::new(Head { top: None, version: 0 });

fn main() {
    let empty = Head { top: None, version: 0 };
    let next = Head { top: None, version: 1 };
    assert_eq!(HEAD.compare_exchange(empty, next, AcqRel, Acquire), Ok(empty), "both words, at once");
    assert_eq!(size_of_val(&HEAD), 16, "the top and its version in two words");
}
```

No Rust operation keeps a pointer's provenance through 16 bytes, so two words
expose each pointer's, the one place atomix does; one word keeps it strictly.

### The Derive

`#[derive(Atom)]` lays a struct or an enum out in one word or two as the crate
compiles, and refuses one that cannot fit, naming what it needs:

```text
error[E0080]: evaluation panicked: `app::Wide` needs 129 bits, but an atomic word holds at most 128: narrow a field, or split the value
```

It derives `AtomAdd`, `AtomOrd` and `AtomBitwise` too, for a newtype whose field
has them.

## Features

| Feature         | Adds                                                                       |
| --------------- | -------------------------------------------------------------------------- |
| `derive`        | `#[derive(Atom)]`: projections, tagged pointers; a newtype's capabilities  |
| `serde`         | serde's traits for an atomic, and a ranged integer, held to its range      |
| `zerocopy-08`   | zerocopy's traits for an atomic, as its validity allows, never `Immutable` |
| `bytemuck`      | `Zeroable` for an atomic, and the bit-pattern traits for a ranged integer  |
| `arbitrary`     | `Arbitrary` for an atomic, and a ranged integer, held to its range         |
| `arbitrary-int` | `Atom`, `AtomOrd`, `FieldBitwise` and `FieldAdd` for `UInt` and `Int`      |
| `deranged-05`   | `Atom`, `AtomOrd` and `From` with atomix's own for deranged 0.5's integers |
| `loom`          | loom's types under `--cfg loom`; nothing without the cfg                   |

None is on by default: `cargo add --git https://github.com/atomix-labs/atomix
atomix-rs --features derive,serde`.

## How It Compares

std's atomics hold integers, `bool` and pointers; portable-atomic carries them
to every target, with a lock where a target has no instruction; crossbeam-utils'
`AtomicCell` and the `atomic` crate's `Atomic` hold a value of any type, behind
a lock where none of std's atomics fits it. atomix holds any value its derive
can pack into one word or two, refuses as it compiles what a target cannot do in
one instruction, and needs a nightly Rust on four targets to do it.

## Documentation

- [The book][book]: what atomix is, and how to use it.
- The API: [`atomix-rs`][api-atomix-rs], [`atomix-core`][api-atomix-core] and
  [`atomix-derive`][api-atomix-derive].
- [CHANGELOG.md][changelog]: what changed in each release.

## Contributing

Issues and pull requests are welcome: read [CONTRIBUTING.md][contributing]
first, and report a vulnerability as [SECURITY.md][security] says.

## License

Either [the MIT License][mit] or [the Apache License, Version 2.0][apache], at
your option.

[platforms]: https://atomix-labs.github.io/atomix/api/atomix/#platforms
[book]: https://atomix-labs.github.io/atomix/
[api-atomix-rs]: https://atomix-labs.github.io/atomix/api/atomix/
[api-atomix-core]: https://atomix-labs.github.io/atomix/api/atomix_core/
[api-atomix-derive]: https://atomix-labs.github.io/atomix/api/atomix_derive/
[changelog]: https://github.com/atomix-labs/atomix/blob/main/CHANGELOG.md
[contributing]: https://github.com/atomix-labs/atomix/blob/main/CONTRIBUTING.md
[security]: https://github.com/atomix-labs/atomix/blob/main/SECURITY.md
[mit]: https://github.com/atomix-labs/atomix/blob/main/LICENSE-MIT
[apache]: https://github.com/atomix-labs/atomix/blob/main/LICENSE-APACHE
