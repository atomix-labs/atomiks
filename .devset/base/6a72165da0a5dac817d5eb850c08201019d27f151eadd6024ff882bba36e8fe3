# Code Generation and the Build Profiles

Read this before an inline attribute, `#[cold]` or a hint, before a bounds check
or a loop meant to vectorize on a hot path, before choosing a target CPU or its
features, and before a build profile, LTO, codegen units or `panic`. The
compiler inlines, lays out and vectorizes better than a hand can in most code;
what it cannot see is which calls cross a crate, which paths are rare, what a
float sum may reorder and which machine runs the code. Those are told here, and
nothing else.

## Mark `#[inline]` a Small Public Function That Calls Another

A caller in another crate reaches a non-generic function's body only if it is
offered: by `#[inline]`, by LTO, or by rustc itself. rustc offers a small
function that, once its own calls are inlined, calls nothing, but only in a
build that is not incremental; a generic function each caller compiles already.
So `#[inline]` matters in three places: a small function that still calls
another, as one that keeps a rare path out of line does; any function, in an
incremental build; and a published crate, whose users build without this
workspace's profiles. The workspace's own `release`, `bench` and `profiling`
builds inline across crates by fat LTO without it. It does not reach through a
call: a function inlined still calls what it calls, unless that is offered too.

```rust
#[derive(Debug, Default)]
pub struct Row {
    tiles: Vec<u8>,
}

impl Row {
    // Bad: it calls `spill`, so another crate's call stays a call in a build without LTO.
    pub fn paint(&mut self, index: usize, tile: u8) {
        match self.tiles.get_mut(index) {
            Some(slot) => *slot = tile,
            None => self.spill(index, tile),
        }
    }

    #[cold]
    #[inline(never)]
    fn spill(&mut self, index: usize, tile: u8) {
        self.tiles.resize(index.saturating_add(1), 0);
        if let Some(slot) = self.tiles.get_mut(index) {
            *slot = tile;
        }
    }
}
```

```rust
#[derive(Debug, Default)]
pub struct Row {
    tiles: Vec<u8>,
}

impl Row {
    #[inline]
    pub fn paint(&mut self, index: usize, tile: u8) {
        match self.tiles.get_mut(index) {
            Some(slot) => *slot = tile,
            None => self.spill(index, tile),
        }
    }

    #[cold]
    #[inline(never)]
    fn spill(&mut self, index: usize, tile: u8) {
        self.tiles.resize(index.saturating_add(1), 0);
        if let Some(slot) = self.tiles.get_mut(index) {
            *slot = tile;
        }
    }
}
```

Held by review.

## `#[inline(always)]` Only Where a Call Would Defeat the Function

`#[inline(always)]` overrides the compiler's judgement at every call site, and
each forced copy grows the code around it, so it is written only where a call
would defeat what the function is for: a barrier and the read it orders, which
no call may sit between, or a body of an instruction or two that a call would
dwarf, measured. Its reason says which, in an `#[expect]` of the lint that
refuses it. It is a hint too, and the compiler may still decline.

```rust,compile_fail
// fails: clippy::inline_always
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub column: u16,
    pub row: u16,
}

// Bad: forced at every call, for no reason it can name.
#[inline(always)]
#[must_use]
pub fn flat_index(position: Position, columns: u16) -> u32 {
    u32::from(position.row)
        .wrapping_mul(u32::from(columns))
        .wrapping_add(u32::from(position.column))
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub column: u16,
    pub row: u16,
}

#[inline(always)]
#[expect(clippy::inline_always, reason = "run for every tile painted; a call would dwarf its two instructions")]
#[must_use]
pub fn flat_index(position: Position, columns: u16) -> u32 {
    u32::from(position.row)
        .wrapping_mul(u32::from(columns))
        .wrapping_add(u32::from(position.column))
}
```

Held by `clippy::inline_always`.

## `#[inline(never)]` Keeps a Function a Function

Some code must stay a call: the body a benchmark times, so the loop around it
cannot be merged with it; a probe whose assembly is read, so it can be found;
the rare half of a function split from its hot half, so the hot call site
inlines only what it runs. `#[inline(never)]` says so, and is a hint as the
others are.

