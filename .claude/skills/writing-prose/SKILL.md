---
name: writing-prose
description: Use when writing or revising any sentence a reader meets in the repository, in any language, whether a README, guide, book page or design document, a comment or doc comment, a commit message, pull request, changelog entry or release note, an error, log or panic message, a CLI's help, a suppression's reason or a test's assertion message; or when asked to tighten text, cut filler, or make it read as a careful person wrote it. Covers the order a reader needs, tense and voice, saying what is rather than what was done, filler, hedges and marketing, comments that say why, the form of each kind of message, commit and pull request prose, and the AI tells to avoid, ranked by strength, with the words technical text uses literally. Not for names or the shape of code, which writing-readable-code covers.
---

# Writing Prose

Every sentence in a repository has a reader who came for one fact: a user at the
top of a README, a maintainer at a comment a year on, an operator at an error, a
reviewer at a commit. This skill is how those sentences are written: the fact
first, once, in the present tense and plain words; nothing of how the text came
to be; and none of the habits that mark text as generated. It holds for every
kind of text below, in every language the repository has. The rules below are
the whole of it, each with its reason; the references hold each rule's why, a
bad and a good example, and what holds it. Names and the shape of code are
`writing-readable-code`'s: where a comment would explain a name or walk through
a tangle, the fix is there.

## Rules

### Every Sentence

1. **Put the purpose first**: the first sentence of a document, a section, a doc
   comment, a commit body or a message is the fact its reader came for, since a
   reader stops at the first sentence that answers, and skims past one that sets
   a scene.
2. **Say one fact in one place, and link it from the others**, since two copies
   drift and a reader cannot tell which is true: a function's contract in its
   doc, a flag's meaning in its help, a design's reason beside the code it
   shaped.
3. **Write in the present tense and the active voice**: "Refuses a row shorter
   than the first", not "will refuse" or "is refused", since the text is read
   while the code does what it says, and a reader looks for who acts. The
   passive stays where the actor is unknown or beside the point.
4. **Say what is, never what was done**: no "now", "new", "no longer",
   "updated", "instead of the old", no phase, ticket, review round or "as
   requested", since the reader has the code and not its history, and a word of
   change turns false once the change is old. History is git's: a commit, a pull
   request, a changelog entry, a migration note, and a deprecation or a `since`
   note are about change, and there alone the text says what changed.
5. **Write only what is so**: a number, a cost, a reason, an issue, a link or an
   output nobody checked is left out and asked for, never filled with a
   plausible guess, since the next reader takes it as true.
6. **Cut filler, hedges and marketing**: `simply`, `basically`, `note that`, `in
   order to`; `might`, `should probably`, `could potentially`; `powerful`,
   `robust`, `seamless`, `comprehensive`, `blazingly fast`. The first adds
   words, the second doubts what was checked, the third claims what nothing
   measured; where a claim needs weight, give the fact behind it. A real doubt
   stays, once, with what was not checked: "not run on Windows".
7. **Use no em dash**, nor an en dash or `--` in its place: a colon, a
   semicolon, a comma, a parenthesis or a full stop, whichever says how the
   clauses relate, since a dash leaves the reader to guess. Dashes in code,
   flags and paths stay.
8. **Keep a neutral register**: no "I", opinion, feeling, joke, exclamation mark
   or emoji in a doc, comment, message, commit or changelog, since the text
   speaks for the code, not for its writer. A reply in a review is a
   conversation, and may say "I" of what its writer did.
9. **Use concrete words, the repository's own, one for each thing**: the value,
   the unit, the row, the file, the command, each in the word the code uses,
   since a vague word makes the reader guess and a synonym reads as a second
   thing.
10. **Avoid the tells while writing, the strongest first**, since each spends
    the reader's attention on a shape instead of a fact: chat left in the text,
    and notes on how the text was made; a contrast with what no one claimed,
    "not X but Y"; a closing line that repeats the point; a saying in place of a
    claim; an opener that announces the point; an argument with no one; then
    triads by habit, stock words, inflated significance, stock `-ing` riders,
    sales language, bold labels on every item, and stacked hedges. Keep the
    words technical text uses literally: a Cargo `feature`, a map's `key`,
    `Grid::new`, syntax highlighting, memory alignment, a feature gate.

### Documents

1. **Open with the fact the reader came for, then the task, then the
   reference**, naming the reader only where two kinds read it, since a reader
   decides in the first paragraph whether to read on.
