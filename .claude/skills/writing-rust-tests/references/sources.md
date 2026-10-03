# Sources

Read this before a rule of this skill needs backing, before citing a source in a
doc or a review, and before adapting a rule from somewhere else. It says what
each source holds, which rule it backs, where this skill departs from it, and
the notice owed for the text it adapts. Cite one of these, or an equally primary
source; open the page before linking it.

## Canon

| source                                                                                                                  | says                                                                                                                                                                                                                                         | backs                                                 |
| ----------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| [The Rust Reference, Testing attributes](https://doc.rust-lang.org/reference/attributes/testing.html)                   | test functions "are only compiled when in test mode"; `ignore` takes a reason; with `expected`, "the given string must appear somewhere within the panic message"; under `should_panic`, "the return type of the test function must be `()`" | `writing-tests.md`                                    |
| [The Cargo Book, Package layout](https://doc.rust-lang.org/cargo/guide/project-layout.html)                             | integration tests are `tests/*.rs`, and a test of several files is `tests/<name>/main.rs`                                                                                                                                                    | `layout.md`                                           |
| [The Rust Book, Test organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)                       | unit tests in `#[cfg(test)] mod tests` beside the code; each file in `tests/` a crate of its own; shared code in `tests/<dir>/mod.rs`, which is not a test crate                                                                             | `layout.md`                                           |
| [cargo-nextest, Running tests](https://nexte.st/docs/running/)                                                          | "Doctests are currently not supported"; run them with `cargo test --doc`; filters, `--run-ignored`, `--no-capture`                                                                                                                           | `running.md`                                          |
| [cargo-nextest, Why process-per-test](https://nexte.st/docs/design/why-process-per-test/)                               | "nextest runs each test in a separate process"; "singletons become separated per-test"                                                                                                                                                       | `running.md`, `writing-tests.md`: a test stands alone |
| [cargo-nextest, Test groups](https://nexte.st/docs/configuration/test-groups/)                                          | `max-threads = 1` "is similar to `cargo test` with the `serial_test` crate, or a global mutex"; "tests that aren't part of a test group are not affected"                                                                                    | `running.md`, `compile-fail.md`                       |
| [cargo-nextest, Per-test overrides](https://nexte.st/docs/configuration/per-test-overrides/)                            | an override in `profile.default` applies to every profile that inherits from it                                                                                                                                                              | `running.md`                                          |
| [cargo-nextest, Retries and flaky tests](https://nexte.st/docs/features/retries/)                                       | a pass on a retry is reported FLAKY and "treated as ultimately successful" by default; `flaky-result = "fail"`                                                                                                                               | `running.md`                                          |
| [cargo-nextest, Slow tests](https://nexte.st/docs/features/slow-tests/)                                                 | a test is marked slow after 60 seconds by default, and not terminated; `slow-timeout = { period, terminate-after }`                                                                                                                          | `running.md`                                          |
| [trybuild](https://docs.rs/trybuild/latest/trybuild/)                                                                   | a missing `.stderr` is written to `wip/`; `TRYBUILD=overwrite` writes in place, and "check `git diff` afterward"; `pass` cases compile and run; fixtures reach the dev-dependencies                                                          | `compile-fail.md`                                     |
| [proptest](https://docs.rs/proptest/latest/proptest/)                                                                   | `proptest!`, strategies and `prop_map`; `prop_assert!`; 256 cases and `PROPTEST_CASES`; 1,024 global rejections; regressions saved beside the source, "recommended to check this file in to source control"                                  | `properties.md`                                       |
| [rstest](https://docs.rs/rstest/latest/rstest/)                                                                         | `#[rstest]` with `#[case]`, each case a test; `#[case::description(…)]` names it                                                                                                                                                             | `properties.md`                                       |
| [The Rust Fuzz Book, cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html)                                      | `cargo fuzz init`, targets, the corpus and artifacts; `cargo fuzz tmin` and `cmin`; libFuzzer's `-runs`                                                                                                                                      | `properties.md`                                       |
| [The rustdoc book, Documentation tests](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html) | `compile_fail` passes when the example fails to compile, for any reason                                                                                                                                                                      | `compile-fail.md`                                     |
| [clippy's lint list](https://rust-lang.github.io/rust-clippy/master/index.html)                                         | `items_after_test_module`, `manual_assert_eq`, `should_panic_without_expect`, `ignore_without_reason`, `tests_outside_test_module`, `redundant_test_prefix`, `panic_in_result_fn`                                                            | each rule "held by" one                               |
| [clippy's configuration](https://doc.rust-lang.org/clippy/lint_configuration.html)                                      | `allow-unwrap-in-tests`, `allow-expect-in-tests` and the rest cover `#[test]` functions and `#[cfg(test)]` modules                                                                                                                           | `layout.md`, `writing-tests.md`                       |

## Where the Workspace Departs

| a source says                                                                             | here                                                                                                           |
| ----------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| rust-skills `test-use-super`: `use super::*` in a test module                             | each name imported                                                                                             |
| rust-skills `test-descriptive-names`: `function_condition_expected_result`, `when_…_then` | the property as a sentence, its subject first                                                                  |
| rust-skills `test-arrange-act-assert`: `// Arrange`, `// Act`, `// Assert` comments       | none; a comment explains a setup that is not obvious, never the assertion                                      |
| rust-skills `test-integration-dir`: `tests/common/mod.rs`                                 | `tests/testing/mod.rs`, as `testing.rs` in a crate, declared `#[cfg(test)]`                                    |
| rust-skills `test-mock-traits`, `test-mockall-mocking`: mockall for dependencies          | a hand-written fake of the trait; a mock framework only where a repository has one                             |
| rust-skills `test-snapshot-testing`: insta for rendered output                            | `assert_eq!` on the rendered string; trybuild's `.stderr` are the only snapshots                               |
| rust-skills `test-proptest-properties`: `assert_eq!` inside `proptest!`                   | `prop_assert_eq!`, which reports the smallest input once                                                       |
| rust-skills `test-criterion-bench`: benchmarks beside the tests                           | not tests; a benchmark is measured, not asserted                                                               |
| rust-skills `test-doctest-examples`: `cargo test` runs the doctests                       | nextest runs the tests and cannot run doctests, so `just check-cargo-nextest` runs `cargo test --doc` after it |

## Corrected Here

Claims of rust-skills that fail on the pinned toolchain or against the sources
above, and what this skill says instead:

- `#[should_panic]` on a test that returns `Result` does not compile: "functions
  using `#[should_panic]` must return `()`".
- A bare `#[should_panic]` passes on any panic;
  `clippy::should_panic_without_expect` refuses it.
- mockall's chained `.returning(…)` keeps the last closure: three calls give 3,
  3, 3, not 1, 2, 3 (mockall 0.15). A value for each call is an expectation for
  each, `.times(1)`.
- `#[tokio::test]` runs on a current-thread runtime, not a multi-thread one.
- `tests/api/mod.rs` is no test target: Cargo builds `tests/*.rs` and
  `tests/*/main.rs`, and the tests beside a lone `mod.rs` never run.
- A `#[test]` function outside `#[cfg(test)]` is not compiled into a release
  build; the helpers beside it are, and warn as dead code.
- `std::env::set_var` is `unsafe` in edition 2024, which v1.0.0's
  `test-fixture-raii` leaves out; v1.5.1 adds it. A test that sets the
  environment races the others in `cargo test`'s one process.
- `RUSTFLAGS="--cfg loom"`, in v1.5.1's `test-loom-concurrency`, replaces the
  rustflags the workspace's Cargo configuration sets; `--config` adds to them.

## Adapted Text

Rules of this skill adapt rules of
[leonardomso/rust-skills](https://github.com/leonardomso/rust-skills), at
v1.0.0, the copy sockudo vendors, and at v1.5.1, rewritten for edition 2024,
cargo-nextest and this workspace, and checked by compiling and running: unit
tests in `#[cfg(test)]` modules, integration tests in `tests/`, descriptive
names, `#[should_panic]`, fixtures that clean up on drop, traits for doubles,
proptest properties, doctests, and loom for concurrent code.

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