```rust
use core::hint::black_box;

pub fn drive<T, F: FnMut() -> T>(rounds: u64, operation: &mut F) {
    // Bad: `drive` may be inlined into each caller and merged with its loop.
    for _ in 0..rounds {
        black_box(operation());
    }
}
```

```rust
use core::hint::black_box;

#[inline(never)]
pub fn drive<T, F: FnMut() -> T>(rounds: u64, operation: &mut F) {
    for _ in 0..rounds {
        black_box(operation());
    }
}
```

Held by review.

## Move a Rare Path's Work into a `#[cold]` Function, and Mark a Rare Branch `cold_path`

`#[cold]` says a function is unlikely to be called, so the compiler may lay its
call sites out of the hot path's way; `core::hint::cold_path()`, stable since
Rust 1.95, says the same of the branch it is called in. So a hot function keeps
its rare path's work, a growth, a refusal built with its context, in a function
of its own, marked `#[cold]` and `#[inline(never)]`, and the hot function stays
small enough to inline where it is called. `#[cold]` alone does not keep a small
function out of line: a small `#[cold]` function is inlined like any other, and
its work may be done on every call, computed beside the hot result and one of
them chosen. The order of branches and arms is no hint to rely on: reordering
one left its code as it was, and changed another's where `cold_path` did not;
`core::hint::likely` and `unlikely` are unstable. Each hint's effect is
measured: a path marked cold that runs often is made slower.

```rust
#[derive(Debug, Default)]
pub struct Row {
    tiles: Vec<u8>,
}

impl Row {
    #[inline]
    pub fn paint(&mut self, index: usize, tile: u8) {
        if let Some(slot) = self.tiles.get_mut(index) {
            *slot = tile;
            return;
        }
        // Bad: the rare growth written into the hot function, with nothing to say it is rare.
        self.tiles.resize(index.saturating_add(1), 0);
        if let Some(slot) = self.tiles.get_mut(index) {
            *slot = tile;
        }
    }
}
```

```rust
use core::hint::cold_path;

#[derive(Debug, Default)]
pub struct Row {
    tiles: Vec<u8>,
}

impl Row {
    #[inline]
    pub fn paint(&mut self, index: usize, tile: u8) {
        match self.tiles.get_mut(index) {
            Some(slot) => *slot = tile,
            None => self.spill(index, tile),
        }
    }

    #[cold]
    #[inline(never)]
    fn spill(&mut self, index: usize, tile: u8) {
        self.tiles.resize(index.saturating_add(1), 0);
        if let Some(slot) = self.tiles.get_mut(index) {
            *slot = tile;
        }
    }
}

#[must_use]
pub fn weight(tile: u8) -> u32 {
    match tile {
        0 => 0,
        1..=15 => 1,
        _ => {
            cold_path();
            u32::from(tile).saturating_mul(3)
        }
    }
}
```

Held by review.

## Let the Loop's Shape Prove Its Bounds

A bounds check the compiler can prove is free: `for index in 0..row.len()` over
`row[index]` has none. An index into a second slice by the first's length stays
in the code, though the compiler often splits the loop so it checks once. A
`get` with a fallback, `overlay.get(index).unwrap_or(0)`, must carry on past the
end, so it is a branch on every element, and the loop stays scalar. So a hot
loop takes a shape that proves its bounds: it iterates, zips the slices, or
slices each to one length before the loop, which leaves no check at all.

```rust
#[must_use]
pub fn blend(base: &[u32], overlay: &[u32]) -> u32 {
    let mut sum = 0_u32;
    for (index, tile) in base.iter().enumerate() {
        // Bad: each turn checks `overlay`, not known to be as long, and nothing vectorizes.
        sum = sum.wrapping_add(tile.wrapping_mul(overlay.get(index).copied().unwrap_or(0)));
    }
    sum
}
```

```rust
#[must_use]
pub fn blend(base: &[u32], overlay: &[u32]) -> u32 {
    base.iter().zip(overlay).fold(0, |sum, (tile, top)| sum.wrapping_add(tile.wrapping_mul(*top)))
}
```

Held by review, and by the disassembly of the binary that ships, as
`measuring.md` shows.

## A Float Sum Keeps Its Order, and Does Not Vectorize

