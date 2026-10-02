"""The repository's labels, as .github/labels.toml says.

Usage: labels.py check | sync | label <pull request>

`check` holds the file to GitHub's limits, and every label a form, the automation or Dependabot
names to it. `sync` makes the repository's labels the file's: each written as it says, and each
retired one removed once its issues carry the type it names; a label the file does not name is
reported, and kept. `label` gives a pull request its areas, from the files it changes, and
`breaking`, from a `!` after its title's type, and takes off those that no longer hold. `sync` and
`label` run in the labels workflow, with `gh` and $GITHUB_REPOSITORY.
"""

import json
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from urllib.parse import quote

FILE = Path(".github/labels.toml")
BREAKING = "breaking"
# A Conventional Commit's header that marks a breaking change: `type!:` or `type(scope)!:`.
TITLE = re.compile(r"^[a-z]+(\([^)]*\))?!:")
COLOR = re.compile(r"^[0-9a-f]{6}$")
KEYS = {"color", "description", "paths"}


def load():
    """The labels to keep, and the retired ones with the type their issues take."""
    data = tomllib.loads(FILE.read_text())
    return data.get("labels", {}), data.get("retired", {})


def glob(pattern):
    """A path pattern as a regex: `**` crosses directories, `*` and `?` stay within one."""
    out, i = "", 0
    while i < len(pattern):
        if pattern.startswith("**/", i):
            out, i = out + "(?:.*/)?", i + 3
        elif pattern.startswith("**", i):
            out, i = out + ".*", i + 2
        elif pattern[i] == "*":
            out, i = out + "[^/]*", i + 1
        elif pattern[i] == "?":
            out, i = out + "[^/]", i + 1
        else:
            out, i = out + re.escape(pattern[i]), i + 1
    return re.compile(out + r"\Z")


def unquote(text):
    return text.split(" #")[0].strip().strip("'\"")


def yaml_labels(text):
    """The labels a YAML file names under its `labels:` keys, as a flow or a block list."""
    names, lines = [], text.splitlines()
    for i, line in enumerate(lines):
        key = re.match(r"^(\s*)labels:(.*)$", line)
        if not key:
            continue
        rest = unquote(key.group(2))
        if rest.startswith("["):
            names += [unquote(name) for name in rest.strip("[]").split(",") if name.strip()]
            continue
        for item in lines[i + 1 :]:
            entry = re.match(r"^(\s*)- (.*)$", item)
            if not entry or len(entry.group(1)) < len(key.group(1)):
                break
            names.append(unquote(entry.group(2)))
    return names


def named():
    """Each file that names labels, and the labels it names."""
    forms = Path(".github/ISSUE_TEMPLATE")
    for path in sorted([*forms.glob("*.yml"), *forms.glob("*.yaml")]):
        if path.stem != "config":
            yield path, yaml_labels(path.read_text())
    dependabot = Path(".github/dependabot.yml")
    if dependabot.is_file():
        yield dependabot, yaml_labels(dependabot.read_text())
    automation = Path(".github/automation.json")
    if automation.is_file():
        settings = json.loads(automation.read_text())
        kinds = [kind for kind in settings.values() if isinstance(kind, dict)]
        yield automation, [name for kind in kinds for name in kind.get("labels", [])]


def check():
    if not FILE.is_file():
        return
    labels, retired = load()
    errors = []
    for name, label in labels.items():
        where = f"{FILE}: label `{name}`"
        if len(name) > 50:
            errors.append(f"{where}: a name is at most 50 characters")
        if not COLOR.match(str(label.get("color", ""))):
            errors.append(f"{where}: `color` is six hex digits, in lower case")
        if len(label.get("description", "")) > 100:
            errors.append(f"{where}: a description is at most 100 characters")
        if unknown := sorted(set(label) - KEYS):
            errors.append(f"{where}: unknown keys {', '.join(unknown)}")
        paths = label.get("paths", [])
        if not isinstance(paths, list) or not all(isinstance(path, str) for path in paths):
            errors.append(f"{where}: `paths` is a list of path patterns")
        if name in retired:
            errors.append(f"{where}: kept and retired both")
    for name, kind in retired.items():
        if not isinstance(kind, str):
            errors.append(f'{FILE}: retired `{name}` takes the type its issues get, or ""')
    for path, names in named():
        for name in names:
            if name in retired:
                errors.append(f"{path}: names `{name}`, which {FILE} retires")
            elif name not in labels:
                errors.append(f"{path}: names `{name}`, which {FILE} does not hold")
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        sys.exit(1)


def gh(*args):
    return subprocess.run(["gh", *args], check=True, text=True, stdout=subprocess.PIPE).stdout


def sync():
    repo = os.environ["GITHUB_REPOSITORY"]
    labels, retired = load()
    fields = "name,color,description"
    listed = gh("label", "list", "--repo", repo, "--limit", "1000", "--json", fields)
    have = {label["name"].lower(): label for label in json.loads(listed)}
    for name, label in labels.items():
        now = have.get(name.lower())
        want = {"name": name, "color": label["color"], "description": label.get("description", "")}
        if now and {**now, "color": now["color"].lower()} == want:
            continue
        options = ["--color", want["color"], "--description", want["description"], "--force"]
        gh("label", "create", name, "--repo", repo, *options)
        print(f"{'updated' if now else 'created'} {name}")
    for name, kind in retired.items():
        if not (now := have.get(name.lower())):
            continue
        if kind:
            query = f"repos/{repo}/issues?labels={quote(now['name'])}&state=all&per_page=100"
            untyped = ".[] | select(.pull_request == null and .type == null) | .number"
            for number in gh("api", "--paginate", query, "--jq", untyped).split():
                gh("api", "-X", "PATCH", f"repos/{repo}/issues/{number}", "-f", f"type={kind}")
                print(f"#{number} is a {kind}")
        gh("label", "delete", now["name"], "--repo", repo, "--yes")
        print(f"retired {now['name']}")
    known = {name.lower() for name in [*labels, *retired]}
    for key, label in have.items():
        if key not in known:
            print(f"::notice::`{label['name']}` is on GitHub, and not in {FILE}: kept")


def label(number):
    repo = os.environ["GITHUB_REPOSITORY"]
    labels, _ = load()
    pull = json.loads(gh("api", f"repos/{repo}/pulls/{number}"))
    query = f"repos/{repo}/pulls/{number}/files?per_page=100"
    files = gh("api", "--paginate", query, "--jq", ".[].filename").splitlines()
    areas = {name: label["paths"] for name, label in labels.items() if label.get("paths")}
    patterns = [(name, glob(path)) for name, paths in areas.items() for path in paths]
    want = {name for name, pattern in patterns if any(map(pattern.match, files))}
    managed = set(areas)
    if BREAKING in labels:
        managed.add(BREAKING)
        if TITLE.match(pull["title"]):
            want.add(BREAKING)
    have = {label["name"] for label in pull["labels"]}
    if add := sorted(want - have):
        fields = [arg for name in add for arg in ("-f", f"labels[]={name}")]
        gh("api", "-X", "POST", f"repos/{repo}/issues/{number}/labels", *fields)
    for name in sorted((have & managed) - want):
        gh("api", "-X", "DELETE", f"repos/{repo}/issues/{number}/labels/{quote(name)}")
    print(f"#{number}: {', '.join(sorted(want)) or 'no labels of its own'}")


if __name__ == "__main__":
    match sys.argv[1:]:
        case ["check"]:
            check()
        case ["sync"]:
            sync()
        case ["label", number]:
            label(number)
        case _:
            sys.exit(__doc__)
