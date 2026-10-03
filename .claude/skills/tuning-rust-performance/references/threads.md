# Threads

Read this before a counter or a flag that several threads write, before a thread
that waits for another by spinning, before a busy poll, a pinned or isolated
core, a clock read in a hot loop, and memory a hot thread touches for the first
time. Threads cost each other through what they share, down to the cache line,
and a hot thread costs its machine a core; this is what each costs, and how it
is paid once instead of on every turn.

Which ordering an atomic needs, and the `// ORDERING:` each operation carries,
are `writing-unsafe-rust`'s.

## Values Written by Different Threads Live on Different Cache Lines

A core writes a whole cache line, never a word of it: two values that different
threads write, side by side on one line, move that line between the two cores on
every write, though neither thread reads the other's value. That is false
sharing, and it runs both threads at the speed of the line crossing between
them. So a value written by one thread and read or written by others sits on a
line of its own, and the line is the target's: 64 bytes on x86-64 and on a
Graviton4's Neoverse V2, but Intel's cores since Sandy Bridge fetch lines in
pairs, the big cores of Arm's big.LITTLE designs have 128-byte lines, and on
Apple silicon, Apple's documentation says, the size differs from Intel Macs' and
is read from `hw.cachelinesize`. crossbeam's `CachePadded` pads to 128 bytes on
x86-64 and aarch64 for the first two reasons, and code that runs on more than
one kind of machine does the same: `#[repr(align(128))]` on a wrapper, its size
asserted, or `CachePadded` where the repository has crossbeam. Measure it: the
cost depends on the machine and on how often each thread writes.

```rust
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering::Relaxed;

#[derive(Debug, Default)]
pub struct Tally {
    // Bad: one line for both, so each painter's write takes it from the eraser.
    painted: AtomicU64,
    erased: AtomicU64,
}

impl Tally {
    pub fn paint(&self) {
        // ORDERING: Relaxed, a statistic that publishes only itself.
        self.painted.fetch_add(1, Relaxed);
    }

    pub fn erase(&self) {
        // ORDERING: Relaxed, a statistic that publishes only itself.
        self.erased.fetch_add(1, Relaxed);
    }
}
```

```rust
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering::Relaxed;

#[derive(Debug, Default)]
#[repr(align(128))]
pub struct Line<T>(T);

#[derive(Debug, Default)]
pub struct Tally {
    painted: Line<AtomicU64>,
    erased: Line<AtomicU64>,
}

// Each count is written by its own thread, so each has a line to itself, a pair on x86-64.
const _: () = assert!(size_of::<Tally>() == 256, "two counts, 128 bytes each");

impl Tally {
    pub fn paint(&self) {
        // ORDERING: Relaxed, a statistic that publishes only itself.
        self.painted.0.fetch_add(1, Relaxed);
    }

    pub fn erase(&self) {
        // ORDERING: Relaxed, a statistic that publishes only itself.
        self.erased.0.fetch_add(1, Relaxed);
    }
}
```

Held by review, and by the size assertion.

## Each Thread Adds Its Count Once

Threads that all write one atomic take turns at its line, whatever they write to
it, so a count bumped once for each tile by every worker runs no faster than one
worker. Each worker counts in a value of its own, and adds it to the shared
count once, when it is done; a count read while the work goes on is split into
one part a thread, each on its own line, and summed when read. The work is split
into one share a worker, since a thread for each row costs a spawn each.

```rust
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering::Relaxed;
use std::thread;

#[must_use]
pub fn lit(rows: &[Vec<u8>], workers: usize) -> u64 {
    let lit = AtomicU64::new(0);
    let share = rows.len().div_ceil(workers.max(1)).max(1);
    thread::scope(|scope| {
        for rows in rows.chunks(share) {
            let lit = &lit;
            scope.spawn(move || {
                for tile in rows.iter().flatten() {
                    if *tile > 0 {
                        // Bad: every worker takes the one line for every tile.
                        // ORDERING: Relaxed throughout, a count read once the scope has joined.
                        lit.fetch_add(1, Relaxed);
                    }
                }
            });
        }
    });
    lit.into_inner()
}
```

```rust
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering::Relaxed;
use std::thread;

#[must_use]
pub fn lit(rows: &[Vec<u8>], workers: usize) -> u64 {
    let lit = AtomicU64::new(0);
    let share = rows.len().div_ceil(workers.max(1)).max(1);
    thread::scope(|scope| {
        for rows in rows.chunks(share) {
            let lit = &lit;
            scope.spawn(move || {
                let mine = rows.iter().flatten().filter(|tile| **tile > 0).count();
                // ORDERING: Relaxed throughout, a count read once the scope has joined.
                lit.fetch_add(u64::try_from(mine).unwrap_or(u64::MAX), Relaxed);
            });
        }
    });
    lit.into_inner()
}
```

Held by review.

## A Wait Blocks, and Spins First Only Where Waking Is Measured Too Slow

A thread waiting for another either blocks, and the operating system wakes it
later, or spins, reading until the value changes. A spinning thread burns its
core, and on a core it shares delays the very thread it waits for, so a wait
blocks: on a channel, a `Condvar`, or `thread::park` with the setter's `unpark`.
std's `Mutex` on Linux already spins briefly, a hundred reads, before it sleeps;
`park` and a `Condvar` sleep at once. A hand-written spin is only for a wake-up
measured too slow, on a core of its own: it calls `core::hint::spin_loop()` on
every turn, which tells the core it is waiting, `pause` on x86-64 and `isb` on
Arm, for a bound measured against the wake-up it saves, then blocks. Never
`thread::yield_now` in a loop: on a core with nothing else to run it returns at
once, and the loop spins on with no bound. The setter unparks the thread that
made the pair, so the half that waits is kept on it by a `PhantomData<*const
()>`, and a wait from another thread does not compile.

