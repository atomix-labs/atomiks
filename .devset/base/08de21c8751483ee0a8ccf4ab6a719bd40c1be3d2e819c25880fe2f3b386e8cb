---
name: tuning-rust-performance
description: Use when making Rust code faster or smaller, or judging whether a change did; when writing a benchmark or profiling a hot path; when an allocation, a buffer, a `format!` or a `clone` sits in a loop; when choosing `#[inline]`, `#[inline(always)]`, `#[cold]` or `cold_path`, or reading bounds checks or vectorization in the assembly; when a type's size, field order, `Option` or large enum variant matters, or a cache line is shared between threads; when a thread spins, busy-polls, is pinned or reads the clock in a hot loop; when choosing a `HashMap`'s hasher or lookups; when choosing LTO, codegen units, a build profile, `panic = "abort"` or target CPU features. Covers measuring, allocation, code generation, data layout, what threads cost each other, and the workspace's build profiles. Not for whether the code is correct, which writing-rust covers.
---

# Tuning Rust Performance

How Rust is made fast in this workspace: measured before a change and after,
with the numbers kept; allocated before the hot path and reused in it; written
so the compiler can inline, lay out and vectorize it, and told only what it
cannot see; laid out so its data fills the cache, with threads that do not fight
over a line; and built with the profile that ships, so what is measured is what
runs. The rules below are the whole of it, each with its reason. The references
hold each rule's why, a bad and a good example that compile under the
workspace's lints, and what holds the rule. A cost this skill states was
measured; where a cost is the machine's, it says to measure it.

The examples leave their docs out to stay short, and compile with the lints that
ask for docs off; real code writes them, and a doc that claims a speed cites its
number and its run.

Under `strict`, real code also documents every item.

## Rules

### Measuring

1. **Measure before a change and after, one change at a time, and keep it only
   if the number moved: the baseline and the change run twice each, both runs of
   the change past both of the baseline, the way claimed, by more than the
   baseline's spread and by more than 1%**, since time is rarely spent where it
   is guessed to be, and a faster-looking form that is not faster only costs its
   reader.
2. **Find where the time goes with a profiler on the `profiling` build**, which
   is `release` with full debug info, so the profile names the functions and
   lines of the code that ships.
3. **A benchmark is a `harness = false` target in `benches/`, run by `cargo
   bench`**, which builds it as `release` is built; criterion or divan only
   where the repository has them, and never a timing in a `#[test]`, which runs
   at `opt-level = 0`.
4. **What a benchmark measures goes through `core::hint::black_box`, in and
   out**, since the optimizer deletes work whose result is unused and folds work
   on a constant.
5. **Warm up, repeat, and report the distribution, p50, p99 and the maximum,
   beside the harness's own floor, on a quiet core pinned with `taskset`**,
   since a mean hides the tail; on isolated cores each thread pins itself, since
   the scheduler spreads none across them. A call within a few times the floor
   is timed in batches, the round divided by the batch, since alone it reads as
   the floor.
6. **Commit each run a change rests on,
   `benches/results/<time>-<commit>-<name>/`, with the machine, kernel,
   toolchain, profile, flags and parameters**, so a later change is judged under
   the same conditions.
7. **A doc that claims a speed cites its number and its committed run**, and a
   claim about generated code, a call inlined or a check gone, is read in the
   disassembled binary that ships, since a benchmark says how fast and not why,
   and under fat LTO the library's `--emit asm` is the code before LTO.

### Allocation

1. **Allocate before the hot path and reuse after**, since a buffer cleared each
   turn keeps its capacity, and a loop that reuses it allocates nothing.
2. **Size a collection when its count is known, `with_capacity` or `reserve`**,
   since growth by doubling allocates and copies: nine allocations for a
   thousand `u32`s pushed one at a time.
3. **An empty `Vec`, `String` or `HashMap` allocates nothing, `vec![]` too**, so
   an empty one needs no `Option` or laziness; the first push allocates.
4. **Format into a buffer with `write!`, never `push_str(&format!(…))`**, since
   each `format!` allocates a string.