2. **A heading names what its section holds**, "How a Grid Is Read", never an
   effect, "The Magic Behind the Grid", and never stands over one sentence,
   since a reader scans the headings to find the section.
3. **Prose carries reasoning; a list holds items a reader scans or follows in
   order; a table compares along two axes**, since a reason needs the words that
   join its clauses, which a list cuts. A bold lead states its item's claim,
   never a label the sentence after it repeats.
4. **Code, paths, commands and values are in backticks, and a command shown runs
   as written**, since a reader copies it.

Headings are in title case, which rumdl holds.

A page of the book also follows `writing-the-book`.

### Comments

1. **A comment says why, not what**: the reason for a choice, an invariant, a
   consequence, a deliberate absence, which the code cannot say; never a
   narration of the line below it, which the reader has.
2. **A comment is the wrong fix for an unclear name or a tangle**: rename, or
   restructure, as `writing-readable-code` says, and the comment goes, since a
   name is read at every use and a comment only where it sits.
3. **No process residue**: no "added", "fixed", "changed to", "now", "new", "as
   discussed", "per review", phase or ticket names, rule IDs or lint narration,
   since none of it is true of the code a reader has. Commented-out code is
   deleted; git keeps it.
4. **A `TODO` names its issue, `TODO(#214)`, and says what is missing as a
   fact**, since a gap nobody tracks is never closed.
5. **A comment sits above what it explains, as a sentence, capitalized, with a
   period**, one fact a comment, so it reads before the code and holds a whole
   sentence; a suppression's reason sits where its tool reads it.
6. **A comment changes with its code, in the same commit, or goes**, since a
   comment the code contradicts is worse than none; one that repeats what a
   name, a type or a test says goes too.
7. **A terse comment that states its fact is finished**, since a longer or more
   elegant rewrite buries the fact in the flourish.

Rust's `///` and `//!` follow `writing-rustdoc`, its voice table and cut list
included. Rust source carries no `TODO`, as `writing-rustdoc` holds: the doc
states the limit, and the issue holds the work.

A `// SAFETY:`, `// INVARIANT:` or `// ORDERING:` proves what
`writing-unsafe-rust` says it must, where it says one goes.

### Messages

1. **An error states what was wrong, with the value and where it was**: "row 3
   has 7 squares, the first has 8", never "invalid input" or "something went
   wrong", since the reader has only the message.
2. **A message is a lowercase fragment with no closing period, and each in a
   chain adds only its own fact**, since a reporter prints each message of a
   chain after a prefix of its own, `error:` or `caused by:`.
3. **No apology, blame, exclamation or "please"; advice only where it surely
   helps, and then the action**: "pass a file of rows of `.` and `#`", never
   "check your input", since tone takes the reader's attention from the fact,
   and a guess sends them where they have looked already.
4. **A log line is a stable phrase with its values as fields, written once,
   where the error is handled**, and never holds a secret, a token or a user's
   data, since a log is searched by its phrase and read by more people than the
   code.
5. **A panic names the assumption that broke**, the precondition a caller missed
   or the violation found, since it fires only on a bug, and its reader is the
   one who fixes it.
6. **A suppression's reason says why the check misreads this code**, a lowercase
   clause, never the consequence ("or the linter complains") or the rule's name,
   since the next reader needs that fact to tell whether the overrule still
   holds.
7. **An assertion's message states the property, continuing the test's
   sentence**: "the ragged row is refused", never "should be equal" or "test
   failed", which any failure could print.
8. **A CLI's help is a verb phrase in the mood and case of its siblings and of
   the parser's own lines, and a flag's says what it sets and its default**,
   since it is read at the prompt, a line at a time.
9. **Read a message where it lands**: run the failure once and read what its
   reader sees, since a runtime, a reporter or a parser may print other than
   what the code wrote.

A Rust error type's message takes `writing-rust`'s form, in its
`references/errors.md`, which also says when code may panic, and how a binary
and a command a person runs report a failure.

A Rust test's names and messages follow `writing-rust-tests`.

### Commits, Pull Requests and Changelogs

