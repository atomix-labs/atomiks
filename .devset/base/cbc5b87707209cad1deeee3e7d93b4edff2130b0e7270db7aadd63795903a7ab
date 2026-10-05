# Atomics

Read this before a value is shared between threads through anything but a lock
or a channel: an atomic, a fence, a `static` that changes, an `unsafe impl Sync`
over shared state. It says which orderings a protocol needs, how each is written
down, and how atomics are reached so a model checker can see them.

An ordering is a promise about what else a thread sees. A `Release` store
publishes everything its thread wrote before it; an `Acquire` load that reads
the value it stored sees all of that. `Relaxed` orders nothing but the atomic
itself. `SeqCst` adds one order every thread agrees on across all `SeqCst`
operations and fences. A data race, two accesses to one place that nothing
orders, one a write and one not atomic, is undefined behaviour; a wrong ordering
between atomics is not, but it lets a thread act on a value another has not
finished, which is the same bug one step removed.

What an atomic costs when threads share its cache line, and how long a spin
should wait, are `tuning-rust-performance`'s.

## Shared State Is a Lock, a `OnceLock` or an Atomic, Never `static mut`

`static mut` is a global any thread may write, whose every access is unsafe with
a proof no module can keep, and edition 2024 denies taking a reference to it. A
value set once is a `OnceLock`, a value that changes is a `Mutex` or an
`RwLock`, and a lone word, a flag or a count, is an atomic. A lock's proof is
the standard library's; an atomic protocol is one the crate proves itself,
ordering by ordering, so it is written only where a measured lock costs too
much.

```rust,compile_fail
// fails: static_mut_refs
static mut TILESET: Vec<char> = Vec::new();

#[expect(unsafe_code, reason = "the tile set installed once at startup")]
pub fn install(glyphs: &[char]) {
    // Bad: a `&mut` to a global any thread may take one of too.
    // SAFETY: `install` runs once, before any thread starts.
    let tileset = unsafe { &mut TILESET };
    tileset.extend_from_slice(glyphs);
}
```

```rust
use std::sync::OnceLock;

use thiserror::Error;

static TILESET: OnceLock<Vec<char>> = OnceLock::new();

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("install error: a tile set is installed already")]
pub struct InstallError {
    pub glyphs: Vec<char>,
}

pub fn install(glyphs: Vec<char>) -> Result<(), InstallError> {
    TILESET.set(glyphs).map_err(|glyphs| InstallError { glyphs })
}

#[must_use]
pub fn tileset() -> Option<&'static [char]> {
    TILESET.get().map(Vec::as_slice)
}
```

Held by `static_mut_refs`, which edition 2024 denies, for a reference to a
`static mut`. A read or write by value, `COUNT = COUNT.wrapping_add(1)`,
compiles, and only its `unsafe` block marks it, so it is held by review.

Under `strict`, `unsafe_code` makes that block name its reason in an
`#[expect]`, and `clippy::mutex_atomic` and `clippy::mutex_integer` refuse a
`Mutex` around what an atomic holds.

## Reach Atomics Through One Module, so `--cfg loom` Swaps Them

A model checker such as loom sees only the atomics, cells and threads that are
its own. So a crate with an atomic protocol takes its atomics, `fence`,
`UnsafeCell` and `spin_loop` from one module of its own, which re-exports
`core`'s and, under `--cfg loom`, loom's, and the same body is both the
production code and the model. Loom's types differ in small ways the module
covers: its atomics have no `const fn new`, so a `static` atomic is modelled
through `loom::lazy_static!`, nor `get_mut`, so a `&mut self` reads them with a
`Relaxed` load, and its `UnsafeCell` is reached through `with` and `with_mut`.

```text
// grid.rs
// Bad: `core`'s atomic, which a loom model cannot see.
use core::sync::atomic::{AtomicBool, Ordering};
```

```text
// sync.rs: every atomic the crate uses, from one place.
#[cfg(not(loom))]
pub(crate) use core::sync::atomic::{AtomicBool, AtomicU8, Ordering, fence};
#[cfg(loom)]
pub(crate) use loom::sync::atomic::{AtomicBool, AtomicU8, Ordering, fence};

// grid.rs
use crate::sync::{AtomicBool, Ordering};
```

