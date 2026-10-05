# Verifying

Read this before a test of unsafe code or of an atomic protocol, before a loom
model or a compile-fail test, and before calling unsafe code done. A `//
SAFETY:` or `// ORDERING:` comment states a proof; this says what runs it.

Loom checks each interleaving a model allows, and a compile-fail test that a
misuse stays refused.

The tests themselves, their names and messages, their files, and a fixture's
committed message, follow `writing-rust-tests`; this says what unsafe code needs
of them.

## A Test for Each Edge a Proof Names

A proof names its facts, "at most the length", "not empty", "not the last", and
each is an edge a test must run: the empty row, the first square and the last,
one past the end, a zero-sized value. A test of the middle alone runs the case
the proof was never in doubt for, and a checker finds nothing on a path no test
reaches.

```rust
#[must_use]
pub const fn halves(squares: &[u8], mid: usize) -> Option<(&[u8], &[u8])> {
    if mid > squares.len() {
        return None;
    }
    // SAFETY: `mid` is at most the length, checked above, which is all `split_at_unchecked`
    // needs.
    #[expect(unsafe_code, reason = "a split the check above has bounded")]
    Some(unsafe { squares.split_at_unchecked(mid) })
}

#[cfg(test)]
mod tests {
    use super::halves;

    // Bad: the one case the proof was never in doubt for.
    #[test]
    fn a_row_splits_at_its_middle() {
        assert_eq!(halves(&[1, 2, 3, 4], 2), Some((&[1, 2][..], &[3, 4][..])), "two and two");
    }
}
```

```rust
#[must_use]
pub const fn halves(squares: &[u8], mid: usize) -> Option<(&[u8], &[u8])> {
    if mid > squares.len() {
        return None;
    }
    // SAFETY: `mid` is at most the length, checked above, which is all `split_at_unchecked`
    // needs.
    #[expect(unsafe_code, reason = "a split the check above has bounded")]
    Some(unsafe { squares.split_at_unchecked(mid) })
}

#[cfg(test)]
mod tests {
    use super::halves;

    #[test]
    fn a_split_at_either_end_leaves_one_half_empty() {
        let row = [1, 2, 3];
        assert_eq!(halves(&row, 0), Some((&[][..], &row[..])), "all after");
        assert_eq!(halves(&row, 3), Some((&row[..], &[][..])), "and all before");
    }

    #[test]
    fn a_split_past_the_end_is_refused() {
        assert_eq!(halves(&[1, 2, 3], 4), None, "one past the length");
        assert_eq!(halves(&[], 1), None, "and past an empty row");
    }
}
```

Held by review.

## A Loom Model for Each Atomic Protocol, Under `--cfg loom`

A test that runs threads sees the interleavings this machine happens to pick,
and a thousand runs see mostly the same one. Loom runs a model's closure once
for each interleaving, and each value a load may return, that the memory model
allows, so an ordering too weak for the protocol fails there. A model sees only
loom's types, which is why atomics come from one module that swaps them in under
`--cfg loom`, as `atomics.md` says; the crate's other tests go under
`#[cfg(not(loom))]`, since a loom type used outside a model panics.

```rust
mod sync {
    // Loom's under `--cfg loom`, as `atomics.md` shows.
    pub(crate) use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
    pub(crate) use core::sync::atomic::{AtomicBool, AtomicU8};
}

use crate::sync::{Acquire, AtomicBool, AtomicU8, Relaxed, Release};

#[derive(Debug, Default)]
pub struct Game {
    winner: AtomicU8,
    finished: AtomicBool,
}

impl Game {
    pub fn finish(&self, winner: u8) {
        // ORDERING: Relaxed; the Release store of `finished` below publishes it.
        self.winner.store(winner, Relaxed);
        // ORDERING: Release, pairing with the Acquire load in `winner`.
        self.finished.store(true, Release);
    }

    #[must_use]
    pub fn winner(&self) -> Option<u8> {
        // ORDERING: Acquire, pairing with the Release store in `finish`.
        let finished = self.finished.load(Acquire);
        // ORDERING: Relaxed; the Acquire load above orders it after the store `finish` published.
        finished.then(|| self.winner.load(Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::Game;

    // Bad: a thousand runs of the interleaving this machine happens to pick.
    #[test]
    fn a_reader_that_sees_the_game_over_sees_its_winner() {
        for _ in 0..1000 {
            let game = Game::default();
            thread::scope(|scope| {
                scope.spawn(|| game.finish(3));
                if let Some(winner) = game.winner() {
                    assert_eq!(winner, 3, "the winner stored before the game ended");
                }
            });
        }
    }
}
```

The model sits beside the code it checks, in `#[cfg(test)] #[cfg(loom)] mod
model`, over the same `Game`, whose atomics are loom's there:

```text
// Beside the crate's other tests, which sit under `#[cfg(test)] #[cfg(not(loom))]`.
#[cfg(test)]
#[cfg(loom)]
mod model {
    use loom::sync::Arc;
    use loom::thread;

    use super::Game;

