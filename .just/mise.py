"""The mise lockfiles: every tool locked for every platform, and every download verified.

Usage: mise.py check | fill <lockfile>... | bump [<report>]

`check` fails when a configured tool has no lock entry at its pinned version, or a tool that
downloads a file lacks a platform, or a platform has neither a checksum nor provenance. `fill` gives
each platform that has no checksum the publisher's digest, or failing that the download's own.
`bump` moves the repository's own tools, those configured outside .config/mise/conf.d/, to their
newest releases past the cooldown, locks and fills them, and writes a Markdown report.
"""

import hashlib
import json
import os
import re
import subprocess
import sys
import urllib.request
from pathlib import Path

import tomllib

# Every lock covers these; `mise install --locked` refuses a platform missing from it.
PLATFORMS = ("linux-arm64", "linux-x64", "macos-arm64")

# Where the profiles pin their tools; the repository's own tools are configured anywhere else.
CONF_D = Path(".config/mise/conf.d")

# Backends that build or fetch from a registry, so a lock entry holds a version and no download.
UNDOWNLOADED = ("cargo:", "pipx:", "npm:", "go:", "gem:", "asdf:", "vfox:", "core:rust", "ubi:")

# The bump rewrites the lock, which the `locked` setting keeps every install from doing.
UNLOCKED = os.environ | {"MISE_LOCKED": "0"}

HEADER = re.compile(r'^\[tools\.("[^"]+"|[^."]+)\."platforms\.([a-z0-9-]+)"\]\s*$')
FIELD = re.compile(r'^(url|url_api|checksum) = "([^"]*)"\s*$')


def configured():
    """Each configured tool: its pin, the version the pin resolves to, and the file that pins it."""
    out = subprocess.run(
        ["mise", "ls", "--current", "--json", "--offline"],
        capture_output=True,
        text=True,
        check=True,
    )
    tools = {}
    for name, entries in json.loads(out.stdout).items():
        for entry in entries:
            source = entry.get("source") or {}
            if source.get("path"):
                path = Path(source["path"]).resolve().relative_to(Path.cwd().resolve())
                pin = entry.get("requested_version") or entry["version"]
                tools[name] = (pin, entry["version"], path)
    return tools


def track(pin):
    """Whether `pin` names a track, `3.14`, whose newest release the lock holds."""
    return re.fullmatch(r"\d+(\.\d+)?", pin) is not None


def pinned(version, pin):
    """Whether a locked `version` is what `pin` asks for: the version, or one on its track."""
    return version == pin or (track(pin) and version.startswith(f"{pin}."))


def lockfile(source):
    """The lockfile a config file's tools are locked in, as mise places it."""
    if source.parent == CONF_D or source == CONF_D.parent / "config.toml":
        return CONF_D.parent / "mise.lock"
    return source.with_suffix(".lock")


def entries(lock):
    """The lock entries in `lock`, by tool."""
    if not lock.is_file():
        return {}
    with lock.open("rb") as file:
        return tomllib.load(file).get("tools", {})


def entry_problems(name, pin, found, where):
    """Why the lock entries `found` do not lock `name` as `pin` asks, for every platform.

    mise keeps one entry per set of options a platform resolves to, so a tool whose options differ
    by platform has several entries at one version: their platforms are read together.
    """
    matching = [e for e in found if pinned(e.get("version", ""), pin)]
    if not matching:
        return [f"{where}: `{name}` is not locked at {pin}: run `mise lock`"]
    if matching[0].get("backend", name).startswith(UNDOWNLOADED):
        return []
    platforms = {
        k.removeprefix("platforms."): v
        for entry in matching
        for k, v in entry.items()
        if k.startswith("platforms.")
    }
    out = []
    for platform in PLATFORMS:
        locked = platforms.get(platform)
        if locked is None:
            out.append(f"{where}: `{name}` is not locked for {platform}: run `just bump-mise`")
        elif not locked.get("checksum") and not locked.get("provenance"):
            out.append(
                f"{where}: `{name}` on {platform} has no checksum or provenance: run `mise.py fill`"
            )
    return out


def check():
    """Prints every problem with the locks, and fails if there is one."""
    found = []
    for name, (pin, _, source) in sorted(configured().items()):
        lock = lockfile(source)
        found += entry_problems(name, pin, entries(lock).get(name, []), lock)
    for problem in found:
        print(problem, file=sys.stderr)
    return 1 if found else 0


