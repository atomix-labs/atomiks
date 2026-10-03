"""Hold every crate manifest to the shape the workspace reads them in.

taplo settles layout: alignment, and alphabetical order inside each dependency group. What it cannot
reach is the text around the values, because a JSON schema validates a parsed tree and a comment is
not in one. Those rules are checked here, on the lines rather than the values.

    cargo-manifest.py [<path>...]         # default: the workspace's manifests, vendored crates aside
    cargo-manifest.py --fix [<path>...]   # each dependency under its group, the workspace's too

`--fix` rewrites each dependency table as `# external` and its entries, then `# internal` and
its, keeping their order and a blank line between the groups: a tool that removes an entry, as
`cargo shear --fix` does, takes the comment above it too. A table already in its groups, or
holding any other line, is left as it is. A dependency written dotted, `serde.workspace = true`,
as `cargo add` writes one, is read as its inline table.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

# A header, an entry, and the two group markers: the whole grammar a manifest is read with here.
TABLE = re.compile(r"^\[+([^\]]+)\]+\s*$")
ENTRY = re.compile(r'^\s*((?:[\w.-]+|"[^"]+")(?:\.[\w-]+)*)\s*=')
GROUPS = ("# external", "# internal")

# `[package]`, in the order every crate writes it. `name` and `description` are the crate's own; the
# rest say `<key>.workspace = true` and say nothing else.
PACKAGE_ORDER = [
    "name",
    "description",
    "version",
    "edition",
    "rust-version",
    "license",
    "authors",
    "publish",
]
INHERITED = {"version", "edition", "rust-version", "license", "authors", "publish"}

DEP_KINDS = {"dependencies", "dev-dependencies", "build-dependencies"}


def dependency(line: str):
    """The dependency a line of a dependency table names, and the key of it the line sets, or None
    for a line that sets none: `serde = { … }` is `("serde", None)`, and the dotted form Cargo reads
    as the same entry, `serde.workspace = true`, is `("serde", "workspace")`."""
    entry = ENTRY.match(line)
    if not entry:
        return None
    key = entry.group(1)
    if key.startswith('"'):
        name, _, rest = key[1:].partition('"')
        rest = rest.removeprefix(".")
    else:
        name, _, rest = key.partition(".")
    return name, rest or None


def tracked(repo: Path) -> list[Path]:
    """The workspace's manifests, its root's and each member's, as cargo reads them: a `Cargo.toml`
    that is no part of it, a test's fixture or an example's, is left alone."""
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=repo, capture_output=True, text=True, check=True
    )
    metadata = json.loads(out.stdout)
    paths = {Path(metadata["workspace_root"]) / "Cargo.toml"}
    paths |= {Path(package["manifest_path"]) for package in metadata["packages"]}
    # A vendored crate is upstream's, under a `vendor/` directory anywhere in the tree.
    return sorted(path for path in paths if "vendor" not in path.relative_to(repo).parts[:-1])


def internal_names(paths: list[Path]) -> set:
    """Every package the workspace builds: what `# internal` is allowed to name."""
    names = set()
    for path in paths:
        for line in path.read_text().splitlines():
            match = re.match(r'^name\s*=\s*"([^"]+)"', line)
            if match:
                names.add(match.group(1))
                break
    return names


def workspace_deps(root: Path) -> set:
    """The `[workspace.dependencies]` keys: the internal deps a crate must inherit rather than path to."""
    names, inside = set(), False
    for line in (root / "Cargo.toml").read_text().splitlines():
        table = TABLE.match(line)
        if table:
            inside = table.group(1) == "workspace.dependencies"
        elif inside:
            dep = dependency(line)
            if dep:
                names.add(dep[0])
    return names


def tables(path: Path) -> list:
    """(header, [(lineno, text)]) per table, the preamble under a header of `""`."""
    out, current = [("", [])], None
    for lineno, line in enumerate(path.read_text().splitlines(), 1):
        table = TABLE.match(line)
        if table:
            current = (table.group(1), [])
            out.append(current)
        else:
            (current or out[0])[1].append((lineno, line))
    return out


def kind(header: str) -> str:
    """The dependency kind a header names, `[target.'cfg(…)'.dev-dependencies]` included."""
    tail = header.rsplit(".", 1)[-1]
    return tail if tail in DEP_KINDS and not header.startswith("workspace.lints") else ""


def bare(line: str) -> str:
    """The line with every quoted string blanked, so a `#` inside a value is not read as a comment."""
    return re.sub(r'"[^"]*"|\'[^\']*\'', '""', line)


def check_comments(path: Path, header: str, body: list, found: list) -> None:
    for lineno, line in body:
        text = line.strip()
        if text.startswith("#"):
            if not (kind(header) or header == "workspace.dependencies") or text not in GROUPS:
                found.append((path, lineno, f"comment: `{text[:60]}`"))
        elif ENTRY.match(line) and "#" in bare(line):
            found.append((path, lineno, "comment: trailing, on an entry"))


def check_groups(path: Path, header: str, body: list, internal: set, found: list) -> None:
    """Both markers, `# external` first, each once, and every entry under the one that names it."""
    seen, group = [], None
    entries = {"# internal": [], "# external": []}
    for lineno, line in body:
        text = line.strip()
        if text in GROUPS:
            if text in seen:
                found.append((path, lineno, f"group: `{text}` written twice"))
            seen.append(text)
            group = text
        elif ENTRY.match(line):
            if group is None:
                found.append((path, lineno, "group: an entry before any `# internal` / `# external`"))
            else:
                entries[group].append((lineno, dependency(line)[0]))
    if seen == ["# internal", "# external"]:
        found.append((path, body[0][0], "group: `# internal` before `# external`"))
    for lineno, name in entries["# internal"]:
        if name not in internal:
            found.append((path, lineno, f"group: `{name}` is external, under `# internal`"))
    for lineno, name in entries["# external"]:
        if name in internal:
            found.append((path, lineno, f"group: `{name}` is internal, under `# external`"))
    for marker in GROUPS:
        if entries[marker] and marker not in seen:
            found.append((path, body[0][0], f"group: entries with no `{marker}` header"))


def owned(crate: Path, target: str) -> bool:
    """Whether a path dep stays inside the crate's own tree, a `macros/` sibling included."""
    roots = [crate, crate.parent] if crate.parent.name == "macros" else [crate]
    resolved = (crate / target).resolve()
    return any(resolved.is_relative_to(root.resolve()) for root in roots)