Held by review. `verifying.md` shows the model, and how the workspace declares
the cfg.

## Every Atomic Operation Has an `// ORDERING:` That Names What It Pairs With

An ordering is chosen for what it pairs with: a `Release` store with the
`Acquire` loads that read it, and the other way about. The `// ORDERING:`
comment above each operation names the ordering and that pair, "Release, pairing
with the Acquire load in `winner`", or says it publishes nothing; a function
whose operations all share one ordering says so once at its top, "Relaxed
throughout". A reader then checks each pair from both ends, and a change to one
end finds the other.

```rust
use core::sync::atomic::Ordering::Relaxed;
use core::sync::atomic::{AtomicBool, AtomicU8};

#[derive(Debug, Default)]
pub struct Game {
    winner: AtomicU8,
    finished: AtomicBool,
}

impl Game {
    pub fn finish(&self, winner: u8) {
        self.winner.store(winner, Relaxed);
        // Bad: `Relaxed` orders nothing, so a reader may see the game over and no winner yet.
        self.finished.store(true, Relaxed);
    }

    #[must_use]
    pub fn winner(&self) -> Option<u8> {
        self.finished.load(Relaxed).then(|| self.winner.load(Relaxed))
    }
}
```

```rust
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use core::sync::atomic::{AtomicBool, AtomicU8};

#[derive(Debug, Default)]
pub struct Game {
    winner: AtomicU8,
    finished: AtomicBool,
}

impl Game {
    pub fn finish(&self, winner: u8) {
        // ORDERING: Relaxed; the Release store of `finished` below publishes it.
        self.winner.store(winner, Relaxed);
        // ORDERING: Release, pairing with the Acquire load in `winner`, so a reader that sees the
        // game over sees the winner stored before it.
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
```

Held by review, and by a loom model of the pair.

## `Relaxed` Where Nothing Pairs

An atomic that publishes nothing but its own value, a stop flag or a statistic,
needs no ordering, and a stronger one "to be safe" may cost a barrier on every
access, and tells a reader a pairing exists that does not. Whatever else such a
word's readers rely on is ordered by something else, a join or a lock, which its
`// ORDERING:` names.

```rust
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::SeqCst;

#[derive(Debug, Default)]
pub struct Renderer {
    stop: AtomicBool,
}

impl Renderer {
    pub fn stop(&self) {
        // Bad: `SeqCst` to be safe, which orders nothing this flag needs, and says nothing why.
        self.stop.store(true, SeqCst);
    }

    #[must_use]
    pub fn stopped(&self) -> bool {
        self.stop.load(SeqCst)
    }
}
```

```rust
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::Relaxed;

#[derive(Debug, Default)]
pub struct Renderer {
    stop: AtomicBool,
}

impl Renderer {
    pub fn stop(&self) {
        // ORDERING: Relaxed; the flag publishes nothing but itself, and whoever waits for the
        // renderer to end joins its thread, which orders the rest.
        self.stop.store(true, Relaxed);
    }

    #[must_use]
    pub fn stopped(&self) -> bool {
        // ORDERING: Relaxed, as in `stop`.
        self.stop.load(Relaxed)
    }
}
```

Held by review.

## `SeqCst` Only with Its Reason, and as a Fence Between a Store and a Load

`Release` and `Acquire` order what a thread does before a store, and what it
does after a load, but never a store before a later load: each of two threads
may store its own flag and then read the other's as it was before either store.
Where a protocol rests on that order, two sides that each announce and then
look, a `SeqCst` fence between the store and the load puts every such fence in
one order, so one side's look comes after the other's announcement. That order
is what `SeqCst` buys, and it is written only where a protocol rests on it, with
an `// ORDERING:` paragraph that says how. It is a fence rather than `SeqCst`
accesses, which loom models only as `AcqRel`.

