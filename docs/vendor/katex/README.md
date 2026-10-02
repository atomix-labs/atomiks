# Vendored KaTeX

`mdbook-katex` renders math at build time, but links a stylesheet from a CDN,
which `no-css = true` in `book.toml` turns off. These files replace it, so the
site makes no third-party request. Their version, 0.16.4, is the one
`mdbook-katex` renders with: the stylesheet and the markup must agree.

The fonts are in `src/vendor/katex/fonts/`: mdBook copies `src/` to the site as
it is, which makes the stylesheet's relative `url(fonts/…)` resolve. The mdbook
profile ships these files, and moves them with `mdbook-katex`.