5. **Clone into a value held with `clone_from`**, which reuses its allocation; a
   derived `Clone`'s `clone_from` does not, so clone its fields.
6. **Borrow from the input rather than copy it**: a parsed value holds `&'a str`
   or `&'a [u8]` of its buffer, since copying each field allocates each field.
7. **Make a large value where it lives, `vec![0; length].into_boxed_slice()`**,
   since `Box::new([0; N])` builds the array on the stack first, and overflows
   it in a build that does not optimize the copy away; `Box::new_zeroed()`
   serves one value, with an unsafe `assume_init`.
8. **A crate that must not allocate lists what allocates in its own
   `clippy.toml`, each with a reason**, which replaces the workspace's, so it
   repeats every key of it.
9. **A path that must not allocate is proven by a test under a counting
   allocator**, since a list sees only the calls written in the crate.

Under `strict`, a library that needs no heap has no `extern crate alloc`, and
cannot allocate at all.

### Code Generation

1. **Mark `#[inline]` a small public function that calls another**, since a
   build without LTO reaches its body from another crate only so. rustc offers a
   small function that calls nothing on its own, outside incremental builds, and
   each caller compiles a generic one; the workspace's fat-LTO builds need none,
   but an incremental build and a published crate's users do.
2. **`#[inline(always)]` only where a call would defeat the function, under an
   `#[expect(clippy::inline_always, reason = "…")]`**, since every forced copy
   grows the code around it.
3. **`#[inline(never)]` keeps a function a function**: the body a benchmark
   times, a probe whose assembly is read, the rare half split from a hot one.
4. **Move a rare path's work into a `#[cold]` `#[inline(never)]` function, and
   mark a rare branch `core::hint::cold_path()`**, since a small cold function
   is still inlined, its work on the hot path; `likely` and `unlikely` are
   unstable, branch order is no hint, and a hint's effect is measured.
5. **Let a hot loop's shape prove its bounds: iterate, zip, or slice to one
   length first**, since a `get` with a fallback is a branch on every element,
   and keeps the loop scalar.
6. **Sum floats into several lanes where the order does not matter**, since the
   compiler keeps a float sum in order, one addition after another, where an
   integer sum vectorizes; a caller that needs the ordered sum bit for bit keeps
   the order.
7. **Build for the CPUs that run the code, never `target-cpu=native` in a
   checked-in file**, since a binary dies on the first instruction its machine
   lacks.

The workspace's `.cargo/config.toml` sets a CPU floor for each architecture:
`RUSTFLAGS` replaces it, and `--config` on the floor's own key adds to it.

### The Build Profiles

1. **Measure the build that ships, `cargo bench` or `--release`, never a dev
   build**, whose own code is unoptimized, and never `release-fast`'s numbers as
   the release's.
2. **`release` is `opt-level = 3`, fat LTO and one codegen unit, without debug
   assertions or overflow checks, its symbols kept**: the fastest code and the
   slowest build; `bench` has its settings, `profiling` adds full debug info,
   and `release-fast` trades thin LTO and sixteen units for a quicker build. A
   panic unwinds in each unless `strict` sets `panic = "abort"`, which a package
   override cannot set.
3. **`dev` builds the workspace's code at `opt-level = 0` and its dependencies
   at 3**, so they run optimized in tests; its `lto = false` is thin LTO within
   a crate, none at `opt-level = 0`, and `"off"` is none anywhere.
4. **The profiles' keys are cargo-profiles'; a crate's own setting is a key of
   its own, `[profile.release.package.<crate>]`, or a profile that `inherits`**,
   since a changed owned key is drift, which devset reports; a smaller binary is
   such a profile, with `strip = true` and `opt-level = "z"`, each measured.
5. **Under `strict`, a panic ends the process, `panic = "abort"` in `release`
   and `dev`**, so a broken invariant stops the process before it acts on what
   it left: no destructor runs, `catch_unwind` catches nothing, and a failure a
   caller must survive is an error, never a panic. Tests and benchmarks still
   unwind, since Cargo builds them and their dependencies so: `cargo bench`
   measures code built to unwind, and a gain that could rest on drops or panics
   is confirmed on a `--release` binary or example, as its run's profile line
   says.

