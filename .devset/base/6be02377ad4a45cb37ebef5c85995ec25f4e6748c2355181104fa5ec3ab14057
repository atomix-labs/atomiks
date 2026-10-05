---
name: writing-readable-code
description: Use when writing, changing, refactoring or reviewing code in any language so the next person can read and change it, whether naming a module, type, function, variable, field or constant; splitting a long function or flattening nested conditionals; choosing between flags and an enum, a boolean parameter and two functions, a trait and a concrete type; deciding whether a helper, wrapper, option, check or layer earns its place; or deleting dead or commented-out code. Covers names, functions and their levels, control flow, states as types, speculative generality, dead code and consistency with the codebase. Rust's own naming, layout and API conventions are writing-rust's. Not for comments, doc comments or messages, which writing-prose covers.
---

# Writing Readable Code

Code is read far more often than it is written, by someone who did not write it
and has only the code to go on. This skill is for the shape of that code, in any
language: names that say what a thing is in the domain's words; functions that
each do one thing at one level; control flow that reads from the top down;
states the types hold, so a wrong one cannot be built; no machinery for a case
nobody has; nothing left that no longer runs; and all of it in the codebase's
own style. The rules below are the whole of it, each with its reason. The
references hold each rule's why, a bad and a good example, and what holds the
rule.

The sentences in code, its comments, doc comments and messages, are
`writing-prose`'s. Where a comment explains what the code does, a better name or
shape says it instead: change the code, and write the comment that stays, the
one that says why, by `writing-prose`.

In Rust, `writing-rust` holds the language's own conventions for names, module
layout, API shape and ownership, the names in its `references/naming.md`. This
skill holds what they do not, and its references name the heading of
`writing-rust` where the two meet.

## Rules

### Consistency

1. **Read the code around a change before writing it, the file, its neighbours,
   its callers and its tests, and match their words, names, shape and idioms**,
   since a reader learns a codebase's patterns once, and a second way of doing
   one thing reads as a second meaning.
2. **The codebase's way wins over a local preference; where its way breaks a
   rule here, new code follows the rule without fixing the old in passing**, and
   the change's description names what it left, so a change does one thing and a
   clean-up is reviewed as one.

### Names

1. **A name says what the thing is or does, in the domain's words**: `square`,
   `tile`, `place`, never `data`, `info`, `item`, `value`, `obj`, `manager`,
   `handler`, `processor`, `util` or `helper`, since a word that fits anything
   tells the reader nothing. Where no domain word fits, the thing is not yet one
   thing.
2. **A name does not repeat its context**: `grid::Cursor`, not
   `grid::GridCursor`; `grid.columns()`, not `grid.grid_columns()`; `tiles`, not
   `tile_vec`, since the reader already sees the module, the receiver and the
   type.
3. **A function's name says all it does**: one that asks changes nothing, one
   that needs "and" is two functions, and one that hides a write, a wait or an
   allocation misleads, since a caller trusts the name and not the body.
4. **A name is whole words, never a fragment**: a word whose subject the reader
   cannot find in the name, its type, its pair or the code read with it, `at`,
   `held`, `fresh`, `want`, leaves them to ask at what or held by whom, so no
   field, method or variable takes one; `expected` and `actual`, `required` and
   `available` are whole, each read against its pair. An error's variant or unit
   type names its whole condition, `RowTooShort`, never `Short`. A short word
   stays where the code around it says what it holds, `next` beside the
   `checked_add(1)` that makes it, and a wide reach takes more words; a dropped
   binding is named for what it holds, `_unsent_tile`, never `_outcome`; an
   abbreviation only as the domain writes it, `id`, `url`, `io`, `utf8`, and
   `rx` and `tx` for a channel's two ends.
5. **A literal that means something is a named constant**, `MAX_COLUMNS`, not
   `64` in three places, so its meaning and its value are written once.
6. **A condition is named for what is true, `is_visible`, never `!is_hidden`,
   and a compound condition gets a name**, since each negation is a step the
   reader takes.
7. **One word for one thing**, in names, docs, messages and tests, so a reader
   never wonders whether two words are two things.
8. **A type's last word says its role, and a module is named for what it
   holds**, in any language, as `writing-rust`'s `references/naming.md`, "A
   Type's Name Says Its Role", and its `references/layout.md`, "A Module Is
   Named for What It Holds", say, since a catch-all name takes in whatever comes
   next.

### Functions

1. **A function does one thing, at one level: its body reads as the steps its
   name promises, each as far from the machine as the others**, and a step that
   drops into detail, index arithmetic or byte parsing, among steps that do not
   becomes a function with a name, since a reader follows one level at a time.
