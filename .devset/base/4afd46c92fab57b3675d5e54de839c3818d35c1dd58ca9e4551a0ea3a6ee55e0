#!/usr/bin/env python3
"""The surface of a crate, item by item, with what each one's docs carry.

Usage: doc-inventory.py <crate-dir>

For every fn, struct, enum, trait, type, const, static, macro and module under src/: its
visibility, whether it is documented, and which sections (Safety, Errors, Panics, Examples) its
block has. `unsafe fn` without Safety and `-> Result` without Errors are flagged with `!`; a pub type without
Examples gets a `?` hint (a type whose use sits in a neighbour's example needs none). Use it for the
inventory step, not as a gate.
"""

import re
import sys
from pathlib import Path

ITEM = re.compile(
    r"^(\s*)(?P<vis>pub(\([a-z]+\))?\s+)?(?P<qual>(const\s+|async\s+|unsafe\s+|extern\s+\"C\"\s+)*)"
    r"(?P<kind>fn|struct|enum|trait|type|const|static|mod|macro_rules!|union)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)(?P<rest>.*)$"
)


def main(argv):
    if len(argv) < 2 or argv[1] in ("-h", "--help"):
        print(__doc__.strip())
        return 0
    crate = Path(argv[1]).resolve()
    src = crate / "src"
    rows = []
    for f in sorted(src.rglob("*.rs")):
        lines = f.read_text().splitlines()
        in_test = False
        impl_indent = None
        for i, line in enumerate(lines):
            if re.match(r"^\s*#\[cfg\(test\)\]", line):
                in_test = True
            im = re.match(r"^(\s*)(unsafe\s+)?impl\b.*\bfor\b.*\{\s*$", line)
            if im:
                impl_indent = len(im.group(1))
            elif impl_indent is not None and re.match(rf"^\s{{{impl_indent}}}\}}", line):
                impl_indent = None
            m = ITEM.match(line)
            if m and m.group("kind") == "mod" and line.rstrip().endswith(";"):
                continue
            if not m or (line.rstrip().endswith(";") and m.group("kind") == "mod" and not in_test):
                if m and m.group("kind") == "mod":
                    pass
                else:
                    continue
            if line.strip().startswith(("//", "use ", "let ", "return")):
                continue
            if m.group("name") in ("tests", "model") and m.group("kind") == "mod":
                continue
            # gather the doc block above, skipping attribute lines (single or multi-line)
            j = i - 1
            while j >= 0 and lines[j].strip() and not lines[j].lstrip().startswith(("///", "//!", "//")) and not ITEM.match(lines[j]) and not lines[j].rstrip().endswith(("{", ";")):
                j -= 1
            block = []
            while j >= 0 and re.match(r"^\s*///", lines[j]):
                block.insert(0, re.sub(r"^\s*///\s?", "", lines[j]))
                j -= 1
            text = "\n".join(block)
            sections = [s for s in ("Safety", "Errors", "Panics", "Examples") if f"# {s}" in text]
            kind = m.group("kind")
            qual = m.group("qual") or ""
            rest = m.group("rest") or ""
            vis = (m.group("vis") or "").strip() or "-"
            in_impl = impl_indent is not None and len(m.group(1)) > impl_indent
            if in_impl:
                vis = "impl"
            flags = []
            if not block and not in_impl and not (in_test and kind == "fn"):
                flags.append("undocumented")
            if "unsafe" in qual and kind == "fn" and "Safety" not in sections and not in_impl:
                flags.append("!Safety")
            if kind == "fn" and "Result<" in rest and "Errors" not in sections and not in_impl:
                flags.append("!Errors")
            if vis == "pub" and kind in ("struct", "enum", "trait") and "Examples" not in sections and not in_test and not m.group("name").endswith("Error"):
                flags.append("?Examples")
            summary = block[0][:70] if block else ""
            rows.append(
                (
                    str(f.relative_to(crate)),
                    i + 1,
                    vis,
                    (qual + kind).strip(),
                    m.group("name"),
                    ",".join(sections) or "-",
                    " ".join(flags),
                    summary,
                )
            )
    w = max((len(r[4]) for r in rows), default=10)
    for path, ln, vis, kind, name, secs, flags, summary in rows:
        print(f"{path}:{ln}: {vis:10} {kind:14} {name:{w}} [{secs}] {flags:22} {summary}")
    print(f"{len(rows)} items, {sum(1 for r in rows if 'undocumented' in r[6])} undocumented, {sum(1 for r in rows if '!' in r[6])} flagged")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