### Data Layout

1. **Assert a hot type's size beside it, `const _: () =
   assert!(size_of::<T>() == N, "…")`**, so a change that grows it fails the
   build.
2. **The compiler orders a struct's fields, unless `#[repr(C)]` fixes them**, so
   reordering a default one saves nothing, and a `repr(C)` one puts its widest
   field first.
3. **An integer is as wide as its domain, `u16` for a column**, since the width
   multiplies by every copy; widen with `From`, narrow at the boundary with
   `TryFrom`.
4. **An `Option` of a type with a spare value costs nothing**: `Option<Vec<T>>`
   is the size of `Vec<T>`, and an id never zero is a `NonZeroU32`, where
   `Option<u32>` doubles.
5. **Box a large, rarely made variant**, since an enum is as large as its
   largest variant.
6. **Data that never grows is `Box<[T]>` or `Box<str>`**, a word smaller than a
   `Vec` or a `String`.
7. **Store values side by side, a `Vec<T>` or a `VecDeque<T>`, never a linked
   list or a `Vec<Box<T>>` of a small `T`**, since each pointer followed is a
   load from anywhere.
8. **Keys from outside the program keep std's hasher, which resists chosen
   collisions; a faster one from the repository only for keys the program makes,
   where a profile shows hashing hot and a run shows it pays**; a map keyed by a
   small dense id is a `Vec`, and `entry` looks a key up once.
9. **What a hot loop reads together is stored together, apart from what it
   skips**, since the skipped fields still fill its cache; measured, since a
   loop that reads them all gains nothing.

### Threads

1. **Values written by different threads live on different cache lines,
   `#[repr(align(128))]` where the code runs on more than one kind of machine**,
   since a line moves between cores on each write, and a line is the target's,
   fetched in pairs on x86-64.
2. **A count many threads add to is added once a thread**, since threads take
   turns at one atomic's line whatever they write.
3. **Block, and spin only where a wake-up is measured too slow, on a core of its
   own: `core::hint::spin_loop()` each turn, for a bound measured against that
   wake-up, then `thread::park` or a `Condvar`**, never `yield_now`, which on a
   core with nothing else to run returns at once; std's `Mutex` spins briefly
   already.
4. **A busy poll runs only on a core of its own, pinned and isolated**, since it
   uses the whole core whether or not input comes.
5. **A hot thread is pinned, and each thread pins itself**, since a moved thread
   loses its caches, and the scheduler spreads no thread across isolated cores.
6. **A hot loop reads the clock once a turn and passes the instant down**, so
   each decision sees one now and the loop pays one read.
7. **A hot thread's memory is written once before its loop, locked, and on huge
   pages where it is large and read at random**, since a first touch traps into
   the kernel.

## Steps

Read each reference a step names, whole, before changing the code.

1. **Making code faster**: `references/measuring.md` first. Profile the
   `profiling` build; find the hot function; write or run a benchmark of it;
   change one thing; run it again; keep the change only if the number moved, and
   commit the run with the change.
2. **A benchmark**: `references/measuring.md`: a `[[bench]]` with `harness =
   false`, `black_box` in and out, a warm-up, a distribution, batches for a
   small call, a pinned core, and its results committed.
3. **Judging a change made for speed**: `references/measuring.md`: the baseline
   and the change run twice each, all committed; both of the change's p50s past
   both of the baseline's, the way claimed, by more than its spread and 1%; a
   claim about generated code read in the disassembled binary that ships.
4. **A hot path or loop**: `references/allocation.md` for what it allocates,
   `references/codegen.md` for its calls, branches and bounds, and
   `references/data-layout.md` for the types it reads.
5. **A type stored many times or read in a hot loop**:
   `references/data-layout.md`: its size asserted, its integers, its `Option`s
   and variants, where its fields live.
6. **State that threads share, a spin, a poll, a pinned thread, a clock in a
   loop**: `references/threads.md`.
