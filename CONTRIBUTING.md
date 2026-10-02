# Contributing

How to report a problem, and how to change the project.

<!-- >>> devset: project >>> -->

## Checks

Run `just check` before you open a pull request: CI runs the same checks, and
names each that fails. `just fix` fixes what a formatter or linter can, and
`just --list` shows every recipe.

<!-- <<< devset: project <<< -->

<!-- >>> devset: git-commits >>> -->

## Commits

A pull request lands squashed, as one commit its title names, so its **title**
follows [Conventional Commits](https://www.conventionalcommits.org), which CI
checks on every edit:

```text
type(scope): subject
```

- The **type** is `feat`, `fix`, `refactor`, `docs`, `perf`, `test`, `build`,
  `ci`, `chore`, `style` or `revert`.
- The **subject** is imperative, lower case, with no closing period: it is the
  line the changelog shows.
- A breaking change adds `!` after the scope, and its description says what to
  do.

The commits on your branch are yours to shape; `just check-git-commits` checks
them against the same rules, for a branch that reads well in review.

<!-- <<< devset: git-commits <<< -->

<!-- >>> devset: github-labels >>> -->

## Labels

An issue's kind is its type, Bug, Feature or Task, and a pull request's is its
title's. Labels say the rest, and most are set for you:

| Label              | Means                                       | Set by               |
| ------------------ | ------------------------------------------- | -------------------- |
| `triage`           | new, and yet to be looked at                | the issue forms      |
| `waiting`          | on someone else: the reporter, or upstream  | a maintainer         |
| `good first issue` | a small change, well described, to start on | a maintainer         |
| `help wanted`      | accepted, and open to anyone                | a maintainer         |
| `breaking`         | changes what users rely on                  | a title's `!`        |
| `dependencies`     | moves a pinned tool, action or crate        | the bump, Dependabot |
| `automation`       | opened by a workflow                        | the workflow         |
| `area: <name>`     | where a change lands                        | the files it changes |

A maintainer takes `triage` off once an issue is understood. See
[what awaits triage][triage], and the [good first issues][first].
`.github/labels.toml` holds the labels, and each area's paths.

[triage]: https://github.com/atomix-labs/atomiks/issues?q=is%3Aopen+label%3Atriage
[first]: https://github.com/atomix-labs/atomiks/contribute

<!-- <<< devset: github-labels <<< -->

<!-- >>> devset: setup >>> -->

## Getting Started

`./setup.sh` readies a machine to work on the repository: it installs mise,
pinned and checked against its release's sha256, then every tool the repository
pins, at the version its lock records, and runs `just setup`. It needs git, curl
and bash, installs into your home directory without sudo, and `--dry-run` says
what it would do. Fork the repository, then:

```sh
git clone https://github.com/<you>/atomiks.git
cd atomiks
./setup.sh
```

Or clone and set up in one line:

```sh
curl -fsSL https://atomix-labs.github.io/atxp/setup.sh | bash -s -- github.com/atomix-labs/atomiks
```

The first run takes a few minutes; run it again after pulling, and it installs
only what moved. `./setup.sh --activate` adds mise to your shell, so the tools
are on `PATH` in every new one; without it, `mise exec -- just check` runs them.

<!-- <<< devset: setup <<< -->