2. **Extract a function to name a step, never only to save lines, and inline one
   whose name says no more than its body**, since each call is a jump the reader
   takes, and only a name repays it.
3. **Where a comment says what the next lines do, the code says it instead: a
   name for the value, a function for the step, a type for the unit**, since a
   name is checked at every use and a comment by no one; a banner inside a
   function, `// Step 2: validate`, is a function asking to be split.

### Control Flow

1. **Refusals and trivial cases first, each ending in a return, so the main path
   runs down the left margin: early returns, `?` and `let … else` over nesting,
   and no `else` after a `return`**, since each level of nesting is a condition
   the reader holds. A refusal returns early; a choice between two values stays
   an `if … else` or `match` expression.
2. **Branch on a value once, with one `match` over its cases, not a chain of
   `if`s that test it again**, so each case is written once and a new one is
   added in one place.

### States as Types

1. **A value in one of several states is an enum whose variants carry each
   state's data, never flags and optional fields**, so a combination that cannot
   happen cannot be built, and every use handles every state.
2. **No boolean parameter that picks what the function does: an enum whose
   variants name the choice, or two functions**, `place(index, tile,
   Clash::Refuse)` or `place` and `replace`, never `place(index, tile, true)`,
   since `true` at a call site says nothing of what it chose. A `bool` that is
   the value being set or stored, `set_visible(visible)`, is data, and stays.
3. **Check input once, where it enters, and turn it into a type that holds the
   proof; past that point, code takes the type and trusts it**, so a check is
   written once and cannot be forgotten.
4. **No check for a state the types rule out**: no guard for an empty collection
   that cannot be empty, no arm for a `None` that cannot come, no second bounds
   check on a position already checked, no fallback for an error that cannot
   occur, since a check tells its reader the state can happen, and they go
   looking for how.

### Simplicity

1. **Build for the callers there are: no trait or interface with one
   implementation, no generic with one type, no option, parameter or setting
   nobody sets, and no hook, registry or factory for a second case nobody has**,
   since each is a concept to learn and a path to keep working, paid for now for
   a future that tends to arrive in another shape. A test double counts as an
   implementation, and an extension trait on another crate's type, or a sealed
   trait a published crate keeps so it can grow, is no guess. A request that the
   code be flexible is met by code that is easy to change, concrete and small,
   and the change's description says where the seam will go when the second case
   comes.
2. **No wrapper that only forwards**, a function, type or module that hands its
   arguments on unchanged, since it adds a name and a hop and nothing else: call
   what it wraps. A forwarder the language's conventions ask for is not one: a
   `new` that returns `Self::default()`, an `IntoIterator` that calls `iter`, a
   `From`, `AsRef`, `Deref` or `Display` that hands on to a newtype's inner
   value, a public path for a private module's item; the line is whether a
   caller needs the name.
3. **Add what the task needs, and nothing beside it: no function, variant, field
   or convenience nobody calls yet**, since each is a promise to keep, and one
   written before its caller guesses its shape.
4. **Two blocks that look alike stay two until they change for the same
   reason**, since the wrong abstraction costs more than the copy, and a shared
   helper grows a flag for each caller that differs.
5. **What the language and the codebase already have is used, not written
   again**, since a reader knows `position` and `saturating_sub` on sight and
   must read a hand-rolled copy line by line.
6. **What would cost far more to add later is judged on its own, not by these
   rules**: a file format's version, data that cannot be recovered, a published
   type's room to grow, since there the future caller is certain and a retrofit
   is not possible.

### Dead Code

1. **Dead code is deleted, never commented out, kept behind a flag, renamed
   `_old` or left beside its replacement**, since version control keeps it and a
   copy nobody runs goes stale while it looks alive; a change deletes what it
   makes obsolete, and a parameter nothing reads is removed, not renamed
   `_index`.

## Steps

Read each reference a step names, whole, before writing the code.

1. **Changing existing code**: read the file, its neighbours, its callers and
   its tests first, and note the words it uses for the domain, how it names
   functions and types, how it refuses, and how it lays out modules and tests;
   write in them.
2. **Naming or renaming**: `references/naming.md`. Say what the thing is or does
   in a sentence; the name is that sentence's noun or verb. A rename of a public
   item a published crate has released breaks its callers: keep the old name as
   a deprecated alias, or leave it, and say which in the change's description;
   otherwise, move every caller and keep no alias.
3. **A new function, or one grown long**: `references/structure.md`: its steps
   at one level, refusals first, nothing it does left out of its name.
4. **A choice: a flag, a mode, an option, a boolean**: `references/structure.md`
   for an enum or two functions, and `references/simplicity.md` for whether
   anyone makes the choice at all.
