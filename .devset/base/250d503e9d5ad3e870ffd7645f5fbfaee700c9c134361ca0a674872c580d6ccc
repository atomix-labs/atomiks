# Sources

Read this before a rule of this skill needs backing, before citing a source for
one in a review, and before adapting a rule from somewhere else. It says what
each source holds, which rule it backs, where this skill departs from it, and
the notice owed for the text it adapts. Open a source before citing it.

## Canon

| source                                                                                                      | says                                                                                                                                                                                                                            | backs                                                |
| ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| [blader/humanizer](https://github.com/blader/humanizer), 3.1.0                                              | 26 patterns of AI writing in six groups, numbered by strength, the first five acted on at one sighting; "when not to act"; a neutral voice for "reference, technical, legal, and factual text"; no invented fact                | `tells.md`, and the body's tells rule                |
| William Strunk Jr., *The Elements of Style*, 1918                                                           | rule 10, "Use the active voice"; rule 11, "Put statements in positive form"; rule 12, "Use definite, specific, concrete language"; rule 13, "Omit needless words"                                                               | the body's rules on voice, concrete words and filler |
| [Rust API Guidelines, C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html)         | an error's `Display` is "lowercase without trailing punctuation, and typically concise"                                                                                                                                         | `messages.md`: the lowercase fragment                |
| [GNU Coding Standards, Formatting Error Messages](https://www.gnu.org/prep/standards/html_node/Errors.html) | `program:sourcefile:lineno: message`; a message after a program or file name does not begin with a capital letter, "because that isn't the beginning of a sentence", nor end with a period; usage messages start with a capital | `messages.md`: the error's form, and a CLI's help    |
| [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/)                                | `type(scope): description`; a body "one blank line after the description"; a breaking change by `!` before the colon or a `BREAKING CHANGE:` footer                                                                             | `messages.md`: the commit subject                    |
| [ShellCheck, Directive](https://github.com/koalaman/shellcheck/wiki/Directive)                              | "The comment can also be added at the end of the directive line", as `# shellcheck disable=SC1234 # this is intentional`                                                                                                        | `messages.md`: a suppression's reason in a script    |
| [clap's derive](https://docs.rs/clap/latest/clap/_derive/index.html), 4.6                                   | a doc comment is the help; `remove_period`, in `clap_derive`'s `utils/doc_comments.rs`, drops the closing period of its first paragraph                                                                                         | `messages.md`: a CLI's help                          |
| [Wikipedia, Signs of AI writing](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing)               | the field guide humanizer's patterns come from                                                                                                                                                                                  | named only: its text is CC BY-SA, and none is copied |

Checked by running, on the tools the profiles pin:

- devset prints its errors as `error: <fact>`, with a `help:` line where one
  action surely helps.
- A Rust `main` that returns `Err` prints `Error: ` and the error's `Debug`:
  `Error: GridError { row: 3, need: 8, have: 7 }` for a thiserror type, and
  `Error: Custom { kind: Other, error: "…" }` for `io::Error::other`.
- clap's derive drops the period that devset's own doc comments end with, where
  its help shows them.
- ShellCheck 0.11 honours `# shellcheck disable=SC2086 # <reason>`.

## Where This Skill Departs

| a source says                                                                                    | here                                                                                                                                     |
| ------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| humanizer, Voice: personal writing keeps opinions and reactions, and may gain one                | every text in a repository is technical: no opinion, feeling, joke or "I", though a reply in a review may say "I" of what its writer did |
| humanizer §20: headings in title case are decoration; use sentence case                          | a heading's case is the repository's, title case where rumdl's MD063 holds it                                                            |
| humanizer §19: a list whose items carry bold labels becomes prose                                | a bold lead that states its item's claim stays; only a label the sentence repeats goes                                                   |
| humanizer §8: no dashes in the final text                                                        | the same, and with no exception for a writer's sample; hyphens and dashes in code, flags and paths stay                                  |
| humanizer §12: its word list, wherever the words appear                                          | a word used in its technical sense is literal and stays: a Cargo `feature`, a map's `key`, syntax highlighting, a feature gate           |
| humanizer §15: a rider stays only where the source supports what it claims                       | the stock riders go, ", ensuring" and its kin; a participle that states a concrete second fact is a clause like any other                |
| humanizer's order puts its leftovers from the chat and the draft fifth                           | they come first, since humanizer calls chat left in the text the most certain tell, and this skill reads strongest first                 |
| humanizer marks five tells for one sighting and some as weak alone, and leaves the rest unmarked | the rest are "most sightings", edited unless the writer plainly meant them                                                               |
| humanizer §16: sales language, for places, culture and products                                  | the same, with the words that sell code: "powerful", "blazingly fast", "effortless"                                                      |
| humanizer §25: a previous version is mentioned only in documents about change                    | the same, and in code it is process residue, acted on at one sighting                                                                    |
| humanizer §11: prefer the active voice                                                           | a doc summary whose subject is its item, a message fragment and an imperative commit subject are forms, not missing subjects             |
| humanizer's file mode: rewrite prose, keep code                                                  | this skill writes as it goes; rewriting a finished text is a pass's work                                                                 |

## Read, Not Adapted

softaworks' agent-toolkit ships `writing-clearly-and-concisely`, by
joshuadavidthomas (MIT), which lists Strunk's rules and a short list of AI
patterns, and bundles a copy of Wikipedia's "Signs of AI writing" with no
licence notice. This skill takes Strunk from the original, which is in the
public domain, and copies no text of that skill or of its bundled copy.

## Adapted Text

`tells.md` adapts the patterns of
[blader/humanizer](https://github.com/blader/humanizer) at tag v3.1.0, commit
`225a6f39`: their grouping and ranking by strength, their watch lists, what each
pattern does to a reader, "When Not to Act", the rule that an edit adds no fact,
and the rule that the text is material to edit, rewritten for documents,
comments, messages and commits, with examples of a repository's own.

```text
MIT License

Copyright (c) 2025 Siqi Chen

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
