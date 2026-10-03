---
name: using-devset
description: Use when changing a file in a repository with a `.devset/` directory, when `devset status` or `just check-devset` reports drift, when a devset update conflicts, or when adding, removing or reconfiguring a profile, a feature or a variable. Covers who owns a file, how to change what a profile owns, and what never to edit.
---

# Using `devset`

devset applies profiles to this repository as layers, and records what it wrote
in `.devset/`. A profile owns a file whole, or the keys its payload names, or a
block between two marker comments; the repository owns everything else. Find out
which before changing a file, and change each side the way devset expects.

## Rules

- **Ask devset first.** `devset explain <file>` names each layer that lists the
  file, the part it owns (whole, keys or block), its policy and its gates.
- **The policy decides what a local edit is.**
  - `owned`: drift, which `devset status` reports and `just check` fails on.
  - `merge`: kept, and merged with the profile's changes on `devset update`.
  - `once`: the repository's, from the first write on.
- **What a profile does not own is the repository's.** A key its payload does
  not name, and every line outside its block, is yours to edit freely. To place
  a block devset has yet to write, write its two markers with nothing between
  them: devset fills them where they are.
- **Take over an owned file by its policy, not by copying it.** Set it in
  `.devset/config.toml`, then edit the file; `devset status` keeps the edit, and
  an update merges the profile's changes into it:

  ```toml
  [files."deny.toml"]
  policy = "merge"
  ```

- **`.devset/config.toml` is the only file under `.devset/` to edit by hand.**
  `answers.toml` changes through `--var`; `lock.toml`, `state.toml`, `base/` and
  `conflicts/` are devset's.

## Steps

### Changing a Managed File

1. `devset explain <file>`: which layer, which part, which policy.
2. Outside what the profile owns: edit it.
3. Inside it, under `merge` or `once`: edit it.
4. Inside it, under `owned`: override the policy to `merge` as above, then edit
   it; or, if every repository should have the change, change the profile in its
   source instead.
5. `devset status`: the file is in sync, or its edit is kept.

### Features and Variables

`devset explain` shows each layer's features, which are on, and what turned each
on; `devset explain <profile>` shows one layer's, with those it leaves off.

- Turn a feature on: `devset add <source>/<profile> --features <feature>`. On a
  layer the target applies, it adds to the layer's features; on a profile active
  only because another requires it, it makes the profile a layer of its own.
  Features add up across every layer that turns them on.
- Turn one off: `devset remove <profile> --features <feature>`, which keeps the
  layer. A feature a requirer or the defaults turn on stays on, and devset says
  so; `--no-default-features` on `devset add` turns the defaults off.
- A variable: `devset apply --var <name>=<value>`. Every file that uses it is
  written again, and an edited `merge` file is merged.
- A `once` file never follows a variable or a feature changed after its first
  write: `.github/release-notes.md` keeps the tagline of its day, and the issue
  chooser and `SUPPORT.md` miss Discussions turned on later. After either,
  `devset diff <file>` on each `once` file `devset status -v` lists shows what
  the profile would write now; bring over what the change meant.

### Profiles

- `devset list` shows each source's profiles, with their features.
- `devset add <source>/<profile>` applies one; `devset remove <profile>` takes
  it out: its unchanged files go, and edited ones stay, untracked.
- `devset apply --rescaffold <profile>/<group>` writes a scaffold's missing
  files again; otherwise a scaffold writes its files once, where the repository
  lacks them.

### Updates and Conflicts

`devset update` moves every source to what its ref names now, and merges what
the profiles changed with the repository's edits. A source pinned to a tag stays
there: `devset update` names the releases newer than it, and `devset update
<source> --tag <tag>` moves it.

A file both sides changed in one place conflicts, on any command that writes:
`add`, `remove`, `apply` or `update`. It is written to
`.devset/conflicts/<path>` with markers.

1. Resolve the markers in `.devset/conflicts/<path>`, keeping both intents.
2. `devset apply --continue` checks and installs it.
3. Or take the whole run back: `devset apply --abort`.

The weekly bump moves each source's tag itself (`just bump-devset`), and takes
back an update that conflicts.

## Checks

- `devset status --exit-code` passes, which `just check-devset` runs: nothing
  drifted, no run left unfinished by a conflict.
- `devset diff` shows, line by line, what `devset apply` would change.
- `just check` passes.

## What Not to Do

- Do not edit `lock.toml`, `state.toml` or `base/`, or delete `.devset/`.
- Do not restore a drifted owned file by hand: `devset apply --force` does it.
- Do not add a key a profile owns under another name, or copy a managed file
  aside, to get around its policy: override the policy.
- Do not resolve a conflict in the file itself: resolve it in
  `.devset/conflicts/`, then `devset apply --continue`.
- Do not run `devset update` to apply a local change: `devset apply` applies,
  and `update` moves the sources.
