# `atomix-derive`

The derives of [atomix][project]: `Atom`, which lays a struct or an enum out in
one atomic word or two as the crate compiles, and refuses one that cannot fit;
and `AtomAdd`, `AtomOrd` and `AtomBitwise`, the read-modify-writes a newtype
takes from its field. The code each writes names `atomix`, so depend on
`atomix-rs` with its `derive` feature, which re-exports them, and not on this
crate.

## Install

```sh
cargo add --git https://github.com/atomix-labs/atomix atomix-rs --features derive
```

The package is `atomix-rs`, and the library it adds `atomix`. It needs a nightly
Rust, `nightly-2026-09-28` or newer, and builds for Linux and macOS, on
`aarch64` and `x86_64`.

## Quick Start

```rust
use core::num::NonZero;

use atomix::ordering::{Acquire, Release};
use atomix::{Atom, Atomic};

/// An owner's id, never zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
struct OwnerId(NonZero<u64>);

// The id's validity is its field's, so `None` takes zero, which no id is.
static OWNER: Atomic<Option<OwnerId>> = Atomic::new(None);

fn main() {
    let owner = OwnerId(NonZero::new(7).expect("7 is not zero"));
    OWNER.store(Some(owner), Release);
    assert_eq!(OWNER.load(Acquire), Some(owner), "the owner stored");
    assert_eq!(None::<OwnerId>.to_repr(), 0, "with `None` at zero");
}
```

## Documentation

- [The book][book]: what atomix is, and how to use it.
- [The API][api], with an example for each shape the derive takes.

## License

Either [the MIT License][mit] or [the Apache License, Version 2.0][apache], at
your option.

[project]: https://github.com/atomix-labs/atomix
[book]: https://atomix-labs.github.io/atomix/
[api]: https://atomix-labs.github.io/atomix/api/atomix_derive/
[mit]: https://github.com/atomix-labs/atomix/blob/main/LICENSE-MIT
[apache]: https://github.com/atomix-labs/atomix/blob/main/LICENSE-APACHE
