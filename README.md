<!-- >>> devset: project >>> -->
<!-- dprint-ignore-start -->

<h1 align="center">atomiks</h1>

<p align="center">An atomics and locks library.</p>

<p align="center">
  <a href="https://github.com/atomix-labs/atomiks/actions/workflows/check.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/atomix-labs/atomiks/check.yml?branch=main&amp;style=flat-square&amp;label=check"></a>
  <a href="https://atomix-labs.github.io/atomiks/"><img alt="Book" src="https://img.shields.io/badge/book-read-blue?style=flat-square"></a>
</p>

<!-- dprint-ignore-end -->
<!-- <<< devset: project <<< -->

atomiks is a workspace of Rust crates for sharing state between threads. Its
first crate, `atomiks`, gives any value that fits one atomic word a typed
atomic: integers, `NonZero`s, `char`s, floats, pointers, and `Option`s that
spend a spare bit pattern on `None`, with orderings checked at compile time and
every operation the instruction its name promises. It is in early development,
and no crate is released yet.

## Documentation

- [The book][book]: what atomiks is, and how to use it.
- [CHANGELOG.md][changelog]: what changed in each release.

## Contributing

Issues and pull requests are welcome: read [CONTRIBUTING.md][contributing]
first.

## License

Either [the MIT License][mit] or [the Apache License, Version 2.0][apache], at
your option.

[book]: https://atomix-labs.github.io/atomiks/
[changelog]: CHANGELOG.md
[contributing]: CONTRIBUTING.md
[mit]: LICENSE-MIT
[apache]: LICENSE-APACHE
