---
name: review-security
description: Use when a change reads input from outside the process, whether an argument, the environment, a file, an archive, a socket or a message, or builds a path, a command or an allocation from one, or adds a dependency; before calling it done or opening a pull request; or when asked for a security review of a library, a CLI, a diff, a branch or a pull request. Reviews what an attacker can reach through the change, and reports each finding with its path from input to harm, what the attacker gains, and the fix.
argument-hint: "[<revision range or paths>] [-- <what the change is for>]"
context: fork
agent: Explore
model: inherit
background: false
---

# Review a Change for What an Attacker Can Reach

This pass reviews a change to a library or a CLI for what someone outside the
process can do through it, and reports each finding with the path from their
input to the harm, what they gain, and the fix. It changes nothing: the agent
that made the change knows what it is for, and fixes each finding or says why
one does not hold. The report is all that agent sees, so it stands alone.

A finding is a path: an input someone else controls, each step it takes through
the code, at its line, and a harm to someone who did not send it. A missing
defence with no such path is none, and so is a harm the sender can only do to
themselves. The attack classes say where paths are found; the path is what makes
the finding.

## Scope

The arguments, as given; an empty block is none:

```text
$ARGUMENTS
```

- **Before `--`**: a revision range, `main..HEAD` or `HEAD~2..`, or one commit,
  whose change is reviewed; or paths, whose change in the working tree and on
  the branch is reviewed, and a path with neither is reviewed whole. A pull
  request is reviewed by its branch or its range, given here, since this pass
  cannot reach GitHub.
- **After `--`**: what the change is for, and who sends its input, against which
  each boundary is judged.
- **No range or paths**: the working tree against `HEAD`, and the branch against
  the default branch, both.
- **The default branch** is the first of `origin/HEAD`, `main` and `master` that
  `git log -1 --format=%H <name>` finds; with none, only the working tree is
  reviewed, and the report says so.
- **No purpose**: read it from the commits, `git log <range>`, and say so. With
  no commits either, the report says the purpose is unknown, and infers none.

Find the change with git, each command run from the repository's root as it is
written here, since a fork cannot ask for an approval and a command written
another way, `git -C <dir> diff`, may need one; a shell loop or a `$VARIABLE` is
refused, so several files are read one command each:

- **The working tree**: `git diff -U20 HEAD`, and `git status --short
  --untracked-files=all` for files git does not track yet, each read whole.
- **The branch**: `git diff -U20 <default>...HEAD`.
- **A range**: `A..B` is diffed as `git diff -U20 A...B`, from where `B` left
  `A`; `git log A..B` lists its commits, and `git show -U20 <commit>` shows one.
- **Paths**: `-- <paths>` after any of these narrows it.
- **A large change**: `git diff --stat` over the range first, then a file at a
  time, `-U20 -- <file>`.

A range that ends anywhere but the working tree is read as it leaves each file,
with `git show <end>:<path>`, and each line number is taken from that.

Every file of the change that runs is in scope: source in any language,
`build.rs`, scripts, the justfile and `.just/`, CI workflows, and each manifest
and lock file, for what it adds. Tests, fixtures and docs are read for what they
say of the code, and are not reviewed as code an attacker reaches. Any other
file is named under Out of scope. With nothing in scope, the report is the line
"No code in scope" and what was out of scope, and the pass stops there.

## Map the Trust Boundaries

Read every changed hunk with its twenty lines of context, and further wherever
its meaning reaches past them: the whole function it sits in, the definitions of
what it calls, and each caller of what it changes. Keep count: the report says
how many files and hunks were read. A file too large to read whole is read hunk
by hunk, never skipped.

Then list each input the change reads, or makes reachable, from outside the
process, where it enters, and who controls it:

- **A CLI's arguments, options and environment** are its user's, unless
  something else sets them: a CI job, a script, a service that runs the CLI for
  others. The purpose or the code says which; with neither, they are the user's.
- **A file, a directory or an archive** is its author's: a download, a
  repository being built, a directory someone else can write. The name a user
  gives is the user's; what the file holds is its author's, unless the user
  wrote it, so a downloaded archive the user opens is its author's input.