def check_inherit(path: Path, body: list, shared: set, internal: set, found: list) -> None:
    """An internal dep the workspace already pins is inherited; a path dep stays in the crate's tree.
    A dep written in dotted form, over one line or several, is read as its inline table."""
    specs: dict = {}
    for lineno, line in body:
        dep = dependency(line)
        if not dep or dep[0] not in internal:
            continue
        name, key = dep
        value = line.split("=", 1)[1].strip()
        specs.setdefault(name, (lineno, []))[1].append(value if key is None else f"{key} = {value}")
    for name, (lineno, parts) in specs.items():
        value = ", ".join(parts)
        target = re.search(r'path\s*=\s*"([^"]+)"', value)
        if target and name in shared:
            found.append((path, lineno, f"dep: `{name}` is in [workspace.dependencies]; inherit it"))
        elif target and not owned(path.parent, target.group(1)):
            found.append((path, lineno, f"dep: `{name}` paths outside the crate's own tree"))
        elif not target and "workspace" not in bare(value):
            found.append((path, lineno, f"dep: `{name}` pins a version; use `{{ workspace = true }}`"))


def check_package(path: Path, body: list, found: list) -> None:
    keys = [ENTRY.match(line).group(1) for _, line in body if ENTRY.match(line)]
    stems = [key.split(".")[0] for key in keys]
    for lineno, line in body:
        entry = ENTRY.match(line)
        if entry and entry.group(1) in INHERITED:
            found.append((path, lineno, f"package: write `{entry.group(1)}.workspace = true`"))
    if "description" not in stems:
        found.append((path, body[0][0] if body else 1, "package: no `description`"))
    wanted = [key for key in PACKAGE_ORDER if key in stems]
    if [key for key in stems if key in PACKAGE_ORDER] != wanted:
        found.append((path, body[0][0], f"package: key order; want {' '.join(wanted)}"))