```rust
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::{Acquire, Release};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Painter {
    Left,
    Right,
}

#[derive(Debug, Default)]
pub struct Square {
    left: AtomicBool,
    right: AtomicBool,
}

impl Square {
    const fn flags(&self, painter: Painter) -> (&AtomicBool, &AtomicBool) {
        match painter {
            Painter::Left => (&self.left, &self.right),
            Painter::Right => (&self.right, &self.left),
        }
    }

    #[must_use]
    pub fn enter(&self, painter: Painter) -> bool {
        let (own_flag, other_flag) = self.flags(painter);
        own_flag.store(true, Release);
        // Bad: the load may see the other painter's flag as it was before its store, and so may
        // theirs of this one: both paint.
        if other_flag.load(Acquire) {
            own_flag.store(false, Release);
            return false;
        }
        true
    }
}
```

```rust
use core::sync::atomic::Ordering::{Acquire, Release, SeqCst};
use core::sync::atomic::{AtomicBool, fence};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Painter {
    Left,
    Right,
}

#[derive(Debug, Default)]
pub struct Square {
    left: AtomicBool,
    right: AtomicBool,
}

impl Square {
    const fn flags(&self, painter: Painter) -> (&AtomicBool, &AtomicBool) {
        match painter {
            Painter::Left => (&self.left, &self.right),
            Painter::Right => (&self.right, &self.left),
        }
    }

    #[must_use]
    pub fn enter(&self, painter: Painter) -> bool {
        let (own_flag, other_flag) = self.flags(painter);
        // ORDERING: Release, so a painter that sees this flag sees what this one did before; then
        // a SeqCst fence, the store-load order the exclusion rests on. Both painters announce,
        // then look, and the fences' one order puts one painter's look after the other's
        // announcement, so at most one finds the square clear. A fence rather than SeqCst
        // accesses, since loom models the fence and not the accesses.
        own_flag.store(true, Release);
        fence(SeqCst);
        // ORDERING: Acquire, pairing with the Release store in `leave`, so a painter that enters
        // sees what the last one painted.
        if other_flag.load(Acquire) {
            // ORDERING: Release, as in `leave`.
            own_flag.store(false, Release);
            return false;
        }
        true
    }

    pub fn leave(&self, painter: Painter) {
        // ORDERING: Release, pairing with the Acquire load in `enter`, so the next painter sees
        // what this one painted.
        self.flags(painter).0.store(false, Release);
    }
}
```

Held by review, and by a loom model, which finds both painters inside without
the fence.

## A `compare_exchange` Chooses Both Orderings

A `compare_exchange` has an ordering for when it succeeds, and one for when it
fails, which only reads: `Release` or `AcqRel` there is refused. A claim that
takes what the last holder released succeeds with `Acquire`, and a failure that
acts on nothing it read is `Relaxed`. `compare_exchange_weak` may fail
spuriously, so it belongs in a loop that retries; a lone attempt is
`compare_exchange`.

```rust,compile_fail
// fails: invalid_atomic_ordering
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering::{Acquire, Release};

#[derive(Debug, Default)]
pub struct Claim {
    owner: AtomicU32,
}

impl Claim {
    #[must_use]
    pub fn try_claim(&self, editor: u32) -> bool {
        // Bad: a failed exchange writes nothing, so it has nothing to release.
        self.owner.compare_exchange(0, editor, Acquire, Release).is_ok()
    }
}
```

```rust
use core::num::NonZeroU32;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

use thiserror::Error;

const FREE: u32 = 0;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("claim error: editor {owner} holds the square")]
pub struct ClaimError {
    pub owner: u32,
}

#[derive(Debug, Default)]
pub struct Claim {
    owner: AtomicU32,
}

impl Claim {
    pub fn try_claim(&self, editor: NonZeroU32) -> Result<(), ClaimError> {
        // ORDERING: Acquire on success, pairing with the Release store in `release`, so this
        // editor sees what the last one wrote; Relaxed on failure, which acts on nothing it read.
        self.owner
            .compare_exchange(FREE, editor.get(), Acquire, Relaxed)
            .map(drop)
            .map_err(|owner| ClaimError { owner })
    }

    pub fn release(&self) {
        // ORDERING: Release, pairing with the Acquire exchange in `try_claim`.
        self.owner.store(FREE, Release);
    }
}
```

