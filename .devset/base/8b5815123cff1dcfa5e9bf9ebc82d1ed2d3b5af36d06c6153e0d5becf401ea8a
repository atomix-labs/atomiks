# Sources

Read this before citing a source for an attack class in a finding, and before
adapting a class or a step from somewhere else. It says what each source holds,
which part of this pass it backs, where the pass departs from Cloudflare's
method, and the notice owed for the text it adapts. Cite one of these, or an
equally primary source; open the page before linking it.

## Canon

| source                                                                                                                                                                                    | says                                                                                                                | backs                                       |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| [CWE-22, Path Traversal](https://cwe.mitre.org/data/definitions/22.html)                                                                                                                  | a pathname from input that resolves outside the restricted directory                                                | "A Path from Input Stays Under Its Base"    |
| [`Path::join`](https://doc.rust-lang.org/std/path/struct.Path.html#method.join)                                                                                                           | "If `path` is absolute, it replaces the current path"                                                               | the same                                    |
| [CWE-78, OS Command Injection](https://cwe.mitre.org/data/definitions/78.html) and [CWE-88, Argument Injection](https://cwe.mitre.org/data/definitions/88.html)                           | input that a shell parses as a command, and input a program reads as an option                                      | every class that runs a command             |
| [`std::process::Command`](https://doc.rust-lang.org/std/process/struct.Command.html)                                                                                                      | a program run with its arguments as given, through no shell                                                         | "A Command Takes Input as Arguments"        |
| [CWE-789, Excessive Size Value](https://cwe.mitre.org/data/definitions/789.html)                                                                                                          | an allocation sized by a value from input with no bound                                                             | "A Count from Input Is Bounded"             |
| [`Vec::with_capacity`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.with_capacity) and [`handle_alloc_error`](https://doc.rust-lang.org/std/alloc/fn.handle_alloc_error.html) | a capacity past `isize::MAX` bytes panics; a failed allocation, by default, prints a message and aborts the process | the same                                    |
| [CWE-502, Deserialization](https://cwe.mitre.org/data/definitions/502.html) and [CWE-674, Uncontrolled Recursion](https://cwe.mitre.org/data/definitions/674.html)                        | a decoder that trusts what it decodes, and a recursion input drives                                                 | "A Decoder Reads to a Limit"                |
| [serde_json's `Deserializer`](https://docs.rs/serde_json/latest/serde_json/de/struct.Deserializer.html#method.disable_recursion_limit)                                                    | `disable_recursion_limit` needs the `unbounded_depth` feature; the default depth is 128                             | the same                                    |
| [CWE-532, Sensitive Information in a Log](https://cwe.mitre.org/data/definitions/532.html) and [CWE-209, in an Error Message](https://cwe.mitre.org/data/definitions/209.html)            | a secret written where others read it                                                                               | "A Secret Reaches No Log, Error or `Debug`" |
| [CWE-214, Visible Sensitive Information](https://cwe.mitre.org/data/definitions/214.html)                                                                                                 | a secret on a process's command line, which other local users can read                                              | the same                                    |
| [CWE-367, TOCTOU](https://cwe.mitre.org/data/definitions/367.html)                                                                                                                        | a resource checked, then used, while someone else can change it between                                             | "A File Is Checked Through the Handle"      |
| [`OpenOptions::create_new`](https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.create_new)                                                                                   | "No file is allowed to exist at the target location, also no (dangling) symlink"; atomic                            | the same                                    |
| [CWE-117, Log Neutralization](https://cwe.mitre.org/data/definitions/117.html) and [CWE-150, Escape Sequences](https://cwe.mitre.org/data/definitions/150.html)                           | input that forges a log line, or reaches a terminal as an escape sequence                                           | "A Value from Input Reaches a Log"          |
| [CWE-248, Uncaught Exception](https://cwe.mitre.org/data/definitions/248.html) and [CWE-129, Array Index](https://cwe.mitre.org/data/definitions/129.html)                                | an input that reaches a panic, and an index from input used unchecked                                               | "A Decoder Never Panics on Its Input"       |
| [CWE-190, Integer Overflow](https://cwe.mitre.org/data/definitions/190.html)                                                                                                              | a calculation that wraps, used for a size or an offset                                                              | "Size Arithmetic on Input Is Checked"       |
| [Cargo's profiles, `overflow-checks`](https://doc.rust-lang.org/cargo/reference/profiles.html#overflow-checks)                                                                            | off in the release profile by default                                                                               | the same                                    |
| [CWE-197, Numeric Truncation](https://cwe.mitre.org/data/definitions/197.html)                                                                                                            | a narrowing cast that cuts a length                                                                                 | the same                                    |
| [clippy's `cast_possible_truncation`](https://rust-lang.github.io/rust-clippy/stable/index.html#cast_possible_truncation)                                                                 | a cast that may truncate, in the pedantic group                                                                     | the same                                    |
| [CWE-1395, Vulnerable Third-Party Component](https://cwe.mitre.org/data/definitions/1395.html)                                                                                            | a dependency with a known vulnerability                                                                             | "A Dependency Carries No Advisory"          |
| [cargo-deny, the advisories check](https://embarkstudios.github.io/cargo-deny/checks/advisories/cfg.html)                                                                                 | every vulnerability advisory is an error; `ignore` takes `{ id, reason }`                                           | the same                                    |
| [RUSTSEC-2020-0071](https://rustsec.org/advisories/RUSTSEC-2020-0071.html)                                                                                                                | `time`'s `now_local`, `local_offset_at`, `time::now` and their kin can segfault on Unix; the example's exception    | the same                                    |
| [ShellCheck, SC2086](https://www.shellcheck.net/wiki/SC2086)                                                                                                                              | an unquoted expansion splits and globs                                                                              | "A Shell Script Runs No Input as Code"      |
| [just's manual, Positional Arguments](https://just.systems/man/en/positional-arguments.html) and [Functions](https://just.systems/man/en/functions.html)                                  | `[positional-arguments]` passes each argument as `$1` and on; `quote(s)` single-quotes a value for the shell        | "A Recipe Passes Its Arguments as Words"    |
| [GitHub Actions, Secure Use](https://docs.github.com/en/actions/reference/security/secure-use#good-practices-for-mitigating-script-injection-attacks)                                     | pass an untrusted expression to a script through an intermediate environment variable                               | "A Workflow Passes an Event's Text"         |
| [zizmor, template-injection](https://docs.zizmor.sh/audits/#template-injection)                                                                                                           | an expression expanded into a `run:` script is code injection                                                       | the same                                    |

## Where This Pass Departs

| Cloudflare's method                                                                             | here                                                                                                                                       |
| ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| a full audit of a codebase, in six phases, by parallel hunters, critics and verifiers           | one forked pass over one change, which maps, traces, refutes and reports in turn                                                           |
| a coverage ledger in JSON, checked by a validator                                               | the report's Trust boundaries and Checked, nothing found, which name each boundary and class checked                                       |
| a candidate confirmed by a bounded run in an OS-enforced sandbox                                | a candidate confirmed by reading the path at each line; the pass runs no code, and a fact it cannot read is Needs validation               |
| severity anchored on web services: authentication bypass, cross-tenant access, account takeover | weight by what an attacker gains from a library or a CLI: code or any file, then a file, memory or a secret, then a process others rely on |
| companions for web, cloud, client, mobile and model-driven targets                              | only the classes a library or a CLI meets, each gated on the profile of its language or its check                                          |

## Adapted Text

This pass adapts the method of
[cloudflare/security-audit-skill](https://github.com/cloudflare/security-audit-skill)
at commit c1c8a8c: a finding only with a concrete path across a trust boundary,
from the input to the harm; an inventory of where input enters before the hunt;
the strongest check on a path read before a candidate stands; self-impact and a
missing best practice reported as nothing; a candidate that turns on a fact
outside the source kept apart, with no severity; severity no greater than what
the path shows; and a coverage list of what was checked. The words are this
pass's own. It carries this notice:

```text
MIT License

Copyright (c) 2025-2026 Cloudflare, Inc.

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