- **Stdin, a socket, a pipe or a message** is its peer's.
- **A library's public function** takes from anywhere what its purpose says, the
  bytes a decoder reads, the name a lookup takes; the choices its caller makes,
  a limit or a base directory, are the caller's authority. A published library's
  public function is judged for every caller its purpose admits; an unpublished
  crate's callers are the workspace's, and a harm that needs a caller it lacks
  goes under Needs validation. A crate is published unless its `publish`, its
  own or the workspace's it inherits, is `false`.
- **A dependency** runs its code with the program's authority.

For each, name what its sender must not reach: a file outside what the input may
name, the process's memory, its work for others, a secret it holds, or a command
beyond the one meant. An input with nothing to reach crosses no boundary.

## Choose the Classes

Each class is a section of `references/attack-classes.md`. List them with `grep
-n '^## ' .claude/skills/review-security/references/attack-classes.md`, and read
each one a boundary needs with `Read` and an offset and a limit, from its
heading to the next, never the whole file.

- **Path traversal**: a path built from input.
- **Command injection**: a command or its arguments built from input.
- **Unbounded allocation**: a count, length or capacity read from input.
- **Deserialization without limits**: a decoder, a read to the end, or a
  recursion over input.
- **Secrets in logs or errors**: a token, key, password or credentialed URL, and
  each log line, error, `Debug` and panic that could print it.
- **Time-of-check races on files**: a check on a path, then a use of it, in a
  directory someone else can write.
- **Output from input**: a log line or a terminal's output that prints a value
  from input.
- **Panics on input**: an index, a slice, an `unwrap`, a `split_at` or an
  overflow in a decoder or a parser.
- **Size arithmetic**: arithmetic on input that sizes an allocation, a bound or
  an offset, and an `as` that narrows it.
- **Unsound `unsafe`**: an `unsafe` operation that a value from input reaches.
- **Dependency advisories**: a dependency the change adds or moves, and each
  advisory exception.
- **Shell scripts**: a script that runs input through `eval`, `sh -c` or an
  unquoted expansion.
- **Recipe arguments**: a recipe whose parameter reaches its shell line.
- **Workflow expressions**: an event's text in a workflow's `run:` step.

A path from input to harm that no class names is still a finding.

Before judging an `unsafe` operation's proof, read `writing-unsafe-rust`'s body,
`.claude/skills/writing-unsafe-rust/SKILL.md`, whole: its Where Unsafe Goes and
Proofs rules are what the proof must meet.

## Dependencies First

Where the change touches a manifest, a lock file or `deny.toml`, run `just
check-cargo-deny` before tracing. What it reports is its own to hold: the report
gives it one line, and no finding restates an advisory it names. It fetches the
advisory database, and where it cannot, Not run says so. Where the change or the
working tree touches the justfile or `.just/`, it is not run, since its recipe
is code under review, and Not run says so.

## Trace Each Path

For each boundary and each class it needs, follow the input from where it enters
to each place it is used: a path opened, a command run, an allocation, an
`unsafe` operation, a line logged. Read each function it passes through whole,
and each check on the way. Ask whether every route to the use passes the check,
error paths and other entry points included, and whether what one function
guarantees is what the next assumes: a check on a string before it is joined to
a path, a bound in bytes against a count of items. Stop once a trace is settled
either way.

## Validate Each Candidate

Before a candidate goes in the table, try to refute it, from the code as the
change leaves it:

1. **The input** enters at a real entry point, and someone other than the
   program's user controls it. A value the program computes, a constant, or a
   test's input is none.
2. **Each step** is read in the file, at its line, with nothing between that
   stops it: a bound, a type that holds one, a resolved path compared with its
   base, `create_new`, a limit. Find the strongest check on the path; one that
   holds refutes the candidate.
3. **The harm** lands on someone who did not send the input, or on a process
   others rely on, which, for a published library's public function, is any
   caller its purpose admits. What the sender can already do with their own
   authority, they gain nothing by doing here.
4. **No stronger than shown**: a panic is a crash, a failed allocation an abort,
   and neither is code execution; a crash of a CLI only its user runs is the
   user's.

A refuted candidate goes under Checked, nothing found, with what stops it. A
fact the purpose, a doc or the code states is held: it decides the candidate
here, as a finding or refuted. One that turns on a fact none of them holds,
whether a directory is shared, how callers use the library, what runs the CLI,
goes under Needs validation with that fact, and has no weight; but only where
every step of its path was read at its line, and that one named fact decides it.
Any other candidate is dropped. Nothing is run to validate a candidate: this
pass reads.

