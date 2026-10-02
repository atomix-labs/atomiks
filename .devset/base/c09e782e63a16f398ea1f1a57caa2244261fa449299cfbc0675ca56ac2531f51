# Sources

Read this before a rule of this skill needs backing, before citing a source in a
doc or a review, and before adapting a rule from somewhere else. It says what
each source holds, which rule it backs, where this skill departs from it, and
the notice owed for the text it adapts. Cite one of these, or an equally primary
source; open the page before linking it. A claim of speed is backed by a run,
never by a page: the pages say what the compiler and Cargo do, and a benchmark
says what it costs.

## Canon

| source                                                                                                           | says                                                                                                                                                                                                                                        | backs                                      |
| ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ |
| [The Rust Performance Book](https://nnethercote.github.io/perf-book/)                                            | benchmarking, profiling, build configuration, inlining ("non-transitive"; `#[cold]` for outlining), heap allocations (`Vec` growth, `clone_from`), type sizes, bounds checks                                                                | every reference                            |
| [The Cargo Book, Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)                              | each key; `lto = false` is "thin local LTO", none "if codegen units is 1 or opt-level is 0", `"off"` disables it; tests and benchmarks "ignore the `panic` setting"; overrides cannot set `lto`, `panic` or `rpath`; overrides and generics | `codegen.md`                               |
| [The Cargo Book, Configuration](https://doc.rust-lang.org/cargo/reference/config.html#buildrustflags)            | `RUSTFLAGS` and the matching `target` tables are "mutually exclusive sources", the first used; arrays join "with higher precedence items being placed later"                                                                                | `codegen.md`: the CPU floor                |
| [The Cargo Book, Targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#benchmarks)               | `[[bench]]`, `harness = false`; `#[bench]` is unstable                                                                                                                                                                                      | `measuring.md`                             |
| [The rustc book, Codegen options](https://doc.rust-lang.org/rustc/codegen-options/index.html#target-cpu)         | `target-cpu`, `target-feature`, `lto`, `codegen-units`, `panic`                                                                                                                                                                             | `codegen.md`                               |
| [The Rust Reference, Code generation attributes](https://doc.rust-lang.org/reference/attributes/codegen.html)    | `inline` "in every form … is a hint"; `cold` "suggests that the attributed function is unlikely to be called"                                                                                                                               | `codegen.md`                               |
| [The Rust Reference, Type layout](https://doc.rust-lang.org/reference/type-layout.html#the-rust-representation)  | the default representation promises no field order; `repr(C)` keeps it                                                                                                                                                                      | `data-layout.md`                           |
| [`core::hint`](https://doc.rust-lang.org/core/hint/index.html)                                                   | `black_box`, "best-effort", "generally be relied upon for benchmarking"; `cold_path`, stable since 1.95, "can actually decrease performance if the branch is called more than expected"; `spin_loop`                                        | `measuring.md`, `codegen.md`, `threads.md` |
| [`GlobalAlloc`](https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html)                                      | the global allocator's contract                                                                                                                                                                                                             | `allocation.md`: the counting test         |
| [clippy's lint list](https://rust-lang.github.io/rust-clippy/master/index.html)                                  | `inline_always`, `format_push_string`, `assigning_clones`, `large_stack_arrays`, `large_enum_variant`, `linkedlist`, `vec_box`, `map_entry`, `implicit_hasher`, `disallowed_methods`, `disallowed_macros`                                   | each rule "held by" one                    |
| [clippy's configuration](https://doc.rust-lang.org/clippy/lint_configuration.html)                               | `disallowed-methods` and `disallowed-macros`; `array-size-threshold` 16384; `enum-variant-size-threshold` 200; `stack-size-threshold` 512000                                                                                                | `allocation.md`, `data-layout.md`          |
| [crossbeam-utils, `CachePadded`](https://docs.rs/crossbeam-utils/latest/crossbeam_utils/struct.CachePadded.html) | 128 bytes on x86-64, since Intel's "spatial prefetcher is pulling pairs of 64-byte cache lines at a time"; on aarch64, since big.LITTLE's "big" cores have 128-byte lines; on powerpc64, its line                                           | `threads.md`                               |

## Measured Here

Each claim of cost or code these references state was checked on
nightly-2026-09-28, run on an AWS Graviton4, and compiled for x86-64 with that
nightly's `x86_64-unknown-linux-gnu` target where it names x86-64: the
disassembly of a benchmark built by the workspace's `bench` profile, and
assembly at `opt-level = 3`, for the inlining, bounds, vectorization, `#[cold]`,
`spin_loop` and atomic claims; a counting allocator for every allocation count;
`size_of` for every size; a run for the stack overflow, `panic = "abort"`, the
scheduler on isolated cores and the flags Cargo passes; and the benchmark
`measuring.md` shows, for its numbers. Where a cost is the machine's, a
reference says to measure it rather than state it.

## Where the Workspace Departs

| a source says                                                                                  | here                                                                                                                                                       |
| ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| rust-skills `perf-release-profile`, `opt-lto-release`: `panic = "abort"` and `strip = true`    | `release` keeps its symbols, so a profile names its functions; `abort` only under `strict`, for its reason                                                 |
| rust-skills `opt-target-cpu`: `target-cpu=native` in `.cargo/config.toml`                      | never in a checked-in file; a floor every machine meets, and a known machine's CPU added with `--config`                                                   |
| rust-skills `mem-smallvec`, `mem-arrayvec`, `mem-thinvec`, `mem-compact-string`                | not taught: reuse and capacity first; another crate's storage only where the repository has it and a run shows it pays                                     |
| rust-skills `perf-ahash`: ahash or FxHash where DoS resistance is not needed                   | `data-layout.md`: the default hasher for keys from outside; the repository's faster one for keys the program makes, where a profile and a run show it pays |
| rust-skills `mem-arena-allocator`: an arena for each request                                   | not taught; a buffer the hot path reuses                                                                                                                   |
| rust-skills `perf-profile-first`: `cargo flamegraph`                                           | a profiler on the `profiling` build, whichever the machine has                                                                                             |
| rust-skills `perf-black-box-bench`, `opt-inline-always-rare`: criterion                        | a `harness = false` benchmark of the crate's own; criterion or divan where the repository has them                                                         |
| rust-skills `opt-inline-always-rare`: `#[inline(always)]` "proven by profiling"                | only where a call would defeat the function, under an `#[expect(clippy::inline_always, …)]` naming why                                                     |
| rust-skills `opt-likely-hint`: an early return and the order of match arms as hints            | no hint to rely on: reordering left one function's code as it was, and changed another's where `cold_path` did not                                         |
| rust-skills `opt-pgo-profile`, `opt-simd-portable`: profile-guided optimization, portable SIMD | not taught                                                                                                                                                 |
| rust-skills `mem-write-over-format`: `write!(out, "…\n").unwrap()`                             | `writeln!`, since `clippy::write_with_newline` refuses the first, and `?`                                                                                  |

## Corrected Here

Claims of rust-skills that fail on the pinned toolchain or against the sources
above, and what this skill says instead:

- A generic function needs no `#[inline]` to be inlined across crates, as
  `opt-inline-small` and `opt-inline-always-rare` say it does: each caller
  compiles its own copy, and inlined `widen::<u16>` from another crate with no
  attribute, in an incremental build too. rustc also offers other crates a small
  function that, once its own calls are inlined, calls nothing, in a build that
  is not incremental; one that still calls another stays a call without
  `#[inline]` or LTO.
- `core::hint::cold_path` is stable since Rust 1.95, which `opt-cold-unlikely`
  leaves out; `likely` and `unlikely` are unstable (E0658 on the pinned
  nightly).
- `#[cold]` alone does not keep a function out of line: a small `#[cold]`
  function was inlined, and its result computed on every call beside the hot
  one.
- `vec![]` does not allocate, as `lint-warn-perf` says it does: its capacity is
  0, and a counting allocator counts no call.
- `Option<Vec<T>>` is the size of `Vec<T>`, 24 bytes, since a `Vec`'s pointer is
  never null; `mem-thinvec` says it gets no niche.
- A struct of Rust's default layout is reordered: `mem-smaller-integers`'s `u8`,
  `u64`, `u8` is 16 bytes, not the 24 it asserts, which only `repr(C)` gives.
- In Cargo, `lto = false` is thin local LTO and `lto = "off"` disables it;
  `opt-lto-release` labels `"off"` thin local.
- The default x86-64 target is SSE2, the first x86-64 level: `rustc --print cfg`
  lists `fxsr`, `sse`, `sse2` and `x87` alone. `opt-target-cpu` calls it
  "roughly Sandy Bridge era", and its
  `[target.x86_64-unknown-linux-gnu.deployment]` table makes Cargo refuse the
  configuration.
- `target-cpu=native` in a checked-in `.cargo/config.toml`, `opt-target-cpu`'s
  good example, builds each machine's binary for that machine alone, CI's
  included.
- `collect_into` is unstable (E0658, `iter_collect_into`); v1.0.0's
  `perf-collect-into` says it was stabilized in 1.83, and v1.5.1 fixes it.
- A float sum is not vectorized: `iter().sum::<f32>()` compiles to one scalar
  addition after another, where `opt-simd-portable` says it may vectorize. A sum
  over lanes is.
- `for i in 0..a.len()` over `a[i]` has no bounds check, where
  `perf-iter-over-index` and `opt-bounds-check` say each turn checks; an index
  into a second slice was checked once, before the loop.

## Adapted Text

Rules of this skill adapt rules of
[leonardomso/rust-skills](https://github.com/leonardomso/rust-skills), at
v1.0.0, the copy sockudo vendors, and at v1.5.1, rewritten for this workspace
and checked by compiling and running: `mem-with-capacity`,
`mem-reuse-collections`, `mem-clone-from`, `mem-avoid-format`,
`mem-write-over-format`, `mem-zero-copy`, `mem-box-large-variant`,
`mem-boxed-slice`, `mem-smaller-integers`, `mem-assert-type-size`,
`opt-inline-small`, `opt-inline-always-rare`, `opt-inline-never-cold`,
`opt-cold-unlikely`, `opt-bounds-check`, `opt-cache-friendly`,
`opt-lto-release`, `opt-codegen-units`, `opt-target-cpu`, `perf-profile-first`,
`perf-black-box-bench`, `perf-release-profile`, `perf-ahash` and
`perf-entry-api`.

Both versions carry the same notice:

```text
MIT License

Copyright (c) 2025 Leonardo Maldonado

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