Held by `invalid_atomic_ordering`, which refuses a failure ordering that writes,
a load that releases and a store that acquires.

## A Type Over an `UnsafeCell` Is `Sync` with the Bound Its Access Needs

An `UnsafeCell` field takes `Sync` away, and the `unsafe impl` that gives it
back takes the bound that matches what `&self` lets a thread do with the `T`
inside. A lock lends one thread at a time a `&mut T`, through which it can swap
a `T` in or out, so a `T` crosses threads: its `Sync` needs `T: Send`, as
`Mutex`'s does, not `T: Sync`, which would let a thread swap out a `MutexGuard`
that must stay on the thread that took it. A type that lends `&T` to several
threads at once needs `T: Sync`, and one that does both, as `RwLock` and
`OnceLock` do, needs both. The cell comes from the crate's atomics module too,
reached through `with_mut` as loom's is, so a model sees every access to it.

```text
// Bad: `lock_with` lends `&mut T`, so a `T` that is `Sync` but not `Send`, a `MutexGuard`, can be
// swapped out onto another thread.
// SAFETY: sharing the lock shares `&T`s, which `T: Sync` allows.
#[expect(unsafe_code, reason = "an `UnsafeCell` field takes away the auto `Sync`")]
unsafe impl<T: Sync> Sync for SpinLock<T> {}
```

```rust
mod sync {
    // Loom's under `--cfg loom`, whose `UnsafeCell` is reached through `with_mut`, as this is.
    pub(crate) use core::hint::spin_loop;
    pub(crate) use core::sync::atomic::AtomicBool;
    pub(crate) use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

    use core::cell;

    #[derive(Debug)]
    pub(crate) struct UnsafeCell<T>(cell::UnsafeCell<T>);

    impl<T> UnsafeCell<T> {
        pub(crate) const fn new(value: T) -> Self {
            Self(cell::UnsafeCell::new(value))
        }

        pub(crate) fn with_mut<R, F: FnOnce(*mut T) -> R>(&self, access: F) -> R {
            access(self.0.get())
        }
    }
}

use crate::sync::{Acquire, AtomicBool, Relaxed, Release, UnsafeCell, spin_loop};

#[derive(Debug)]
pub struct SpinLock<T> {
    locked: AtomicBool,
    // INVARIANT: reached only inside `lock_with`, by the thread whose exchange set `locked`.
    value: UnsafeCell<T>,
}

// SAFETY: `lock_with` lends `&mut T` to one thread at a time, through which it may move a `T` in or
// out, so sharing the lock moves `T`s between threads, which `T: Send` allows; it never lends two
// threads a `&T` at once, so it needs no `T: Sync`.
#[expect(unsafe_code, reason = "an `UnsafeCell` field takes away the auto `Sync`")]
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    #[expect(
        clippy::missing_const_for_fn,
        reason = "loom's atomics, which a model swaps in, have no `const fn new`"
    )]
    pub fn new(value: T) -> Self {
        Self { locked: AtomicBool::new(false), value: UnsafeCell::new(value) }
    }

    pub fn lock_with<R, F: FnOnce(&mut T) -> R>(&self, access: F) -> R {
        // ORDERING: Acquire on success, pairing with the Release store in `Unlock::drop`, so this
        // thread sees what the last holder wrote; Relaxed on failure, which only retries.
        while self.locked.compare_exchange_weak(false, true, Acquire, Relaxed).is_err() {
            spin_loop();
        }
        let _guard = Unlock(&self.locked);
        // SAFETY: by the field INVARIANT only the holder reaches the value, and the exchange above
        // made this thread the holder until `_guard` drops, after `access` returns or unwinds.
        #[expect(unsafe_code, reason = "the holder's exclusive borrow of the value")]
        self.value.with_mut(|value| access(unsafe { &mut *value }))
    }
}

struct Unlock<'a>(&'a AtomicBool);

impl Drop for Unlock<'_> {
    fn drop(&mut self) {
        // ORDERING: Release, pairing with the Acquire exchange in `lock_with`, so the next holder sees
        // what this one wrote.
        self.0.store(false, Release);
    }
}
```