## Weigh by What the Attacker Gains

The weight is what the attacker gains once the path is taken; where two fit, the
heavier holds:

- **critical**: they run code, or write any file, as the process, from input
  anyone can send, a file, an archive or a message.
- **high**: they read or write a file outside what the input may name, break the
  process's memory safety, learn a secret it holds, or run code given a
  precondition they must arrange.
- **medium**: they stop or exhaust a process others rely on, by a panic, an
  allocation, a recursion or a loop, or forge a line in a log others act on.
- **low**: they learn what is no secret, a path or a version, or gain little for
  a sustained effort.

## The Report

The report is Markdown in this form, and nothing else:

````text
## Security review of <scope>: <what the change is for>

Purpose read from the commits.
Read <n> files and <n> hunks.
Dependencies: `just check-cargo-deny` passes, or fails with <advisories>.

Trust boundaries: <each input, where it enters, who controls it, and what it
must not reach>.

| #   | Where        | Rule                                 | Weight |
| --- | ------------ | ------------------------------------ | ------ |
| 1   | `<path>:<n>` | `attack-classes.md`, "<its heading>" | high   |

### 1. `<path>:<n>`

<Who sends what, and what they gain, in a sentence or two.>

1. Input: <what enters, at `<path>:<n>`, and who controls it>.
2. <Each step, at `<path>:<n>`, and why nothing there stops it>.
3. Harm: <what happens, at `<path>:<n>`, and to whom>.

```rust
<the fix>
```

Needs validation: <each path that turns on a fact outside the repository, the
fact, and how its owner checks it>.

Checked, nothing found: <each boundary and class checked with no finding, and
each candidate refuted, with what stops it>.

Not run: <each command refused or failed, and what it would have checked>.

Out of scope: <each file not reviewed, each path the change neither adds nor
reaches, and anything not read, with why>.
````

- **The purpose line** says "Purpose read from the commits" where it was, or
  "Purpose unknown" where nothing gave one, and is left out where the arguments
  gave it.
- **Where** is the line the fix changes, from the repository's root, as the
  change leaves it; the path names the rest. Each line number, in the table, a
  path or Needs validation, is taken from the file as read, with `Read` or `git
  show <end>:<path>`, never counted from a hunk. A fault repeated on several
  lines is one finding that names each.
- **Rule** is the heading of the class's section, as it is written; for a path
  no class names, the boundary it crosses.
- **Each finding's section** says who sends what and what they gain, then the
  path, one line a step, then the fix in a fenced block, `rust`, `bash`,
  `python`, `toml` or `text`, as the code should read, short enough to apply as
  written.
- **Needs validation** holds only what Validate Each Candidate admits to it. It
  and **Not run** are left out where they hold nothing.

Findings go critical first, then by path and line. With no finding, the table is
the line "Nothing found."

## Keeping It Honest

- **Read every changed hunk.** One that could not be read is named under Out of
  scope, with why.
- **No finding without its path**: the input, each step and the harm, each read
  in the file at its line, never inferred from a name or the diff's shape.
- **Refute before reporting**: the strongest check on the path is read, and the
  finding says why it does not hold.
- **No harm the sender does to themselves**, and none stronger than the path
  shows.
- **No finding for a missing defence alone**: a limit, a check or a flag with no
  path to harm is not reported.
- **Nothing a check holds**: an advisory, an unquoted expansion or a lint that a
  check the repository runs reports is that check's, not a finding.
- **Only the change**: a path the change neither adds nor makes reachable goes
  under Out of scope, once. A change that makes old code reachable from a new
  input owns the path. In a path reviewed whole, every line is the change.
- **Say "nothing found" rather than invent one**: a class with no finding is a
  result.
- **Change nothing, and run only the commands named here**, and read-only search
  and listing, `grep`, `find`, `ls`, `head` and `cat`; never the program under
  review, with any input, and no recipe where the change touches the justfile or
  `.just/`, since just would run the code under review.

## References

Read each reference section a boundary needs once the boundaries are mapped, and
read it again after compaction:

- `references/attack-classes.md`: before tracing a class, its section, found by
  heading.
- `references/sources.md`: before citing a source for a class, or adapting one.
