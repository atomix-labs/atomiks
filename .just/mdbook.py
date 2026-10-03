"""The book's helpers: its lint, and the API's landing page.

Usage: mdbook.py lint <book-dir>
       mdbook.py api <api-dir>

`lint` finds what mdBook renders without a word, and should not, prints each finding, and fails if
there is one:

- A page under src/ that SUMMARY.md never links is never published.
- An include of a file that is not there only logs; one of an anchor the file lacks renders an
  empty block.
- A recipe a page names that the justfile does not have describes something that fails.

`api` rewrites rustdoc's list of every crate, in `<api-dir>/index.html`, as the workspace lays its
crates out. Each crate rustdoc documented, as its `crates.js` says, is listed with the first
sentence of its docs, under the directory its package is in; a group that other groups depend on
comes first. Only the list and its heading change: the page's head, scripts and theme stay
rustdoc's.
"""

import html
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

LINK = re.compile(r"\]\(\s*(?:\./)?([^)#\s]+\.md)(?:#[^)]*)?\s*\)")
INCLUDE = re.compile(r"\{\{#(?:rustdoc_)?include\s+([^}\s]+)\s*\}\}")
RECIPE = re.compile(r"`just ([a-z][a-z0-9-]*)")
# rustdoc's index page: its title, its heading, and the list of crates to replace.
TITLE = "<title>Index of crates</title>"
HEADING = "<h1>List of all crates</h1>"
LIST = re.compile(r'<ul class="all-items">.*?</ul>', re.DOTALL)
NAMED = "API Reference"


def listed(src):
    """Every page SUMMARY.md links, resolved."""
    summary = src / "SUMMARY.md"
    return {(src / link).resolve() for link in LINK.findall(summary.read_text())}


def recipes():
    """Every recipe the justfile has."""
    out = subprocess.run(["just", "--summary"], capture_output=True, text=True, check=True).stdout
    return set(out.split())


def includes(page, text):
    """Each broken include on `page`: a file that is not there, or an anchor the file lacks."""
    for spec in INCLUDE.findall(text):
        path, _, anchor = spec.partition(":")
        target = page.parent / path
        if not target.is_file():
            yield f"{page}: includes {path}, which is not there"
        elif anchor and not anchor[0].isdigit():
            body = target.read_text()
            for marker in ("ANCHOR", "ANCHOR_END"):
                if not re.search(rf"{marker}:\s*{re.escape(anchor)}(?![\w-])", body):
                    yield f"{page}: {path} has no `{marker}: {anchor}`"


def lint(book):
    """Prints each finding of the book at `book`; 1 if there is one."""
    src = book / "src"
    published, known, found = listed(src), recipes(), []
    for page in sorted(src.rglob("*.md")):
        if page.name != "SUMMARY.md" and page.resolve() not in published:
            found.append(f"{page}: no entry in SUMMARY.md")
        text = page.read_text()
        found += includes(page, text)
        found += [
            f"{page}: names `just {recipe}`, which is no recipe"
            for recipe in RECIPE.findall(text)
            if recipe not in known
        ]
    for finding in found:
        print(finding, file=sys.stderr)
    return 1 if found else 0


def summary(source):
    """The first sentence of the `//!` docs opening `source`, as HTML."""
    lines = []
    for line in source.read_text(errors="replace").splitlines():
        text = line.strip()
        if text.startswith("//!"):
            lines.append(text[3:].strip())
        elif lines or (text and not text.startswith(("#!", "//"))):
            break
    text = re.sub(r"\s+", " ", " ".join(lines)).strip()
    # An intra-doc link resolves only in rustdoc: its text stays, its target goes.
    text = re.sub(r"\[(`[^`]+`|[^\]]+)\](?:\([^)]*\)|\[[^\]]*\])?", r"\1", text)
    sentence = re.match(r"(.+?[.!?])(?:\s|$)", text)
    escaped = html.escape(sentence.group(1) if sentence else text)
    return re.sub(r"`([^`]+)`", r"<code>\1</code>", escaped)


def groups(doc):
    """The crates rustdoc documented in `doc`, as (name, summary) by the directory holding their
    packages, in reading order: a group more groups depend on first, then by name.

    A binary named as a library is one crate to rustdoc, which documents the library."""
    present = set(re.findall(r'"([^"]+)"', (doc / "crates.js").read_text()))
    command = ["cargo", "metadata", "--no-deps", "--format-version", "1"]
    meta = json.loads(subprocess.run(command, capture_output=True, text=True, check=True).stdout)
    root = Path(meta["workspace_root"])
    home, chosen = {}, {}
    for package in meta["packages"]:
        where = Path(package["manifest_path"]).parent.relative_to(root)
        home[package["name"]] = root.name if where == Path() else where.parent.as_posix()
        for target in package["targets"]:
            crate = target["name"].replace("-", "_")
            library = bool({"lib", "rlib", "proc-macro"} & set(target["kind"]))
            if crate in present and (crate not in chosen or library):
                chosen[crate] = (home[package["name"]], summary(Path(target["src_path"])))
    found = defaultdict(list)
    for crate, (group, text) in chosen.items():
        found[group].append((crate, text))
    dependents = defaultdict(set)
    for package in meta["packages"]:
        for dependency in package["dependencies"]:
            there, here = home.get(dependency["name"]), home[package["name"]]
            if there and there != here and dependency.get("kind") != "dev":
                dependents[there].add(here)
    order = sorted(found, key=lambda group: (-len(dependents[group]), group))
    return [(group, sorted(found[group])) for group in order]


def api(doc):
    """Rewrites the crate list of `doc`'s index page in groups; 1 if it is not rustdoc's page."""
    index = doc / "index.html"
    page = index.read_text()
    if HEADING not in page or not LIST.search(page):
        print(f"{index}: not rustdoc's index page, with its list of crates", file=sys.stderr)
        return 1
    grouped = groups(doc)
    out = []
    for group, crates in grouped:
        if len(grouped) > 1:
            anchor = re.sub(r"[^a-z0-9]+", "-", group.lower()).strip("-")
            out.append(
                f'<h2 id="{anchor}" class="section-header">{html.escape(group)}'
                f'<a href="#{anchor}" class="anchor">§</a></h2>'
            )
        items = "".join(
            f'<dt><a class="mod" href="{crate}/index.html">{crate}</a></dt><dd>{text}</dd>'
            for crate, text in crates
        )
        out.append(f'<dl class="item-table">{items}</dl>')
    page = page.replace(TITLE, f"<title>{NAMED}</title>").replace(HEADING, f"<h1>{NAMED}</h1>")
    index.write_text(LIST.sub(lambda _: "".join(out), page, count=1))
    return 0


def main(args):
    commands = {"lint": lint, "api": api}
    if len(args) != 2 or args[0] not in commands:
        usage = __doc__.split("\n\n")[1]
        print(usage, file=sys.stderr)
        return 2
    return commands[args[0]](Path(args[1]))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