Held by review, and by a loom model, which reports two threads in the cell at
once when the lock's orderings are too weak.

The lock shows the orderings a lock needs; how long a waiter spins before it
blocks, and whether a spin pays at all, are `tuning-rust-performance`'s.

## A Pointer Shared Across Threads Is an `AtomicPtr`

An `AtomicUsize` holds an address, and an address has no provenance, so a
pointer stored in one comes back as a guess. `AtomicPtr<T>` holds the pointer
itself, provenance and all, and its loads and stores order what it points at
like any other atomic's.

```rust
use core::ptr;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::{Acquire, Release};

#[derive(Debug)]
pub struct Tileset {
    pub glyphs: [char; 4],
}

#[derive(Debug, Default)]
pub struct Theme {
    // Bad: an address, so the tile set's provenance is dropped with each store.
    tileset: AtomicUsize,
}

impl Theme {
    pub fn set(&self, tileset: &'static Tileset) {
        self.tileset.store(ptr::from_ref(tileset).expose_provenance(), Release);
    }

    #[must_use]
    #[expect(unsafe_code, reason = "a tile set rebuilt from the address the theme holds")]
    pub fn get(&self) -> Option<&'static Tileset> {
        let tileset = ptr::with_exposed_provenance::<Tileset>(self.tileset.load(Acquire));
        // SAFETY: zero, or the address of a `&'static Tileset` whose provenance `set` exposed.
        unsafe { tileset.as_ref() }
    }
}
```

```rust
use core::ptr;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering::{Acquire, Release};

#[derive(Debug)]
pub struct Tileset {
    pub glyphs: [char; 4],
}

#[derive(Debug, Default)]
pub struct Theme {
    // INVARIANT: null, or a `&'static Tileset` that `set` stored, never written through.
    tileset: AtomicPtr<Tileset>,
}

impl Theme {
    pub fn set(&self, tileset: &'static Tileset) {
        // ORDERING: Release, pairing with the Acquire load in `get`, so a reader sees the tile set
        // as it was built.
        self.tileset.store(ptr::from_ref(tileset).cast_mut(), Release);
    }

    #[must_use]
    #[expect(unsafe_code, reason = "the tile set the theme's pointer names")]
    pub fn get(&self) -> Option<&'static Tileset> {
        // ORDERING: Acquire, pairing with the Release store in `set`.
        let tileset = self.tileset.load(Acquire);
        // SAFETY: by the field INVARIANT the pointer is null or a `&'static Tileset`'s, with its
        // provenance, and nothing writes through it.
        unsafe { tileset.as_ref() }
    }
}
```

Held by review.

Under `nightly`, `implicit_provenance_casts` refuses the `as` casts an
`AtomicUsize` of pointers would otherwise need.

## A Node Another Thread May Read Is Freed Only Through a Reclamation Scheme

In a structure without a lock, a thread that has loaded a pointer to a node may
read the node after another thread unlinks it, so freeing it then is a use after
free. Its address, reused by a new node, also lets a `compare_exchange` that
expects the old node succeed on the new one and link in a `next` long gone, the
ABA problem. So a node another thread may still read is freed only once none
can: through a reclamation scheme, epochs or hazard pointers, as
`crossbeam-epoch` provides, or not before the whole structure drops, when `&mut
self` proves no reader is left.

```rust
extern crate alloc;

use alloc::boxed::Box;
use core::ptr;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

#[derive(Debug)]
struct Node {
    tile: u8,
    next: *mut Self,
}

#[derive(Debug, Default)]
pub struct Stack {
    // INVARIANT: null, or a node from `Box::into_raw` in `push`, heading a list of such nodes.
    head: AtomicPtr<Node>,
}