1. **A subject takes the form AGENTS.md's Commits section gives**, which `just
   check-git-commits` holds, since every session reads that section; the rules
   below are what the form leaves to the writer.
2. **The subject says what changes for its reader, not which files**: "read a
   grid from text, refusing ragged rows", not "add parse_grid and GridError",
   since a reader of the log wants the outcome, and the files are in the diff.
3. **The body is prose that says why, and what holds once the change is in**,
   not a list of files or the diff retold, since the diff is one click away and
   the reason is not.
4. **A pull request's description is that prose for a reviewer**: what the
   change does, then why, how it was checked, and what is left; never "This PR",
   a heading over each bullet, or chat, since a reviewer reads it before the
   diff to know what to look for.
5. **A changelog entry says what changed for a user of the release**, in the
   user's words, one line each, since its reader decides from it whether to
   upgrade, and has not read the code.

The changelog is written from the subjects at a release, so a subject is also
its changelog line.

A wrong line is fixed by rewording its commit.

## Steps

A one-line comment, a message whose form a neighbour shows, and a commit subject
need no reference: the rules above are enough. Otherwise read only the section a
step names; the headings are the index.

1. **Any text**: read its neighbours first, the file, the module's comments, the
   README, `git log` for commits, and match their voice and words; a rule here
   that they break is named in the change, not fixed in passing.
2. **A document**: the fact, then the task, then the reference;
   `references/tells.md` before the draft is final.
3. **A comment of more than one line, a `TODO`, or deleting one**:
   `references/comments.md`; where the comment would explain a name or a tangle,
   `writing-readable-code` instead.
4. **An error, log, panic or assertion message, a suppression's reason, or a
   CLI's help**: the section of `references/messages.md` for a kind the
   neighbours do not show. Run one failure of each kind whose output you have
   not seen; a test that pins standard error replaces running it by hand.
5. **A commit body, a pull request's description or a changelog entry**: its
   section of `references/messages.md`, and `git log` for the repository's own.
6. **Revising or reviewing text**, or asked to make it read as a person wrote
   it: `references/tells.md`. Mark the tells strongest first; keep every fact
   and add none; rewrite each sentence around its point rather than swapping
   words; leave what its "When Not to Act" names.
7. **Before finishing**: read the text as its reader, then search it for the
   tells that survive a rewrite, "not just", "now", "new", a dash, a closing
   line, a bold label, skipping the literal senses tells.md lists; then the
   checks below.

## Checks

- `just check`: every check, as CI runs them, after `just fix`, which formats
  what a formatter can.
- `just check-spelling`: typos, over code, documents and configuration.
- `just check-markdown`: rumdl, headings in title case among its rules.
- `just check-git-commits`: the branch's commits, as Conventional Commits.
- `just check-rust-doc`: with the doc lint, which refuses an em dash, a `TODO`,
  a phase or a rule ID in a Rust comment, and filler and marketing in a doc.

Nothing else reads the words: the rest is held by review, the writer's first.

## What Not to Do

| Thought                                             | Instead                                                       |
| --------------------------------------------------- | ------------------------------------------------------------- |
| "`// Now uses a map for faster lookups`"            | The reason as a fact: `// A map: lookups by square dominate.` |
| "A comment on each step helps the reviewer"         | The reviewer reads the code; a comment says what it cannot.   |
| "Keep the old approach in a comment for context"    | History is git's; the comment says what is.                   |
| "`TODO: handle this later`"                         | Fix it, or open the issue; Rust source carries no `TODO`.     |
| "`Error: Invalid input!`"                           | `row 3 has 7 squares, the first has 8`.                       |
| "`failed to parse: failed to read: …`"              | Each layer adds its own fact once.                            |
| "`feat: Add comprehensive grid parsing`"            | `feat(core): read a grid from text, refusing ragged rows`.    |
| "A `## Summary` and a bullet per file in the body"  | Prose: what changes, why, how it was checked.                 |
| "A dash reads naturally here"                       | A colon, semicolon, comma or parenthesis, by what follows.    |
| "Sentence case reads better in headings"            | The repository's case: title case, which rumdl holds.         |
| "This terse comment would read better at length"    | A line that states its fact is finished.                      |
| "It isn't just a parser: it guards the whole board" | Say what it does: reads a grid, refusing ragged rows.         |

## References

Read a reference, or its section, when a task calls for it, and again after
compaction: this body is the summary, and the examples are there.

- `references/messages.md`: before a kind of message the neighbours do not show,
  that kind's section; before a pull request's description or a changelog entry,
  its section.
- `references/comments.md`: before a comment of more than one line, a `TODO`, or
  deleting one; in Rust, `writing-rustdoc`'s `references/comments.md` instead.
- `references/tells.md`: before a document, a pull request's description or a
  changelog is final, and when revising or reviewing text.
- `references/sources.md`: before citing a source for a rule, or adapting text
  from one.