7. **A build profile, LTO, codegen units, a target CPU, or `panic`**:
   `references/codegen.md`, from its CPU rule on.
8. **Before finishing**: the checks below, and the benchmark the change rests
   on, run again.

The code a change makes faster still follows `writing-rust`, and an atomic, a
lock or `get_unchecked` follows `writing-unsafe-rust`.

A test that counts allocations, or asserts a size, follows `writing-rust-tests`.

## Checks

- `just check`: every check, as CI runs them, after `just fix`.
- `just check-rust-clippy`: clippy on every crate, target and feature, the
  benchmarks included, which holds `inline_always`, `format_push_string`,
  `assigning_clones`, `large_stack_arrays`, `large_enum_variant`, `linkedlist`,
  `map_entry`, `implicit_hasher` and a crate's `disallowed-methods`.
- `just check-cargo-nextest`: every test, the counting-allocator tests included.
- By hand, since no recipe runs them: `cargo bench -p <crate> --bench <name>` on
  a pinned core, and `cargo build --profile profiling` for a profiler.

## What Not to Do

| Thought                                       | Instead                                                                        |
| --------------------------------------------- | ------------------------------------------------------------------------------ |
| "This is obviously faster"                    | Measure it, before and after, and keep it only if it moved.                    |
| "A quick timing in a test"                    | A `harness = false` benchmark, under `cargo bench`.                            |
| "The loop's result isn't needed"              | `black_box` it: an unused result lets the work be deleted.                     |
| "The mean is enough"                          | p50, p99 and the maximum, on a pinned core.                                    |
| "`Option<Vec<_>>` to avoid allocating"        | An empty `Vec`, which allocates nothing and is the same size.                  |
| "`push_str(&format!(…))`"                     | `write!` into the buffer.                                                      |
| "`#[inline(always)]` to be sure"              | `#[inline]`, and `always` only where a call defeats the function, with reason. |
| "`#[cold]` keeps it out of the hot path"      | `#[cold]` with `#[inline(never)]`, since a small one is inlined anyway.        |
| "`get_unchecked` to drop the check"           | A loop that zips or slices first, so no check is there to drop.                |
| "Reorder the fields to save padding"          | The compiler does, unless `repr(C)`.                                           |
| "`target-cpu=native` in `.cargo/config.toml`" | A floor every machine meets, and a machine's CPU added for its own build.      |
| "`lto = false` turns LTO off"                 | `lto = "off"`; `false` is thin LTO within each crate.                          |
| "Change the release profile for this crate"   | `[profile.release.package.<crate>]`, or a profile that `inherits`.             |
| "Two counters side by side, one per thread"   | A 128-byte line each, asserted.                                                |
| "Spin until it's ready"                       | Block; spin first only for a measured wake-up, bounded, then park.             |
| "`dyn` is slow, make it generic"              | Measure: each type's copy of a generic costs code size too.                    |
| "`Arc::clone` per item, across threads"       | One clone a thread: every clone and drop writes the one count's line.          |

## References

Read every reference a task touches before changing code, and read them again
after compaction: this body is the summary, and the examples are there.

- `references/measuring.md`: before a change made for speed, a benchmark, a
  profile, or a claim of speed or of generated code.
- `references/allocation.md`: before a hot path, a buffer, a `format!`, a
  `clone` or a `collect` in a loop, a large value, or a crate or path that must
  not allocate.
- `references/codegen.md`: before an inline attribute, `#[cold]`, a hint, a
  bounds check, a float sum, a target CPU, a build profile, LTO, codegen units
  or `panic`.
- `references/data-layout.md`: before a type stored many times or read in a hot
  loop, a `repr`, an `Option`, an enum's variants, an integer's width, a
  collection of boxes, or a hash map's hasher, keys and lookups.
- `references/threads.md`: before state several threads write, a spin, a busy
  poll, a pinned or isolated core, a clock read in a loop, or a hot thread's
  memory.
- `references/sources.md`: before citing a source for a rule, or adapting one.
