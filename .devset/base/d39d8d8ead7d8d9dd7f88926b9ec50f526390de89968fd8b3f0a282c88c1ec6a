# Sources

Read this before a rule of this skill needs backing, before citing a source for
one, and before adapting a rule from somewhere else. It says what each source
holds, which rule it backs, where this skill departs from it, and the notice
owed for the text it adapts. Cite one of these, or an equally primary source;
open the page before linking it.

## Canon

| source                                                                                                                                                                                                                       | says                                                                                                                                                                                                                                                                                                                                                                                                               | backs                                                                    |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------ |
| [crates.io, `crates_io_markdown`](https://github.com/rust-lang/crates.io/blob/c2513ad6d248fab5908f66f8ca2fc5a18c365069/crates/crates_io_markdown/src/lib.rs)                                                                 | `text_to_html` takes the README, its path in the package, the `repository` URL and the package's `path_in_vcs`; a relative URL is rewritten only for a `repository` on `github.com`, `gitlab.com` or `bitbucket.org`, else "relative links will be omitted"; a link goes to `blob/HEAD`, an image by its extension (`svg`, `png`, `jpg`, `gif`, `webp` and others) to `raw/HEAD`, joined to the README's directory | the body's absolute links; templates.md §5                               |
| [ammonia 4.2.0, `is_url_attr`](https://docs.rs/crate/ammonia/4.2.0/source/src/lib.rs)                                                                                                                                        | the attributes whose relative URLs a sanitizer's rewrite reaches: `href`, `src`, `xlink:href`, a form's `action`, an object's `data`, `formaction`, `ping` and a video's `poster`; `srcset` is none of them, and crates.io's sanitizer, on 4.2.0, passes `srcset` on `<source>` as written                                                                                                                         | the dark `<source srcset>`, which crates.io leaves relative              |
| [crates.io, the publish controller](https://github.com/rust-lang/crates.io/blob/c2513ad6d248fab5908f66f8ca2fc5a18c365069/src/controllers/krate/publish.rs)                                                                   | each version's README is rendered at its publish, from the `readme` text cargo uploads, its `readme_file`, the `repository`, and `.cargo_vcs_info.json`'s `path_in_vcs`                                                                                                                                                                                                                                            | templates.md §5: what each version's page shows                          |
| [The Cargo Book, the `readme` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-readme-field)                                                                                                               | the file "will be transferred to the registry when you publish"; with no value, a `README.md`, `README.txt` or `README` in the package root is used                                                                                                                                                                                                                                                                | the body's Links and Images, templates.md §4 and §5                      |
| [Cargo, `prepare_for_publish`](https://github.com/rust-lang/cargo/blob/f3865b2a4d1acc5276f6b3c67d0e057f4dab3928/src/workspace/parser/mod.rs)                                                                                 | a `readme` outside the package root is copied into it, and the packaged manifest's `readme` becomes the file's name, so the README is recorded in the crate's own directory                                                                                                                                                                                                                                        | templates.md §5: a root README's relative paths resolve in the crate     |
| [The Cargo Book, `cargo install`](https://doc.rust-lang.org/cargo/commands/cargo-install.html)                                                                                                                               | "By default, the Cargo.lock file that is included with the package will be ignored"; "The `--locked` flag can be used to force Cargo to use the packaged Cargo.lock file"; `--git` installs from a repository                                                                                                                                                                                                      | the body's Rust rules 1 and 3                                            |
| [The Cargo Book, `cargo add`](https://doc.rust-lang.org/cargo/commands/cargo-add.html)                                                                                                                                       | with no version, `cargo add <dep>` takes the "Latest release in the registry"; `--git <url>` adds one from a repository; `--features` turns features on                                                                                                                                                                                                                                                            | the body's Rust rules 1, 3 and 5                                         |
| [GitHub, Relative links and image paths](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes#relative-links-and-image-paths-in-markdown-files) | GitHub resolves a relative link against the file's place on the current branch, and recommends relative links, since absolute ones "may not work in clones"                                                                                                                                                                                                                                                        | the body's Links and Images rule 3: relative where only GitHub shows it  |
| [GitHub, The Picture element](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#the-picture-element)                        | "The `<picture>` HTML element is supported"                                                                                                                                                                                                                                                                                                                                                                        | the body's Links and Images rule 2                                       |
| [Linebender, `#![doc = include_str!("README.md")]`](https://linebender.org/blog/doc-include/)                                                                                                                                | a README included as the crate page needs its intra-doc links repeated as definitions, and a style to hide what only GitHub should show                                                                                                                                                                                                                                                                            | the body's Rust rule 6: two pages, and why the house writes `//!` itself |
| [Standard Readme, the specification](https://github.com/RichardLitt/standard-readme/blob/main/spec.md)                                                                                                                       | a README's sections in a fixed order, title and short description first, Install and Usage next, Contributing and License last; the short description matches the package manager's `description`; code examples are linted as the project's code is                                                                                                                                                               | the body's order, and the pitch agreeing with `description`              |

Where the skill says what crates.io does, it was read from crates.io's source at
the commit linked above, 2026-09-30, and what Cargo does from Cargo's at the
commit linked, 2026-09-29, with Cargo 1.98.1's `cargo package --list` run on a
crate with and without `readme`.

## Read, Not Adapted

- Art of README, by hackergrrl, licensed CC BY 2.0, as agent-toolkit copies it
  (its repository, `hackergrrl/art-of-readme`, no longer resolves): a README
  that narrows from what the module is to how it is used, and brevity as a
  feature. The body's order agrees with it; no text of it is copied.
- [Make a README](https://www.makeareadme.com), by Danny Guo, MIT: the common
  sections, each with what it is for. No text of it is copied.

## Where the Skill Departs

| a source says                                                                                | here                                                                                                            |
| -------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| crafting-effective-readmes: ask who the audience is and what kind of project it is, first    | the manifests say what the repository builds, and the template follows; a question is only for a fact they lack |
| crafting-effective-readmes, the open-source template: badges written by hand under the title | the project profile's header block brings them, and nothing repeats it                                          |
| crafting-effective-readmes: a "Last reviewed" date                                           | none: git holds when the file changed                                                                           |
| Standard Readme: a banner links a local image in the repository                              | an absolute URL where the README is shown away from the repository                                              |
| Standard Readme: a table of contents past 100 lines; License states its owner                | a table of contents where the page needs one; the licence files name the owner                                  |
| Linebender: a crate page included from the README                                            | not done: the crate page is its own `//!`                                                                       |

## Adapted Text

This skill adapts the README tasks of
[crafting-effective-readmes](https://github.com/joshuadavidthomas/agent-skills/tree/main/crafting-effective-readmes),
by joshuadavidthomas, as agent-toolkit carries it, rewritten for crates and
command lines under devset: a README created, a section added, one updated and
one reviewed against the project's state; the sections every README has, a name,
a description and its use; and the open-source template's order, install, use,
contributing and licence. Its notice:

```text
MIT License

Copyright (c) 2025 Josh Thomas

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

The body's order and its rule that the pitch agrees with the manifest's
`description` adapt
[Standard Readme](https://github.com/RichardLitt/standard-readme), by Richard
Littauer (RichardLitt). Its notice:

```text
The MIT License (MIT)

Copyright (c) 2017-2025 Richard Littauer

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
