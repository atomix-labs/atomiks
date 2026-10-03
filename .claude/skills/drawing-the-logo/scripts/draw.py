#!/usr/bin/env python3
"""Draws a project's logo in the house style, from its mark and its tagline.

Usage: draw.py <mark-light.svg> <name> <tagline> --mono <ttf> --sans <ttf> [--book <dir>]

<mark-light.svg> is the 32 x 32 mark in the light ink, #1f2328: one filled shape, then strokes 2.5
wide with round caps and joins, a <title> naming the project. <name> is the word the logo sets.
<tagline> is the README's tagline, broken into the card's two lines at its `|`, or else at the
space nearest its middle. --mono is JetBrains Mono Bold, `fonts/ttf/JetBrainsMono-Bold.ttf` of
its 2.304 release; --sans is Inter Regular, `extras/ttf/Inter-Regular.ttf` of its 4.1 release.
Writes, beside the mark and in the book's directory, `docs` by default:

    logo-light.svg, logo-dark.svg  the mark, then the name in JetBrains Mono Bold at 31, 42 to the
                                   right of the mark's origin, on the baseline 26.75
    mark-dark.svg                  the mark in the dark ink, #f0f6fc
    favicon.svg                    the mark, inked by the reader's colour scheme
    tile.svg                       the mark inset 4 on a 40 x 40 tile, #f6f8fa within #d1d9e0
    social.png                     1280 x 640, white: the logo, and the tagline in #59636e below it
    <book>/theme/favicon.svg       the favicon, for the book
    <book>/theme/favicon.png       the tile at 32 x 32, for a browser that takes no SVG

Each SVG is optimized by svgo 4.1.0, which npx runs. The outlines are traced as opentype.js 1.3.4
traces them, its numbers to two places, so that svgo writes the house's files again byte for byte.
Needs fontTools and cairosvg.
"""

import argparse
import math
import re
import subprocess
import sys
from fractions import Fraction
from pathlib import Path

import cairosvg
from fontTools.ttLib import TTFont

LIGHT, DARK = "#1f2328", "#f0f6fc"
TILE, BORDER, GREY = "#f6f8fa", "#d1d9e0", "#59636e"
SVGO = ["npx", "--yes", "svgo@4.1.0", "--multipass", "--final-newline", "--quiet"]
NS = 'xmlns="http://www.w3.org/2000/svg"'

# The wordmark: its size, where it starts, and its baseline, in the mark's 32 units.
WORD_SIZE, WORD_X, WORD_BASELINE = 31, 42, 26.75
# The card: the logo 112 high, centred to the pixel, and the tagline's two baselines, at 36 px.
CARD_WIDTH, CARD_HEIGHT, CARD_SCALE, CARD_TOP = 1280, 640, 3.5, 192
TAGLINE_SIZE, TAGLINE_BASELINES = 36, (386, 436)


def fixed(value):
    """`value` to two places as JavaScript's `toFixed(2)` writes it: a tie goes away from zero."""
    hundredths = math.floor(abs(Fraction(value)) * 100 + Fraction(1, 2))
    sign = "-" if value < 0 and hundredths else ""
    return f"{sign}{hundredths // 100}.{hundredths % 100:02d}"


def number(value):
    """A coordinate as opentype.js's `toPathData(2)` writes it: whole, or to two places."""
    return str(round(value)) if round(value) == value else fixed(value)


def outline(font, name):
    """The contours of the glyph `name`, in font units, as opentype.js reads them: each starts at
    its last point, or the first on-curve one after it, and an off-curve run takes the midpoints
    between its points as on-curve."""
    glyf = font["glyf"]
    coordinates, ends, flags = glyf[name].getCoordinates(glyf)
    commands, start = [], 0
    for end in ends:
        points = [(*coordinates[i], bool(flags[i] & 1)) for i in range(start, end + 1)]
        start = end + 1
        current, following = points[-1], points[0]
        if current[2]:
            commands.append(("M", current[:2]))
        elif following[2]:
            commands.append(("M", following[:2]))
        else:
            commands.append(("M", midpoint(current, following)))
        for index in range(len(points)):
            current, following = following, points[(index + 1) % len(points)]
            if current[2]:
                commands.append(("L", current[:2]))
            else:
                after = following[:2] if following[2] else midpoint(current, following)
                commands.append(("Q", current[:2], after))
        commands.append(("Z",))
    return commands


def midpoint(one, other):
    """The point halfway between two points of an outline."""
    return ((one[0] + other[0]) * 0.5, (one[1] + other[1]) * 0.5)