def check(path: Path, repo: Path, shared: set, internal: set) -> list:
    found: list = []
    text = path.read_text()
    nested = re.search(r"^\[workspace\]", text, re.M) is not None  # its own workspace: it inherits nothing to check
    seen = set()
    for header, body in tables(path):
        seen.add(header)
        check_comments(path, header, body, found)
        if kind(header) or header == "workspace.dependencies":
            check_groups(path, header, body, internal, found)
            if not nested:
                check_inherit(path, body, shared, internal, found)
        elif header == "package" and not nested:
            check_package(path, body, found)
        elif header == "features":
            keys = [ENTRY.match(line).group(1) for _, line in body if ENTRY.match(line)]
            if keys and keys[0] != "default":
                found.append((path, body[0][0], "features: `default` is missing or is not first"))
    if not nested and not any(header.split(".")[0] == "lints" for header in seen):
        found.append((path, 1, "lints: no `[lints] workspace = true`"))
    return sorted(found, key=lambda item: item[1])


def regroup(path: Path, internal: set) -> bool:
    """Writes each dependency table of `path` in its two groups; whether anything changed."""
    lines = path.read_text().splitlines(keepends=True)
    out, body, header = [], [], None

    def flush():
        if header is not None and (kind(header) or header == "workspace.dependencies"):
            out.extend(grouped(body, internal))
        else:
            out.extend(body)

    for line in lines:
        table = TABLE.match(line)
        if table:
            flush()
            out.append(line)
            body, header = [], table.group(1)
        else:
            body.append(line)
    flush()
    text = "".join(out)
    if text == path.read_text():
        return False
    path.write_text(text)
    return True


def grouped(body: list, internal: set) -> list:
    """A dependency table's lines, `# external` and its entries first, then `# internal` and its,
    a blank line between them where the table had one; the blank lines around it stay. A table
    already in its groups, or holding a line of any other kind, is left as it was."""
    start, end = 0, len(body)
    while start < end and not body[start].strip():
        start += 1
    while end > start and not body[end - 1].strip():
        end -= 1
    entries = [line for line in body[start:end] if line.strip() and line.strip() not in GROUPS]
    if not all(ENTRY.match(line) for line in entries):
        return body
    groups = {marker: [] for marker in GROUPS}
    for line in entries:
        groups["# internal" if dependency(line)[0] in internal else "# external"].append(line)
    gap = ["\n"] if any(not line.strip() for line in body[start:end]) else []
    out = []
    for marker in GROUPS:
        if groups[marker]:
            out += [*(gap if out else []), f"{marker}\n", *groups[marker]]
    if [line for line in out if line.strip()] == [line for line in body if line.strip()]:
        return body
    return body[:start] + out + body[end:]


def main() -> int:
    repo = Path(subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True).stdout.strip())
    every = tracked(repo)
    args = sys.argv[1:]
    if args[:1] == ["--fix"]:
        internal = internal_names(every)
        changed = [path for path in ([Path(arg).resolve() for arg in args[1:]] or every) if regroup(path, internal)]
        for path in changed:
            print(f"{path.relative_to(repo)}: regrouped")
        return 0
    paths = [Path(arg).resolve() for arg in args] or [path for path in every if path != repo / "Cargo.toml"]
    for path in paths:
        if not path.is_file():
            print(f"error: no manifest at {path}", file=sys.stderr)
            return 2
    shared, internal = workspace_deps(repo), internal_names(every)

    found = [item for path in paths for item in check(path, repo, shared, internal)]
    for path, lineno, message in found:
        print(f"{path.relative_to(repo)}:{lineno}  {message}")
    print(f"  manifests: {len(found)} finding(s) over {len(paths)} manifest(s)")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