def digest(url, api):
    """The sha256 the publisher gives for a download, or else the download's own; and which."""
    headers = {"Accept": "application/vnd.github+json", "User-Agent": "devset-mise"}
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if api and token:
        headers["Authorization"] = f"Bearer {token}"
    if api:
        with urllib.request.urlopen(urllib.request.Request(api, headers=headers)) as response:
            published = json.load(response).get("digest")
        if published and published.startswith("sha256:"):
            return published, "the publisher's digest"
    sha = hashlib.sha256()
    with urllib.request.urlopen(
        urllib.request.Request(url, headers={"User-Agent": "devset-mise"})
    ) as response:
        for chunk in iter(lambda: response.read(1 << 16), b""):
            sha.update(chunk)
    return f"sha256:{sha.hexdigest()}", "hashed from the download"


def fill(lock):
    """Gives every platform of `lock` that downloads a file without a checksum one; each, noted."""
    lines = lock.read_text().splitlines(keepends=True)
    out, notes, at = [], [], 0
    while at < len(lines):
        line = lines[at]
        out.append(line)
        at += 1
        header = HEADER.match(line)
        if not header:
            continue
        table = []
        while at < len(lines) and not lines[at].startswith("["):
            table.append(lines[at])
            at += 1
        fields = dict(m.groups() for m in map(FIELD.match, table) if m)
        if "url" in fields and "checksum" not in fields:
            checksum, how = digest(fields["url"], fields.get("url_api"))
            out.append(f'checksum = "{checksum}"\n')
            notes.append(f"{header.group(1).strip(chr(34))} on {header.group(2)}: {how}")
        out.extend(table)
    lock.write_text("".join(out))
    return notes


def version_key(version):
    """`version` for ordering: its numbers, then the rest."""
    return [int(part) for part in re.findall(r"\d+", version)], version


def latest(name, age=None):
    """The newest release of `name`, past the cooldown unless `age` overrides it."""
    args = ["mise", "latest", name] + (["--minimum-release-age", age] if age else [])
    return subprocess.run(args, capture_output=True, text=True, check=True).stdout.strip()


def bump(report):
    """Moves the repository's own tools past the cooldown, locks and fills them; reports."""
    moved, held, failed, notes, locks = [], [], [], [], set()
    own = {n: v for n, v in configured().items() if v[2].parent != CONF_D}
    for name, (pin, have, source) in sorted(own.items()):
        # A track moves within itself and keeps its pin; an exact pin is rewritten.
        wanted = f"{name}@{pin}" if track(pin) else name
        try:
            want, newest = latest(wanted), latest(wanted, "0s")
        except subprocess.CalledProcessError as error:
            failed.append(
                f"`{name}`: {error.stderr.strip().splitlines()[-1] if error.stderr else error}"
            )
            continue
        if newest != want and version_key(newest) > version_key(want):
            held.append(f"`{name}` {newest}")
        if version_key(want) > version_key(have):
            if track(pin):
                subprocess.run(["mise", "upgrade", name], check=True, env=UNLOCKED)
            else:
                subprocess.run(
                    ["mise", "use", "--path", str(source), f"{name}@{want}"], check=True, env=UNLOCKED
                )
            moved.append(f"`{name}` {have} -> {want}")
        locks.add(lockfile(source))
    if own:
        subprocess.run(
            ["mise", "lock", "--platform", ",".join(PLATFORMS), *own], check=True, env=UNLOCKED
        )
    for lock in sorted(locks):
        if lock.is_file():
            notes += fill(lock)
    sections = [
        ("Moved", moved),
        ("Held by the cooldown", held),
        ("Checksums added", notes),
        ("Could not check", failed),
    ]
    text = (
        "### Tools\n\n"
        + "".join(
            f"{title} ({len(items)}):\n\n" + "".join(f"- {item}\n" for item in items) + "\n"
            for title, items in sections
            if items
        )
        if any(items for _, items in sections)
        else "### Tools\n\nNothing to move.\n"
    )
    if report:
        Path(report).write_text(text)
    else:
        print(text, end="")
    return 1 if failed else 0


def main(args):
    match args:
        case ["check"]:
            return check()
        case ["fill", *locks] if locks:
            for note in (n for lock in locks for n in fill(Path(lock))):
                print(note)
            return 0
        case ["bump", *report] if len(report) <= 1:
            return bump(report[0] if report else None)
    print(__doc__.strip().splitlines()[2], file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