    #[test]
    fn a_reader_that_sees_the_game_over_sees_its_winner() {
        loom::model(|| {
            let game = Arc::new(Game::default());
            let finisher = {
                let game = Arc::clone(&game);
                thread::spawn(move || game.finish(3))
            };
            if let Some(winner) = game.winner() {
                assert_eq!(winner, 3, "the winner stored before the game ended");
            }
            finisher.join().expect("the finishing thread ends");
        });
    }
}
```

Loom is a dependency only under the cfg, and the workspace declares the cfg in
the lint table's `unexpected_cfgs`, a key the repository owns, which devset
leaves as it is:

```text
# The crate's Cargo.toml.
[target.'cfg(loom)'.dependencies]
loom = "0.7"

# The workspace Cargo.toml.
[workspace.lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ["cfg(loom)"] }
```

Name the crate with `-p`, so its models run, and pass the cfg through
`--config`, which adds to the rustflags Cargo's configuration sets, where
`RUSTFLAGS` replaces them. Either reaches every crate the build compiles,
dependencies included, so a dependency that reads `cfg(loom)` itself builds its
loom variant too:

```text
cargo test -p tiles --lib --release --config 'target."cfg(all())".rustflags=["--cfg","loom"]'
```

Loom does not model everything, and a model is written within what it does:

- It treats a `SeqCst` access as `AcqRel`, and models a `SeqCst` fence, so a
  protocol that needs `SeqCst` uses the fence.
- It sees only its own atomics and cells: a plain field a protocol publishes is
  invisible to it, so the data it guards sits in its `UnsafeCell`, or a test of
  it runs under a checker that sees memory.
- It does not explore every reordering `Relaxed` allows within a thread, such as
  a load buffered past a later store, so a model that passes is not a proof of a
  `Relaxed` protocol.
- It runs at most five threads, the model's own included, and a model beyond two
  or three takes long: `LOOM_MAX_PREEMPTIONS=3` bounds the search, which its
  documentation says still catches most bugs.
- Its scheduler is not fair, so a spin loop calls the module's `spin_loop`,
  loom's under the cfg, to let the other thread run.

Held by review.

## A Compile-Fail Test for Each Misuse the Types Refuse

A type that refuses a misuse by construction, a handle that cannot outlive its
grid, a brush that cannot cross threads, holds that refusal only while nothing
changes it, so a test pins it: a trybuild fixture that must fail to compile,
with the compiler's message committed beside it, so a fixture that fails for
another reason, a typo, fails the test. A rustdoc `compile_fail` block passes on
any error at all, which is why it is not the test. What must stay `Send` or
`Sync` is asserted in the crate itself, at compile time, and that an `unsafe fn`
stays unsafe by a test that calls it inside `unsafe` under
`#[deny(unused_unsafe)]`, which fails once the function is made safe.

````text
/// A brush names a slot in this thread's palette.
///
/// ```compile_fail
/// // Bad: fails on any error, so a renamed function passes it as well as a `Send` brush.
/// let brush = tiles::Brush::for_current_thread();
/// std::thread::spawn(move || brush.slot());
/// ```
````

```text
// tests/compile_fail/a_brush_stays_on_its_thread.rs
//! A brush names a slot in its own thread's palette, so it cannot cross to another.

use std::thread;

use tiles::Brush;

fn main() {
    let brush = Brush::for_current_thread();
    thread::spawn(move || brush.slot());
}

// tests/trybuild.rs, beside `a_brush_stays_on_its_thread.stderr`, which holds its E0277.
//! The misuses the types refuse, each a fixture that must not compile.

#[cfg(test)]
mod tests {
    #[test]
    fn each_misuse_fails_to_compile() {
        trybuild::TestCases::new().compile_fail("tests/compile_fail/*.rs");
    }
}
```

A fixture's message is written, and rewritten after a change, with
`TRYBUILD=overwrite cargo nextest run -p tiles --test trybuild`, and the new
message is read before it is committed.

In a crate with loom models the file carries `#![cfg(not(loom))]`, since a loom
model drives no compiler.

```rust
#[derive(Debug, Default)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    /// The square at `index`, unchecked.
    ///
    /// # Safety
    /// `index` is below the grid's square count.
    #[expect(unsafe_code, reason = "an unchecked read for callers that have bounded the index")]
    #[must_use]
    pub unsafe fn square_unchecked(&self, index: usize) -> u8 {
        // SAFETY: the caller promises `index` is below the count, which is all `get_unchecked`
        // asks.
        unsafe { *self.squares.get_unchecked(index) }
    }
}

// A grid is shared by the renderer's threads, so a field that takes that away fails here.
const fn send_and_sync<T: Send + Sync>() {}
const _: () = send_and_sync::<Grid>();

#[cfg(test)]
mod tests {
    use super::Grid;

    /// Reads through `unsafe`, so a `square_unchecked` made safe stops compiling here.
    #[deny(unused_unsafe)]
    #[test]
    #[expect(unsafe_code, reason = "a call of the unchecked read, to pin that it stays unsafe")]
    fn an_unchecked_read_stays_unsafe() {
        let grid = Grid { squares: vec![1, 2, 3] };
        // SAFETY: 1 is below the grid's three squares.
        assert_eq!(unsafe { grid.square_unchecked(1) }, 2, "the second square");
    }
}
```

Held by the fixtures, which fail the test when a refusal goes, and by review for
the refusals that have none.