```rust
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::Acquire;
use std::thread;

pub fn wait_for(ready: &AtomicBool) {
    // Bad: a spin with no bound, and a yield that returns at once where nothing else runs.
    // ORDERING: Acquire, pairing with the Release store that sets `ready`.
    while !ready.load(Acquire) {
        thread::yield_now();
    }
}
```

```rust
extern crate alloc;

use alloc::sync::Arc;
use core::hint::spin_loop;
use core::marker::PhantomData;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::{Acquire, Release};
use std::thread::{self, Thread};

// A hundred turns, about 1.5 us on a Graviton4, under the 6 us a park and an unpark took there.
const SPINS: u32 = 100;

#[derive(Debug)]
struct Flag {
    set: AtomicBool,
    waiter: Thread,
}

#[derive(Debug, Clone)]
pub struct Setter(Arc<Flag>);

#[derive(Debug)]
pub struct Waiter {
    flag: Arc<Flag>,
    // The setter unparks the thread that made the pair, so the waiter stays on it.
    _here: PhantomData<*const ()>,
}

#[must_use]
pub fn ready() -> (Setter, Waiter) {
    let flag = Arc::new(Flag { set: AtomicBool::new(false), waiter: thread::current() });
    (Setter(Arc::clone(&flag)), Waiter { flag, _here: PhantomData })
}

impl Setter {
    pub fn set(&self) {
        // ORDERING: Release, pairing with the Acquire loads in `Waiter::wait`.
        self.0.set.store(true, Release);
        self.0.waiter.unpark();
    }
}

impl Waiter {
    pub fn wait(&self) {
        for _ in 0..SPINS {
            // ORDERING: Acquire, pairing with the Release store in `Setter::set`.
            if self.flag.set.load(Acquire) {
                return;
            }
            spin_loop();
        }
        // ORDERING: Acquire, pairing with the Release store in `Setter::set`.
        while !self.flag.set.load(Acquire) {
            thread::park();
        }
    }
}
```

Held by review, and by a benchmark of the wake-up.

## A Busy Poll Runs Only on a Core of Its Own

A loop that polls for input with no wait, `epoll_wait` with a zero timeout or a
`try_recv` in a loop, answers the moment input lands, and uses all of its core
to do it, whether or not anything comes. So it runs only where it has a core to
itself, pinned there, on a machine that isolates that core from other work;
anywhere else, it waits in the kernel and pays the wake-up.

```text
# Bad: a zero-timeout poll on a core the scheduler shares with the rest.
poll(timeout = 0) in a loop, on any core
```

```text
# The hot thread, pinned to isolated core 6, polls with no wait; every other thread blocks.
pin(6); loop { poll(timeout = 0); drive(ready) }
```

Held by review.

## A Hot Thread Is Pinned, and Each Thread Pins Itself

A thread the scheduler moves loses its caches, and one that shares a core waits
its turn, so a thread that must answer fast is pinned to a core of its own. On
Linux, `taskset -c` pins a whole process, and `sched_setaffinity`, through the
crate the repository uses for it, pins one thread. The scheduler spreads no
thread across isolated cores: two threads of a process given two isolated cores
both run on the first, so each thread pins itself to its own.

```text
# Bad: two isolated cores for two hot threads, which then share the first.
taskset -c 6,7 ./paint
```

```text
# Each thread pins itself: the painter to 6, the uploader to 7.
./paint --painter-core 6 --uploader-core 7
```

Held by review.

## A Hot Loop Reads the Clock Once a Turn

A clock read is a call into the platform's timekeeping, cheap but not free, and
two reads in one turn disagree. So a loop reads the clock once a turn, at its
top, and passes that instant to everything the turn decides, as a reactor hands
out the time it woke: each decision sees the same now, and the loop pays for one
read. Measure what a read costs on the machine.

```rust
use std::time::Instant;

#[derive(Debug)]
pub struct Stamp {
    pub tile: u32,
    pub at: Instant,
}

pub fn stamp(tiles: &[u32], out: &mut Vec<Stamp>) {
    for tile in tiles {
        // Bad: a clock read for each tile, and each tile a different now.
        out.push(Stamp { tile: *tile, at: Instant::now() });
    }
}
```

```rust
use std::time::Instant;

#[derive(Debug)]
pub struct Stamp {
    pub tile: u32,
    pub at: Instant,
}

pub fn stamp(tiles: &[u32], now: Instant, out: &mut Vec<Stamp>) {
    out.extend(tiles.iter().map(|tile| Stamp { tile: *tile, at: now }));
}
```

Held by review.

## A Hot Thread's Memory Is Written Before Its Loop Starts

The operating system maps a large fresh allocation lazily: the first write to
each page traps into the kernel, which finds a page and maps it, so a hot path
that touches new memory pays for the page as well as the write. A hot thread's
buffers are allocated and written once before its loop starts, locked where the
kernel could otherwise take them back, and a large table read at random sits on
huge pages, since each 4 KiB page costs an entry in the translation cache and a
2 MiB page covers 512 of them. Measure the first touch against the second.

```text
# Bad: the ring allocated at startup, and first written by the hot loop.
ring = allocate(64 MiB); loop { ring.write(next) }
```

```text
# Each page of the ring written once, and the ring locked, before the loop.
ring = allocate(64 MiB); ring.write_each_page(0); lock(ring); loop { ring.write(next) }
```

Held by review, and by a benchmark that measures the first turn.
