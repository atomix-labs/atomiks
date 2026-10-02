---
name: fixing-ci
description: Use when a CI job fails on a pull request or the default branch, when the `check` workflow is red, when a job passes locally but fails in CI, or when asked why CI failed. Covers finding the failing job, reproducing it with its recipe, and fixing the cause.
---

# Fixing CI

The `check` workflow reads the justfile and runs every `check-*` recipe as a job
named for it, with the tools the lock pins. So a failing job is a recipe, and
running it, `just <job>`, is the same check on a checkout: a fix that passes
there passes in CI.

## Rules

- **Fix the cause, not the check.** Never edit the workflow, skip a job, add
  `continue-on-error`, widen a formatter's excludes, or silence a lint to make a
  job pass. A check that is wrong for the repository is changed on purpose, as
  its own change, not to get a pull request through.
- **A formatter's failure is fixed by the formatter**: `just fix` runs every
  `fix-*` recipe, and a hand edit that only imitates one tends to miss a line.
- **The workflow files are managed.** devset owns `.github/workflows/check.yml`;
  change it through its profile, never by hand.
- **Nothing reaches the remote without the maintainer**: commit the fix, and
  leave the push to them, unless they asked for it.

## Steps

1. **Find the failing jobs**, and read their logs; the annotations name the file
   and the line:

   ```sh
   gh pr checks
   gh run view <run-id> --log-failed
   ```

2. **Reproduce each**: `just <job>`. `plan` is not a recipe: it reads the
   justfile and `.github/automation.json`, and its log names what it refused.
   `check` only gathers the others.
3. **Fix it**: `just fix` for a formatter or a linter that fixes; otherwise the
   code, the document or the configuration the log names.
4. **Check everything**: `just check`, since a fix for one job can fail another.
5. **Commit** in the repository's commit form, and report each job, its cause
   and its fix.

## When CI Disagrees with a Checkout

- **A tool's version.** `mise install` brings a checkout to the versions the
  lock pins, which CI installs.
- **The history.** Jobs check out every commit, so a check that reads commits,
  such as the commit-message check, sees the whole branch.
- **The runner.** Jobs run on `ubuntu-latest`, or on the runner the repository
  variable `CI_RUNNER` names.
- **The network.** A download or a registry that timed out is not the change's
  fault: `gh run rerun <run-id> --failed` runs the failed jobs again, once. A
  failure that repeats is real.

## Checks

- `just <job>` passes for every job that failed, and `just check` passes.
- After the maintainer pushes, `gh pr checks --watch` shows every job green.

## What Not to Do

- Do not rerun a job that failed on the change itself, hoping it passes.
- Do not edit the workflow, or add an exclude or a suppression, to pass a job.
- Do not push, or force-push, without the maintainer.