def text_path(font_path, text, size, x, baseline):
    """The path data of `text` set in the font at `font_path`, from `x` on `baseline`, and the
    advance it ends at."""
    font = TTFont(font_path)
    cmap, metrics = font.getBestCmap(), font["hmtx"]
    scale = 1 / font["head"].unitsPerEm * size
    parts = []
    for character in text:
        name = cmap[ord(character)]
        for command, *points in outline(font, name):
            placed = [(x + px * scale, baseline + -py * scale) for px, py in points]
            parts.append(command + " ".join(f"{number(px)} {number(py)}" for px, py in placed))
        x += metrics[name][0] * scale
    return "".join(parts), x


def optimize(*paths):
    """Optimizes each SVG at `paths` in place with svgo."""
    subprocess.run([*SVGO, *map(str, paths)], check=True)


def inner(svg):
    """What an SVG holds between its title and its closing tag: the mark's shapes."""
    return re.sub(r"^.*?</title>|</svg>\s*$", "", svg, flags=re.DOTALL)


def two_lines(tagline):
    """The tagline as the card's two lines: at its `|`, or at the space nearest its middle."""
    if "|" in tagline:
        first, second = tagline.split("|", 1)
        return first.strip(), second.strip()
    spaces = [index for index, character in enumerate(tagline) if character == " "]
    middle = min(spaces, key=lambda index: abs(index - len(tagline) / 2))
    return tagline[:middle], tagline[middle + 1 :]


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("mark", type=Path)
    parser.add_argument("name")
    parser.add_argument("tagline")
    parser.add_argument("--mono", type=Path, required=True)
    parser.add_argument("--sans", type=Path, required=True)
    parser.add_argument("--book", type=Path, default=Path("docs"))
    args = parser.parse_args()
    media, theme = args.mark.parent, args.book / "theme"
    theme.mkdir(parents=True, exist_ok=True)

    optimize(args.mark)
    mark = args.mark.read_text()
    title = re.search(r"<title>.*?</title>", mark).group(0)
    shapes = inner(mark)
    word, end = text_path(args.mono, args.name, WORD_SIZE, WORD_X, WORD_BASELINE)
    width = math.floor(end)
    logo = f'<svg {NS} viewBox="0 0 {width} 32">{title}{shapes}<path fill="{LIGHT}" d="{word}"/></svg>'
    tile = f'<svg {NS} viewBox="0 0 40 40">{title}<rect width="39" height="39" x=".5" y=".5" fill="{TILE}" stroke="{BORDER}" rx="9"/><g transform="translate(4 4)">{shapes}</g></svg>'
    drawn = {
        media / "logo-light.svg": logo,
        media / "logo-dark.svg": logo.replace(LIGHT, DARK),
        media / "mark-dark.svg": mark.replace(LIGHT, DARK),
        media / "tile.svg": tile,
    }
    for path, svg in drawn.items():
        path.write_text(svg)
    optimize(*drawn)
    # A fill closes each subpath by itself, so the wordmark's closing `z` draws nothing.
    for path in (media / "logo-light.svg", media / "logo-dark.svg"):
        path.write_text(re.sub(r'z"/></svg>\n$', '"/></svg>\n', path.read_text()))
    # The favicon skips svgo, which would move the light ink into a `style` attribute that the
    # dark scheme's rule cannot override.
    scheme = f"svg{{color:{LIGHT}}}@media (prefers-color-scheme:dark){{svg{{color:{DARK}}}}}"
    inked = shapes.replace(f'"{LIGHT}"', '"currentColor"')
    favicon = f'<svg {NS} viewBox="0 0 32 32">{title}<style>{scheme}</style>{inked}</svg>\n'
    for path in (media / "favicon.svg", theme / "favicon.svg"):
        path.write_text(favicon)

    left = math.floor((CARD_WIDTH - width * CARD_SCALE) / 2)
    lines = []
    for index, line in enumerate(two_lines(args.tagline)):
        baseline = TAGLINE_BASELINES[index]
        _, advance = text_path(args.sans, line, TAGLINE_SIZE, 0, baseline)
        path, _ = text_path(args.sans, line, TAGLINE_SIZE, (CARD_WIDTH - advance) / 2, baseline)
        lines.append(f'<path fill="{GREY}" d="{path}"/>')
    card = f'<svg {NS} width="{CARD_WIDTH}" height="{CARD_HEIGHT}"><rect width="{CARD_WIDTH}" height="{CARD_HEIGHT}" fill="#fff"/><g transform="translate({left} {CARD_TOP}) scale({CARD_SCALE})">{inner((media / "logo-light.svg").read_text())}</g>{"".join(lines)}</svg>'
    cairosvg.svg2png(bytestring=card.encode(), write_to=str(media / "social.png"))
    cairosvg.svg2png(
        bytestring=(media / "tile.svg").read_bytes(),
        write_to=str(theme / "favicon.png"),
        output_width=32,
        output_height=32,
    )
    for path in [*drawn, media / "favicon.svg", theme / "favicon.svg", media / "social.png"]:
        print(path)
    print(theme / "favicon.png")
    return 0


if __name__ == "__main__":
    sys.exit(main())
