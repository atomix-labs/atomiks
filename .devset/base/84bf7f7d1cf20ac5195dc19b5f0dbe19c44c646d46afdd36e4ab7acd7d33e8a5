# Running

Read this before running tests by hand, when a test fails, hangs or passes only
sometimes, before ignoring a test or making tests run one at a time, and before
touching `.config/nextest.toml`. It says what runs the tests here, what each
runner does and does not run, and how to reach one test.

## `just check-cargo-nextest` Runs Every Test, Then the Doctests

The recipe runs `cargo nextest run --workspace --all-features`, then `cargo test
--workspace --all-features --doc`, since nextest cannot run doctests: its
documentation says so, and a doctest is left to Cargo. `just check` runs the
recipe with every other check. Under CI, nextest's `ci` profile runs every test
however many fail, prints each failure as it happens and again at the end, and
writes a JUnit report to `target/nextest/ci/junit.xml`.

```text
# Bad: a doctest is never run, and a documented example that no longer compiles passes.
cargo nextest run --workspace
```

```text
just check-cargo-nextest
```

Held by the recipe, which `just check` and CI run.

A benchmark is no test: nextest never runs it, and `cargo bench` runs it as
`tuning-rust-performance` says.

## Reach One Test by Its Name

A filter after the options runs the tests whose names hold it, across the
workspace or one crate; `-E` takes nextest's filter expressions for more, such
as one test by its exact name or every test in a module. A failing test is run
alone first, so its output is its own.

```text
# Bad: every test in the workspace, to watch one.
cargo nextest run --workspace --no-capture
```

```text
cargo nextest run -p tiles a_step_past_the_last_column
cargo nextest run -p tiles -E 'test(=board::tests::a_step_past_the_last_column_is_refused)'
cargo nextest run -p tiles -E 'test(/^board::/)' --no-capture
```

Held by review. `--no-capture` shows what a test prints as it runs, and runs the
tests one at a time to keep their output apart.

## Each Test Runs in a Process of Its Own

nextest starts a process for each test, so a test's globals, its environment and
its working directory end with it, and a crash takes down that test alone.
`cargo test` runs each binary's tests as threads of one process, and so does a
loom model's run. A test that passes under nextest and fails under `cargo test`
shares something with another test, which `writing-tests.md` says how to remove.

```text
# Bad: nextest alone, where a test touches a global, so one that leans on another's leftovers passes.
cargo nextest run -p tiles
```

```text
cargo nextest run -p tiles
cargo test -p tiles
```

Held by review.

## An Ignored Test Runs Where What It Needs Is

An `#[ignore = "…"]` test is skipped by every run the checks make. It runs by
hand, where what its reason names is there: `--run-ignored only` runs the
ignored tests alone, and `--run-ignored all` runs them beside the rest. No
recipe runs them, so a change to what one tests runs it before it is done.

```text
# Bad: the ignored test that pins the change never ran.
just check-cargo-nextest
```

```text
cargo nextest run -p tiles --run-ignored only
```

Held by review.

## Tests That Cannot Share the Machine Run in a Test Group

What a test makes outside the process it makes fresh: a directory of its own, a
socket on port 0. What cannot be made fresh, a core a test pins, a device the
machine has one of, the build directory trybuild compiles its fixtures in, is
shared, and the tests that contend for it run one at a time in a nextest test
group, in `.config/nextest.toml`, whose `[test-groups]` and `[profile.default]`
keys are the repository's: the cargo-nextest profile owns only the `ci`
profile's. An override in `profile.default` holds under `ci` too. Tests outside
the group run as before.

```text
# Bad: every test in the workspace one at a time, for the two that drive the tile display.
cargo nextest run --workspace --test-threads 1
```

```toml
[test-groups.tile-display]
max-threads = 1

[[profile.default.overrides]]
filter     = 'test(/on_the_tile_display/)'
test-group = 'tile-display'
```

Held by review. A filter that names a binary or a package the workspace lacks
fails to parse, so a group names only what the workspace has.

## A Flaky Test Is Found by a Retry, and Fixed

A test that fails one run in fifty has a race, a sleep or a shared global. A
retry is how it is found: with retries set, nextest runs a failed test again,
and reports one that passes on a retry as FLAKY, while the run itself passes.
That report is a fault to fix, never a pass to accept. How many retries is the
repository's to set, `retries` in its `.config/nextest.toml`, and
`flaky-result = "fail"` fails the run on a flaky test, so none goes unread.
`--stress-count` runs one test until it fails, which is how its race is shown,
and then its fix.

```text
# Bad: the run passes, and its FLAKY line is taken for a pass.
   FLAKY 2/3 [   0.003s] (1/1) tiles tests::a_painter_reports_its_tile
     Summary [   0.007s] 1 test run: 1 passed (1 flaky), 0 skipped
```

```text
cargo nextest run -p tiles --stress-count 200 a_painter_reports_its_tile
```

Held by review.

## A Test That Hangs Is Stopped, and Says Where

nextest marks a test SLOW once it runs past a period, 60 seconds by default, and
stops nothing on its own, so a test that deadlocks holds the run until CI's job
times out. A repository whose tests can hang sets `slow-timeout` with a
`terminate-after`, so nextest ends the test and names it; the test's own waits
have deadlines, as `writing-tests.md` says, so it fails with a message first.

```text
# Bad: in `.config/nextest.toml`, nothing ever ends a hung test.
[profile.default]
slow-timeout = "60s"
```

```toml
[profile.default]
slow-timeout = { period = "60s", terminate-after = 3 }
```

Held by review.
