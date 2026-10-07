<!-- >>> devset: project >>> -->
<!-- dprint-ignore-start -->

<h1 align="center">atomiks</h1>

<p align="center">Typed atomics for any value that fits one atomic word, and the locks built on them.</p>

<p align="center">
  <a href="https://github.com/atomix-labs/atomiks/actions/workflows/check.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/atomix-labs/atomiks/check.yml?branch=main&amp;style=flat-square&amp;label=check"></a>
  <a href="https://atomix-labs.github.io/atomiks/"><img alt="Book" src="https://img.shields.io/badge/book-read-blue?style=flat-square"></a>
  <a href="https://github.com/atomix-labs/devset"><img alt="managed with devset" src="https://img.shields.io/badge/managed_with-devset-0969da?style=flat-square&amp;logo=data:image/svg%2bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHRpdGxlPmRldnNldDwvdGl0bGU+PHBhdGggZmlsbD0iI2YwZjZmYyIgZD0ibTE2IDMgMTMgNi41TDE2IDE2IDMgOS41WiIvPjxwYXRoIGZpbGw9Im5vbmUiIHN0cm9rZT0iI2YwZjZmYyIgc3Ryb2tlLWxpbmVjYXA9InJvdW5kIiBzdHJva2UtbGluZWpvaW49InJvdW5kIiBzdHJva2Utd2lkdGg9IjIuNSIgZD0ibTMgMTYgMTMgNi41TDI5IDE2TTMgMjIuNSAxNiAyOWwxMy02LjUiLz48L3N2Zz4K"></a>
</p>

<!-- dprint-ignore-end -->
<!-- <<< devset: project <<< -->

atomiks is a workspace of Rust crates for sharing state between threads. The
`atomiks` crate gives any value that fits one atomic word a typed atomic:
integers, plain or held to a range, `NonZero`s, `char`s, floats, pointers,
`Option`s that spend a spare bit pattern on `None`, and the structs and enums
`#[derive(Atom)]` packs into one word, with orderings checked at compile time
and every operation the instruction its name promises. One instruction on the
whole word changes one field of a packed struct alone:
`QUOTE.fields().live.set(Release)` is a `lock or`, or an `ldset`. A pointer
keeps small fields as tags in the low bits its pointee's alignment leaves clear,
and keeps its provenance: `HEAD.fields().closed.test_and_set(AcqRel)` closes a
Treiber stack's head with a `lock bts`, or an `ldsetal`. It works with serde,
zerocopy, bytemuck, arbitrary, arbitrary-int and deranged, each behind a
feature. `atomiks-core` holds the typed atomic, `atomiks-derive` the derive, and
`atomiks` re-exports both, the derive under its `derive` feature; all build for
Linux and macOS, on aarch64 and x86_64. atomiks is in early development, and no
crate is released yet.

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
[changelog]: https://github.com/atomix-labs/atomiks/blob/main/CHANGELOG.md
[contributing]: https://github.com/atomix-labs/atomiks/blob/main/CONTRIBUTING.md
[mit]: https://github.com/atomix-labs/atomiks/blob/main/LICENSE-MIT
[apache]: https://github.com/atomix-labs/atomiks/blob/main/LICENSE-APACHE