Adding floats in another order can change the sum, so the compiler adds them in
the order written, one after another, where an integer sum is split across
vector lanes. Where the order does not matter to the caller, a sum into several
lanes, then of the lanes, lets the compiler vectorize it. Its result may differ
from the ordered sum: a little where the values share a sign, and by much more
where large values of both signs cancel, as `1e8`, `-1e8` and 4,094 ones do,
which sum to 4,094 in order and 3,072 over eight lanes. A caller that needs the
ordered sum bit for bit keeps the order.

```rust
#[must_use]
pub fn brightness(row: &[f32]) -> f32 {
    // Bad: one addition after another, in order, over the whole row.
    row.iter().sum()
}
```

```rust
#[must_use]
pub fn brightness(row: &[f32]) -> f32 {
    let (chunks, rest) = row.as_chunks::<8>();
    let mut lanes = [0.0_f32; 8];
    for chunk in chunks {
        for (lane, value) in lanes.iter_mut().zip(chunk) {
            *lane += value;
        }
    }
    lanes.iter().sum::<f32>() + rest.iter().sum::<f32>()
}
```

Held by review.

## Build for the CPUs That Run the Code, Never `target-cpu=native` in the Workspace

A build uses only the instructions its target CPU has. rustc's default for
x86-64 is its first level, SSE2; for Arm Linux, the first 64-bit Arm, where an
atomic read-modify-write calls a helper that picks its instruction at run time,
and `+lse` makes it one instruction. A binary built for more than the machine it
runs on dies on the first instruction that machine lacks, so a checked-in
configuration never says `target-cpu=native`, which means whatever machine built
it, CI's included. A build for one known machine names its CPU for that build
alone, with `--config`; `rustc --print cfg -C target-cpu=<cpu>` lists what a CPU
enables.

```text
# Bad: .cargo/config.toml, checked in; every machine that builds it builds for itself.
[build]
rustflags = ["-C", "target-cpu=native"]
```

```text
# A benchmark for the Graviton4 that runs it, its CPU named for this build alone:
cargo bench -p tiles --config 'target."cfg(all())".rustflags=["-C","target-cpu=neoverse-v2"]'
```

Held by review.

The workspace's `.cargo/config.toml` sets a floor for each architecture:
`x86-64-v2`, `+crc` on Arm, and the first Apple silicon. `RUSTFLAGS` replaces
those flags whole, which a build for the machine it runs on may do:
`RUSTFLAGS="-C target-cpu=native"`. `--config` adds to them, and the last
`target-cpu` wins: on the floor's own key, Cargo joins the added flags after the
floor's; in another table they may come first, so on x86-64, whose floor names a
CPU, a CPU added there can lose to `x86-64-v2`.

```text
# A build for Zen 4, on the x86-64 floor's own key, so its CPU comes last:
cargo build --release --config 'target."cfg(target_arch = \"x86_64\")".rustflags=["-C","target-cpu=znver4"]'
```

## Measure the Build That Ships

