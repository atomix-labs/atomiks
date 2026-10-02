"""Moves every Cargo requirement and git revision past the cooldown, one at a time.

Usage: cargo-bump.py [<report>]

For every workspace root, but one a root's `[workspace] exclude` leaves out: each requirement `cargo upgrade` would move, to a release at least three
days old, is moved on its own and kept only if the workspace still resolves; each git dependency
pinned by `rev` moves to its repository's HEAD once that commit is three days old; then every
lockfile is settled. Crates named in the root's `[workspace.metadata.bump] exclude` stay put.
Writes a Markdown report; exits non-zero when something could not be checked.
"""

import datetime
import json
import os
import re
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
from pathlib import Path

import tomllib

COOLDOWN = datetime.timedelta(days=3)
FLAGS = ["--compatible=allow", "--incompatible=allow"]
GIT_DEP = re.compile(r'^(\s*"?([\w-]+)"?\s*=\s*\{.*git\s*=\s*"([^"]+)".*rev\s*=\s*")([0-9a-f]{7,40})(".*)$')


def run(*args, check=True):
    """`args`, run; its completed process."""
    return subprocess.run(args, capture_output=True, text=True, check=check)


def roots():
    """Every manifest that is a workspace root, but one under a path another root's `[workspace]
    exclude` names, as a crate's examples or test fixtures are: those are data, not workspaces."""
    out = run("git", "grep", "-lE", r"^\[workspace\]", "--", "**/Cargo.toml", "Cargo.toml", check=False)
    found = [Path(line) for line in out.stdout.split()]
    excluded = []
    for root in found:
        with root.open("rb") as file:
            paths = tomllib.load(file).get("workspace", {}).get("exclude", [])
        excluded += [Path(os.path.normpath(root.parent / path)) for path in paths]
    return [root for root in found if not any(root.is_relative_to(path) for path in excluded)]


def excluded(root):
    """The crates `root` keeps out of every bump."""
    with root.open("rb") as file:
        manifest = tomllib.load(file)
    return set(manifest.get("workspace", {}).get("metadata", {}).get("bump", {}).get("exclude", []))


def candidates(text):
    """Every requirement a `cargo upgrade --dry-run` table would move: (name, from, to)."""
    moves, ruled = [], False
    for line in text.splitlines():
        cells = line.split()
        if cells and all(cell == "=" * len(cell) for cell in cells):
            ruled = True
            continue
        if not ruled:
            continue
        if len(cells) > 1 and cells[1].startswith("("):  # a renamed dependency: `name (rename)`
            del cells[1]
        if len(cells) not in (
            5,
            6,
        ):  # anything else ends the table; a sixth cell is a hold-back note
            ruled = False
        elif len(cells) == 5 and cells[1] != cells[4]:
            moves.append((cells[0], cells[1], cells[4]))
    return moves


def fetch(url):
    """A JSON document from `url`, authenticated to GitHub when a token is at hand."""
    headers = {"User-Agent": "atxp-cargo-bump", "Accept": "application/json"}
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token and url.startswith("https://api.github.com/"):
        headers["Authorization"] = f"Bearer {token}"
    with urllib.request.urlopen(urllib.request.Request(url, headers=headers)) as response:
        return json.load(response)


def aged(when):
    """Whether an ISO 8601 time is past the cooldown."""
    return datetime.datetime.now(datetime.UTC) - datetime.datetime.fromisoformat(when) >= COOLDOWN


def published(name, requirement):
    """When the release a requirement names was published."""
    version = requirement.lstrip("=^~")
    return fetch(f"https://crates.io/api/v1/crates/{name}/{version}")["version"]["created_at"]


def bump_one(root, name):
    """Moves `name` in `root`, keeping it only if the workspace still resolves; why not, if not."""
    tracked = run("git", "ls-files", "-z", "--", "*Cargo.toml", "*Cargo.lock").stdout.split("\0")
    with tempfile.TemporaryFile() as snapshot:
        with tarfile.open(fileobj=snapshot, mode="w") as tar:
            for path in filter(None, tracked):
                tar.add(path)
        upgrade = run("cargo", "upgrade", *FLAGS, "--manifest-path", str(root), "--package", name, check=False)
        resolved = (
            upgrade.returncode == 0
            and run(
                "cargo",
                "metadata",
                "--format-version",
                "1",
                "--manifest-path",
                str(root),
                check=False,
            ).returncode
            == 0
        )
        if resolved:
            return None
        snapshot.seek(0)
        with tarfile.open(fileobj=snapshot) as tar:
            tar.extractall(filter="data")
    lines = (upgrade.stderr or "").strip().splitlines()
    reason = (line for line in lines if "does not have" in line or "no matching package" in line)
    return next(reason, lines[-1] if lines else "did not resolve")


def bump_git(root, held, moved, failed):
    """Moves each git dependency `root` pins by `rev` to its repository's HEAD, once aged."""
    text = root.read_text()
    out = []
    for line in text.splitlines(keepends=True):
        match = GIT_DEP.match(line.rstrip("\n"))
        repo = re.match(r"https://github\.com/([^/]+/[^/.]+)", match.group(3)) if match else None
        if not match:
            out.append(line)
            continue
        name, have = match.group(2), match.group(4)
        if not repo:
            failed.append(f"`{name}`: {match.group(3)} is not on GitHub, so its commit's age is unknown")
            out.append(line)
            continue
        try:
            head = fetch(f"https://api.github.com/repos/{repo.group(1)}/commits/HEAD")
        except OSError as error:
            failed.append(f"`{name}`: {error}")
            out.append(line)
            continue
        sha, when = head["sha"], head["commit"]["committer"]["date"]
        if sha.startswith(have) or have.startswith(sha):
            out.append(line)
        elif not aged(when):
            held.append(f"`{name}` {sha[:12]}")
            out.append(line)
        else:
            out.append(f"{match.group(1)}{sha}{match.group(5)}\n")
            moved.append(f"`{name}` {have[:12]} -> {sha[:12]}")
    root.write_text("".join(out))


def main(args):
    applied, skipped, held, unchecked, failed = [], [], [], [], []
    for root in roots():
        dry = run("cargo", "upgrade", *FLAGS, "--dry-run", "--manifest-path", str(root), check=False)
        if dry.returncode != 0:
            unchecked.append(f"`{root}`: {(dry.stderr.strip().splitlines() or ['no output'])[0]}")
            continue
        keep = excluded(root)
        for name, have, want in candidates(dry.stdout):
            if name in keep:
                continue
            try:
                if not aged(published(name, want)):
                    held.append(f"`{name}` {want}")
                    continue
            except OSError as error:
                failed.append(f"`{name}`: {error}")
                continue
            why = bump_one(root, name)
            (skipped if why else applied).append(f"`{name}` {have} -> {want}" + (f": {why}" if why else ""))
        bump_git(root, held, applied, failed)
    for root in roots():
        run("cargo", "update", "--manifest-path", str(root), check=False)
    sections = [
        ("Applied", applied),
        ("Skipped, needing a fix by hand", skipped),
        ("Held by the cooldown", held),
        ("Roots cargo upgrade could not read", unchecked),
        ("Could not check", failed),
    ]
    body = "".join(f"{title} ({len(items)}):\n\n" + "".join(f"- {item}\n" for item in items) + "\n" for title, items in sections if items)
    text = "### Cargo\n\n" + (body or "Nothing to move.\n")
    if args:
        Path(args[0]).write_text(text)
    else:
        print(text, end="")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