5. **Input from outside**: `references/structure.md`: turn it into a type where
   it enters, and check nothing again past there.
6. **An abstraction: a trait, an interface, a generic, a wrapper, a helper, a
   module or a layer**: `references/simplicity.md`. Count the implementations
   and callers the change has; with one, write the concrete thing.
7. **Deleting or replacing code**: `references/simplicity.md`: remove the
   replaced code and every use of it, and leave no stub.
8. **Before finishing**: read the diff as its next reader will, against the
   tells below, then run the checks until they pass.

The tells of code written mechanically, strongest first:

1. A name from the list in the first rule of Names, a fragment, `at` or `held`,
   a variant that names part of its condition, `Past`, or a name that needs
   "and".
2. A `bool` parameter that picks behaviour, or flags and optional fields that
   together make a state.
3. A trait, interface or generic with one implementation; a factory, registry,
   strategy or manager around one thing.
4. A wrapper that only forwards.
5. A check, fallback or catch-all for a state the types rule out; an error
   swallowed to carry on.
6. Nesting three deep where early returns would be flat; `else` after `return`.
7. A comment that restates the next line; a step banner inside a function.
8. Commented-out code; a name ending `_old`, `_new` or `_v2`; a parameter kept
   only so callers need not change.
9. An option no caller sets; a function no caller calls.
10. A second way of doing what the codebase does one way already.

## Checks

- `just check`: every check, as CI runs them, after `just fix`.
- `just check-rust-clippy`, in Rust: `collapsible_if` and `question_mark` refuse
  nesting a flat path would avoid; `dead_code` and `unused_variables` what
  nothing reaches or reads; `disallowed_names` only `foo`, `baz` and `quux`; and
  `enum_variant_names`, only in an enum the crate does not export, variants that
  repeat its name.
- With rust-lints' table, which denies clippy's `pedantic` group, the same
  recipe also runs `manual_let_else` and `redundant_else`, against nesting;
  `fn_params_excessive_bools` and `struct_excessive_bools`, which refuse four
  bools or more, never one; `too_many_lines`, a function past 100 lines; and
  `unnecessary_wraps`, `unused_self` and `struct_field_names`, which hold only
  an item the crate does not export.
- `just check-shell`: ShellCheck's `SC2034` refuses a variable nothing reads.

No check sees a vague name, a trait with one implementation, a single boolean
parameter that picks behaviour, an option nobody sets, a check the types rule
out or commented-out code: review holds those.

## What Not to Do

| Thought                                                  | Instead                                                                       |
| -------------------------------------------------------- | ----------------------------------------------------------------------------- |
| "A `bool` for whether to overwrite is simplest"          | An enum naming both choices, or two functions.                                |
| "A trait, so the store can be swapped later"             | The concrete type; the trait comes with the second implementation.            |
| "They asked for it to be flexible, so a trait"           | Concrete, small code is what changes most easily; say where the seam will go. |
| "Put it in `utils`, it is shared"                        | On the type it works on, or in a module named for its concept.                |
| "`TileManager` to hold the logic"                        | Methods on the type it manages, each named for what it does.                  |
| "Check it again here, to be safe"                        | The type already holds it; a check that cannot fail misleads.                 |
| "Validate, then pass the raw value on"                   | Turn it into a type where it enters, and pass the type.                       |
| "An option for this, someone may want it"                | Leave it out until a caller sets it.                                          |
| "Comment out the old version, in case"                   | Delete it; version control keeps it.                                          |
| "Rename the parameter `_tile` to quiet the lint"         | Remove the parameter.                                                         |
| "A comment above each step of this function"             | A function for each step, named for it.                                       |
| "`place_and_record`"                                     | Two functions, or a name for the one thing they do together.                  |
| "I prefer another style to the codebase's"               | The codebase's, and the preference in the change's description if it holds.   |
| "Tidy the neighbours while I am here"                    | A change of their own; name them in this one's description.                   |
| "`remove`, `clear` and `place_many` too, for a full API" | Only what the task asks for.                                                  |

## References

Read every reference a task touches before writing code, and read them again
after compaction: this body is the summary, and the examples are there.

- `references/naming.md`: before naming or renaming anything, a module, a type,
  a function, a variable, a field, a constant or a file.
- `references/structure.md`: before writing or splitting a function, before a
  conditional more than one level deep, before a boolean, a flag or a type with
  states, and before checking input.
- `references/simplicity.md`: before adding a trait, an interface, a generic, an
  option, a parameter, a wrapper, a helper, a module or a layer, before adding
  what the task did not ask for, and before deleting code or leaving any behind.
- `references/sources.md`: before citing a source for a rule, or adapting one.