What ships is `release`, and a number from another build is a number about other
code. `cargo bench` builds with `bench`, `release`'s settings, and `cargo build
--profile profiling` with its optimizations. `dev` builds the workspace's own
code at `opt-level = 0`, unoptimized, so a dev number, or a dev profile's hot
spot, says nothing about the release; `release-fast` builds with thin LTO and
sixteen codegen units, so its numbers are not the release's either.

```text
# Bad: a dev build timed, and its number reported.
cargo run -p tiles --bin paint
```

```text
cargo bench -p tiles --bench paint
```

Held by review.

## The Profiles, and Why Each Is Set So

The workspace's `Cargo.toml` holds six profiles. `release` builds at
`opt-level = 3` with fat LTO, which optimizes across every crate in the graph,
and one codegen unit, which lets it optimize across the whole crate: the fastest
code, and the slowest build. It has no debug assertions and no overflow checks,
keeps its symbols, `strip = false`, so a profiler can name its functions, and
compiles each crate whole, `incremental = false`. `bench` has `release`'s
settings, though Cargo builds a benchmark to unwind on a panic. `profiling` is
`release` with full debug info, packed beside the binary, `split-debuginfo =
"packed"`, and nothing stripped. `release-fast` is `release` with thin LTO,
sixteen units and incremental builds, for a quicker build while iterating. `dev`
builds the workspace's code at `opt-level = 0` with debug assertions and
overflow checks, and its dependencies at `opt-level = 3`,
`[profile.dev.package."*"]`, so their code runs optimized in tests and debug
builds; a generic function of a dependency may be compiled where it is used, at
the using crate's level, `0`. `test` is `dev` with sixteen codegen units.
Without `strict`, a panic unwinds in every profile; `strict` sets `panic =
"abort"`, and a package override cannot set `panic` at all.

`lto = false`, as `dev` has it, is not no LTO: it is thin LTO within each crate,
across its codegen units, and none at all where a crate has one unit or
`opt-level = 0`; `lto = "off"` disables it.

```toml
# Bad: `false` read as "no LTO", and a release left at 16 units.
[profile.release]
lto           = false
codegen-units = 16
```

```toml
[profile.release]
opt-level     = 3
lto           = "fat"
codegen-units = 1
```

Held by cargo-profiles, whose keys these are.

## A Smaller Binary Is a Profile of Its Own

`release` is built for speed, and keeps its symbols and the standard library's
debug info so a profile can name what it runs; a binary that must be small is a
profile of its own that `inherits` `release`: `strip = true` drops the symbols
and that debug info, and `opt-level = "z"` trades speed for size. Each is
measured, for size and for speed: for a small command line, `release` built
2,148,664 bytes, `strip = true` 398,760, and `opt-level = "z"` with it 333,224.

```toml
# Bad: `release` changed for size, which slows every build that ships and blinds the profiler.
[profile.release]
opt-level = "z"
strip     = true
```

```toml
[profile.small]
inherits  = "release"
opt-level = "z"
strip     = true
```

Held by review.

## A Crate's Own Setting Is a Key of Its Own

The profiles' keys are the cargo-profiles profile's, so a change to one is
drift, which `devset status` reports and `devset apply --force` undoes. A crate
that needs a setting of its own takes a key the profile does not own, which
stays the repository's: `[profile.release.package.<crate>]`, which may set
`opt-level`, `codegen-units` or `debug`, though never `lto`, `panic` or `rpath`,
which Cargo refuses there; and a build of its own is a profile of its own that
`inherits` one of the six.

How a managed key is changed is `using-devset`'s.

```toml
# Bad: an owned key edited, which devset reports as drift.
[profile.release]
opt-level = 2
```

```toml
[profile.release.package.tiles-atlas]
opt-level = "s"

[profile.bench-native]
inherits = "bench"
```

Held by devset, which reports a changed key as drift.

## Under `strict`, a Panic Ends the Process

`strict` sets `panic = "abort"` in `release` and `dev`, and so in every profile
that inherits them. Under `strict`, library code panics only for a broken
invariant, and a process whose invariant broke stops before it acts on what the
broken invariant left: the panic prints its message and the process aborts, with
`SIGABRT`. So nothing unwinds: no destructor runs, and a `BufWriter` is not
flushed; `catch_unwind` catches nothing; a thread that panics ends every thread.
A failure a caller must survive is an error it is handed, never a panic it
catches. The code the workspace compiles carries no unwinding paths, where the
prebuilt standard library keeps its own. Tests and benchmarks still unwind,
their dependencies with them, since Cargo builds them with `unwind` whatever the
profile says, so a `#[should_panic]` test works as it does elsewhere, and `cargo
bench` measures code built to unwind: a gain that could rest on drops or panics
is confirmed on a `--release` binary or example, whose run's profile line says
so.

```rust
use core::panic::AssertUnwindSafe;
use std::panic;

#[must_use]
pub fn try_paint(paint: fn(&mut [u8]), row: &mut [u8]) -> bool {
    // Bad: under `abort`, a panic in `paint` ends the process, so this never returns `false`.
    panic::catch_unwind(AssertUnwindSafe(|| paint(row))).is_ok()
}
```

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("paint error: tile {tile} is past the palette of {colors}")]
pub struct PaintError {
    pub tile: u8,
    pub colors: u8,
}

pub fn paint(row: &mut [u8], tile: u8, colors: u8) -> Result<(), PaintError> {
    if tile >= colors {
        return Err(PaintError { tile, colors });
    }
    row.fill(tile);
    Ok(())
}
```

Held by the profile.
