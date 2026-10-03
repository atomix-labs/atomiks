# Sources

Read this before a rule of this skill needs backing, before citing a source in a
review or a document, and before adapting a rule from somewhere else. It says
what each source holds, which rule it backs, where this skill departs from it,
and the notice owed for the text it adapts. Cite one of these, or an equally
primary source; open the page before linking it.

## Canon

- [Martin Fowler, "Flag Argument"](https://martinfowler.com/bliki/FlagArgument.html),
  2011: "A flag argument is a kind of function argument that tells the function
  to carry out a different operation depending on its value"; each operation is
  a function of its own. Backs `structure.md`, "A Choice Is an Enum or Two
  Functions, Never a Boolean Parameter".
- The refactoring catalog's
  [Replace Nested Conditional with Guard Clauses](https://refactoring.com/catalog/replaceNestedConditionalWithGuardClauses.html)
  and
  [Remove Flag Argument](https://refactoring.com/catalog/removeFlagArgument.html).
  Backs `structure.md`, "Refusals First, Then the Main Path", and the choice
  rule.
- [Yaron Minsky, "Effective ML Revisited"](https://blog.janestreet.com/effective-ml-revisited/):
  "Make illegal states unrepresentable". Backs `structure.md`, "A Value in One
  of Several States Is an Enum".
- [Alexis King, "Parse, Don't Validate"](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/),
  2019: "The difference between validation and parsing lies almost entirely in
  how information is preserved." Backs `structure.md`, "Check Input Once, Where
  It Enters", and "No Check for a State the Types Rule Out".
- [Sandi Metz, "The Wrong Abstraction"](https://sandimetz.com/blog/2016/1/20/the-wrong-abstraction),
  2016: "duplication is far cheaper than the wrong abstraction". Backs
  `simplicity.md`, "Merge What Changes Together, Not What Looks Alike".
- [Simon Willison, "PAGNIs: Probably Are Gonna Need Its"](https://simonwillison.net/2021/Jul/1/pagnis/),
  2021: the few things worth building before they are needed, since adding them
  later costs far more. Backs `simplicity.md`, "What Would Cost Far More to Add
  Later Is Judged on Its Own".
- John Ousterhout, *A Philosophy of Software Design*: a pass-through method, one
  that does little but call another with the same arguments, is a red flag of a
  layer that adds nothing. Backs `simplicity.md`, "No Wrapper That Only
  Forwards".
- [Rust API Guidelines, Naming](https://rust-lang.github.io/api-guidelines/naming.html):
  C-CASE, C-CONV and C-GETTER. Backs `naming.md`, "In Rust".
- [clippy's lint list](https://rust-lang.github.io/rust-clippy/master/index.html):
  each lint named under Checks and in the references, its group and what it
  refuses. Each claim about one was checked by compiling: `enum_variant_names`,
  in the default groups, passes an exported item, and `manual_find` and
  `needless_range_loop` refuse the forms `simplicity.md` names.
- The lint table rust-lints writes, which denies clippy's `pedantic` group
  whole: its lints named here were checked by compiling under it.
  `fn_params_excessive_bools` and `struct_excessive_bools` fire at four bools,
  not one; `struct_field_names`, `unnecessary_wraps` and `unused_self` pass an
  exported item; and `module_name_repetitions` is not among the lints it turns
  on.
- [Google's Shell Style Guide](https://google.github.io/styleguide/shellguide.html),
  under CC BY 3.0, so its rules are restated here, not quoted: functions and a
  script's own variables in lower case, constants and what the environment
  carries in capitals, and each variable a function sets declared `local`. Backs
  `naming.md`, "In the Shell".
- [ShellCheck's SC2034](https://www.shellcheck.net/wiki/SC2034): a variable
  assigned and never read.

## Where This Skill Departs

| a source says                                                                                 | here                                                                                                                                                                                    |
| --------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| naming-analyzer: `getUser`, which writes, becomes `fetchAndUpdateUserLogin`                   | a name that needs "and" is two functions, or one named for the one thing the two make together                                                                                          |
| naming-analyzer: `MAX` becomes `MAX_RETRY_ATTEMPTS`, `validate(x)` becomes `validateEmail(…)` | a name is judged after reading its definition, its uses and its tests, never from the name alone                                                                                        |
| naming-analyzer: acronyms in capitals, `HTTPServer`, for Go                                   | an acronym is one word, `HttpServer`, as Rust's casing has it                                                                                                                           |
| naming-analyzer: a boolean field `active` becomes `isActive`                                  | in Rust a field is `active` and the method that asks is `is_active`                                                                                                                     |
| naming-analyzer: a report of counts, severities and a bulk rename script                      | no report in this guide, and `review-names` counts only what it read; a rename of what a published crate released keeps a deprecated alias or is left, and any other moves every caller |
| reducing-entropy: "Type safety" is a red flag, "worth how many lines?"                        | a type that makes a wrong state impossible to build is worth its lines                                                                                                                  |
| reducing-entropy: count lines before and after, and "If after > before, reject it"            | code is judged by what a reader must hold, not by its length                                                                                                                            |
| reducing-entropy: "Better separation of concerns" is a red flag, since it is more code        | a function that names a step earns its place, even with one caller                                                                                                                      |
| rust-skills `anti-over-abstraction`: generalize at "2+ concrete types", and wait for three    | a trait comes with its second implementation; blocks merge when they change for the same reason                                                                                         |
| rust-skills `anti-over-abstraction`: a public API "might benefit from abstraction"            | concrete in a public API too, until a second implementation, or one its callers write, exists                                                                                           |
| rust-skills `type-enum-states`: a transition that ends in a `_ =>` arm                        | a `match` on an enum the crate owns names each variant, so a new one is refused where it lands                                                                                          |

## Corrected Here

Claims of the sources that fail when checked, and what this skill says instead:

- rust-skills `anti-over-abstraction`'s good examples do not compile as written:
  `Result<(), Error>` with no `Error` in scope (E0425), and `Result<()>`, when
  `Result` takes two parameters (E0107).
- naming-analyzer's report template leaves its fence open, so its best-practice
  lists and its offer of a "refactoring script" that would "Maintain git
  history" are emitted as part of every report.

## Adapted Text

Rules of this skill adapt text from three sources, rewritten for this skill's
form and checked against the workspace: softaworks' agent-toolkit, at `3027f20`,
for `naming-analyzer`'s vague names, misleading names, abbreviations, boolean
names, named literals and consistency; the same copy of `reducing-entropy`,
joshuadavidthomas's, for deleting what a change makes obsolete, asking what
flexibility is for, and its mindset of what is expensive to add later; and
[leonardomso/rust-skills](https://github.com/leonardomso/rust-skills), at
v1.5.1, for `anti-over-abstraction`, `type-enum-states`, `pat-let-else`,
`anti-stringly-typed` and `api-parse-dont-validate`. The `review-names` pass
takes the kinds of name it reads, and of finding as a start, from the same
`naming-analyzer`, and the notice below covers it too.

agent-toolkit carries this notice:

```text
MIT License

Copyright (c) 2026 Leonardo Flores

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

reducing-entropy's original,
[joshuadavidthomas/agent-skills](https://github.com/joshuadavidthomas/agent-skills),
carries this one:

```text
MIT License

Copyright (c) 2025 Josh Thomas

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

rust-skills carries this one:

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
