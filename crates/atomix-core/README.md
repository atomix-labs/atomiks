# `atomix-core`

The typed atomic of [atomix][project], the values it holds, and the orderings
and primitives under it: `Atomic<T>` and its aliases, the `Atom` trait and its
impls for core's types, the ranged integers, the fences, and the loom-shaped
cell. It is `no_std`. Most users want [`atomix-rs`][project], which re-exports
all of it and adds `#[derive(Atom)]`.

## Install

```sh
cargo add --git https://github.com/atomix-labs/atomix atomix-core
```

`atomix-core` needs a nightly Rust, `nightly-2026-09-28` or newer, and builds
for `aarch64` and `x86_64`, little-endian with 64-bit pointers, on any OS, and
for Windows' `arm64ec`; CI tests it on Linux, macOS and Windows.

## Quick Start

```rust
use core::num::NonZero;

use atomix_core::ordering::{Acquire, Relaxed, Release};
use atomix_core::{Atomic, AtomicU64};

static NEXT: AtomicU64 = AtomicU64::new(1);
static OWNER: Atomic<Option<NonZero<u64>>> = Atomic::new(None);

fn main() {
    OWNER.store(NonZero::new(NEXT.fetch_add(1, Relaxed)), Release);
    assert_eq!(OWNER.load(Acquire), NonZero::new(1), "the first id taken");
}
```

## Features

| Feature         | Adds                                                                       |
| --------------- | -------------------------------------------------------------------------- |
| `serde`         | serde's traits for an atomic, and a ranged integer, held to its range      |
| `zerocopy-08`   | zerocopy's traits for an atomic, as its validity allows, never `Immutable` |
| `bytemuck`      | `Zeroable` for an atomic, and the bit-pattern traits for a ranged integer  |
| `arbitrary`     | `Arbitrary` for an atomic, and a ranged integer, held to its range         |
| `arbitrary-int` | `Atom`, `AtomOrd`, `FieldBitwise` and `FieldAdd` for `UInt` and `Int`      |
| `deranged-05`   | `Atom`, `AtomOrd` and `From` with atomix's own for deranged 0.5's integers |
| `loom`          | loom's types under `--cfg loom`; nothing without the cfg                   |

None is on by default: `cargo add --git https://github.com/atomix-labs/atomix
atomix-core --features serde`.

## Documentation

- [The book][book]: what atomix is, and how to use it.
- [The API][api].
- [CHANGELOG.md][changelog]: what changed in each release.

## License

Either [the MIT License][mit] or [the Apache License, Version 2.0][apache], at
your option.

[project]: https://github.com/atomix-labs/atomix
[book]: https://atomix-labs.github.io/atomix/
[api]: https://atomix-labs.github.io/atomix/api/atomix_core/
[changelog]: https://github.com/atomix-labs/atomix/blob/main/CHANGELOG.md
[mit]: https://github.com/atomix-labs/atomix/blob/main/LICENSE-MIT
[apache]: https://github.com/atomix-labs/atomix/blob/main/LICENSE-APACHE