impl Stack {
    #[expect(unsafe_code, reason = "a node linked in front of the head")]
    pub fn push(&self, tile: u8) {
        let node = Box::into_raw(Box::new(Node { tile, next: ptr::null_mut() }));
        // ORDERING: Relaxed; the exchange below publishes the node.
        let mut head = self.head.load(Relaxed);
        loop {
            // SAFETY: `node` is this thread's alone until the exchange below publishes it.
            unsafe {
                (*node).next = head;
            }
            // ORDERING: Release on success, pairing with the Acquire loads in `pop`; Relaxed on
            // failure, which retries.
            match self.head.compare_exchange_weak(head, node, Release, Relaxed) {
                Ok(_) => return,
                Err(actual) => head = actual,
            }
        }
    }

    #[must_use]
    #[expect(unsafe_code, reason = "a node unlinked from the head, and freed")]
    pub fn pop(&self) -> Option<u8> {
        // ORDERING: Acquire, pairing with the Release exchange in `push`.
        let mut head = self.head.load(Acquire);
        while !head.is_null() {
            // Bad: another thread may have popped and freed `head` since it was loaded.
            // SAFETY: by the field INVARIANT `head` is a node of the list.
            let next = unsafe { (*head).next };
            // ORDERING: Acquire both ways, pairing with the Release exchange in `push`.
            match self.head.compare_exchange_weak(head, next, Acquire, Acquire) {
                Ok(_) => {
                    // SAFETY: the exchange unlinked `head`, so this thread alone holds it.
                    let node = unsafe { Box::from_raw(head) };
                    return Some(node.tile);
                },
                Err(actual) => head = actual,
            }
        }
        None
    }
}
```

```rust
extern crate alloc;

use alloc::boxed::Box;
use core::ptr;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

#[derive(Debug)]
struct Node {
    tile: u8,
    next: *mut Self,
}

#[derive(Debug, Default)]
pub struct Log {
    // INVARIANT: null, or a node from `Box::into_raw` in `push`, heading a list of such nodes,
    // none freed before the log drops.
    head: AtomicPtr<Node>,
}

impl Log {
    #[expect(unsafe_code, reason = "a node linked in front of the head")]
    pub fn push(&self, tile: u8) {
        let node = Box::into_raw(Box::new(Node { tile, next: ptr::null_mut() }));
        // ORDERING: Relaxed; the exchange below publishes the node.
        let mut head = self.head.load(Relaxed);
        loop {
            // SAFETY: `node` is this thread's alone until the exchange below publishes it.
            unsafe {
                (*node).next = head;
            }
            // ORDERING: Release on success, pairing with the Acquire load in `contains`; Relaxed
            // on failure, which retries.
            match self.head.compare_exchange_weak(head, node, Release, Relaxed) {
                Ok(_) => return,
                Err(actual) => head = actual,
            }
        }
    }

    #[must_use]
    #[expect(unsafe_code, reason = "a walk over nodes that live until the log drops")]
    pub fn contains(&self, tile: u8) -> bool {
        // ORDERING: Acquire, pairing with the Release exchange that published the head, and,
        // since each later exchange carries the earlier ones along, with those of the nodes below.
        let mut cursor = self.head.load(Acquire);
        while !cursor.is_null() {
            // SAFETY: by the field INVARIANT `cursor` is a live node, freed only once the log
            // drops.
            let node = unsafe { &*cursor };
            if node.tile == tile {
                return true;
            }
            cursor = node.next;
        }
        false
    }
}

impl Drop for Log {
    #[expect(unsafe_code, reason = "the log frees its nodes once no reader is left")]
    fn drop(&mut self) {
        // ORDERING: Relaxed; `&mut self` means every push has happened before this.
        let mut cursor = self.head.load(Relaxed);
        while !cursor.is_null() {
            // SAFETY: `&mut self` leaves no reader, and by the field INVARIANT each node came from
            // `Box::into_raw` and is freed only here.
            let node = unsafe { Box::from_raw(cursor) };
            cursor = node.next;
        }
    }
}
```

Held by review. A loom model of a push beside a walk checks the orderings on the
head, and not the nodes' fields, which are not loom's cells; a reclamation
scheme's own proofs are its crate's.
