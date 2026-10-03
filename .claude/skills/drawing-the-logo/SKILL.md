---
name: drawing-the-logo
description: Use when a project needs a logo, a favicon or a social preview image, or when one is to be redrawn, renamed or checked against the house style; when the `logo`, `book_logo` or `book_social` variable is about to be set; or when the README's header, the book's menu bar or a shared link shows a plain name where a logo belongs. Covers the mark, the wordmark, the light and dark variants, the favicon, the tile and the social card, how each is drawn and generated, and how the owner confirms one before it is applied.
---

# Drawing the Logo

A project's logo is a mark, a small drawing of what the project is, beside its
name. Every project here draws it the same way, so its pages read as one family:
devset's and atxp's are the models, in their `docs/src/media/`. The mark is the
one part drawn by hand; `scripts/draw.py` makes every other file from it, the
wordmark, the dark variants, the favicon, the tile and the social card, so they
never drift from the mark or from each other. The rules below are the house
style, each with its reason.

## Rules

### The Mark

1. **The mark is 32 by 32 units, one filled shape and then strokes, each stroke
   2.5 wide with round caps and round joins**, since one weight at one grid
   reads the same at 16 pixels in a tab as at 112 on the social card, and the
   round ends match the house's other marks.
2. **It draws what the project is, in the plainest shape that says it**:
   devset's is a stack of layers, a filled sheet over two stroked ones, atxp's
   four sheets over the same two. A letter, a gradient, a shadow or a third
   colour is no mark, since it is lost at 16 pixels.
3. **Its ink is `#1f2328` on light and `#f0f6fc` on dark**, GitHub's own text
   colours, so the mark sits on either theme as the text beside it does.
4. **Each SVG holds a `<title>` naming the project, and nothing a renderer must
   fetch**, since a screen reader reads the title, and an SVG shown as an image
   loads no font or file.

### The Files

Every file is generated from `mark-light.svg` by `scripts/draw.py`; none is
edited by hand.

| File                             | What it is                                                                                                                |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `docs/src/media/mark-light.svg`  | the mark, in the light ink, the one file drawn by hand                                                                    |
| `docs/src/media/mark-dark.svg`   | the mark in the dark ink                                                                                                  |
| `docs/src/media/logo-light.svg`  | the mark, then the name in JetBrains Mono Bold at size 31, 42 units right of the mark's origin, on the baseline y = 26.75 |
| `docs/src/media/logo-dark.svg`   | the same, in the dark ink                                                                                                 |
| `docs/src/media/favicon.svg`     | the mark, whose `<style>` takes the dark ink under `prefers-color-scheme: dark`                                           |
| `docs/src/media/tile.svg`        | the mark inset 4 on a 40 by 40 tile, `#f6f8fa` with a `#d1d9e0` border, corners of radius 9                               |
| `docs/src/media/social.png`      | 1280 by 640, white: the logo 112 high, centred, and the tagline in two lines of Inter at 36, `#59636e`, below it          |
| `docs/theme/favicon.svg`, `.png` | the book's favicon, and the tile at 32 by 32 for a browser that takes no SVG                                              |

1. **The name is outlines, never `<text>`**, since a reader's machine may lack
   the font, and the logo must look the same on every one. The logo's width is
   where the name's advance ends, 42 plus 18.6 a letter, rounded down; its
   height is 32, the mark's.
2. **Every SVG is optimized by svgo, but the favicon**, since svgo would move
   the light ink into a `style` attribute that the dark scheme's rule cannot
   override.
3. **The social card's tagline is the README's tagline, in two lines**, broken
   where the sense breaks, since a link to the repository shared anywhere shows
   the card alone, and it must say what the README's header says.
4. **The logo is proposed to the owner and confirmed before it is applied**,
   since it is the project's face, and the owner's to choose: propose two or
   three marks, each with what it draws and why, shown light and dark at the
   sizes they are used.

## Steps

1. **Read what the project is**: the README's tagline, the `description`
   variable, and the crate page or the book's introduction. Name the one thing
   the project is about, and find its plainest shape.
2. **Draw two or three marks**, each a `mark-light.svg` in a directory of its
   own outside the repository, on the 32 grid, by the rules above. Render each
   light and dark, at 16, 32 and 128 pixels, on one sheet, and show the owner
   the sheet with a line on each mark: what it draws, and why.
3. **Wait for the owner to choose.** Apply nothing until then; a mark they
   change is drawn again and shown again.
4. **Generate the rest**, once confirmed, with the chosen mark at
   `docs/src/media/mark-light.svg`. The script needs Python with fontTools and
   cairosvg, which needs the Cairo library (`brew install cairo`, `apt install
   libcairo2`), Node for `npx`, and the two fonts, from their releases:

   ```sh
   fonts=$(mktemp -d)
   curl -fsSL -o "$fonts/mono.zip" https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip
   curl -fsSL -o "$fonts/sans.zip" https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip
   unzip -q "$fonts/mono.zip" fonts/ttf/JetBrainsMono-Bold.ttf -d "$fonts"
   unzip -q "$fonts/sans.zip" extras/ttf/Inter-Regular.ttf -d "$fonts"
   python3 -m venv "$fonts/venv"
   "$fonts/venv/bin/pip" install --quiet fonttools cairosvg
   "$fonts/venv/bin/python" .claude/skills/drawing-the-logo/scripts/draw.py \
       docs/src/media/mark-light.svg <name> "<first line> | <second line>" \
       --mono "$fonts/fonts/ttf/JetBrainsMono-Bold.ttf" \
       --sans "$fonts/extras/ttf/Inter-Regular.ttf"
   ```

5. **Apply it**, setting the variables that show it, as `using-devset` says:

   ```sh
   devset apply --var logo=docs/src/media/logo
   devset apply --var book_logo=media/mark --var book_social=media/social.png
   ```

   `logo` is a path in the repository, since only GitHub shows the README.
6. **Hand the owner the social card**: GitHub shows it only once it is uploaded
   in Settings, General, Social preview, which needs the repository's admin.

## Checks

- By hand: open each SVG in a browser in the light scheme and in the dark, and
  `social.png` beside the README's tagline, word for word.
- `git diff --stat docs/` lists the files above and nothing else; running the
  script again changes none of them.
- `just check-devset`: every file the variables render, as they render it.
- `just check-mdbook`: the book builds with its logo and favicon.
- `just check`: every check, as CI runs them.

No check reads a logo: whether it says what the project is, is the owner's.

## What Not to Do

| Thought                                         | Instead                                                 |
| ----------------------------------------------- | ------------------------------------------------------- |
| "The owner will like this one; apply it"        | Propose it, with the others, and wait for their choice. |
| "A `<text>` element is simpler than outlines"   | Outlines: the reader may lack the font.                 |
| "The wordmark is a pixel off; nudge it by hand" | Fix the mark, or the script, and generate again.        |
| "A brand colour would make it stand out"        | The two inks: the mark sits on either theme.            |
| "svgo the favicon too, for a few bytes"         | Its ink then cannot follow the dark scheme.             |
| "The card can say more than the tagline"        | The tagline, word for word, in two lines.               |

## References

- `scripts/draw.py`: run it for every file the mark does not hold; its docstring
  says what each argument takes. It traces the name as opentype.js 1.3.4 does
  and optimizes with svgo 4.1.0, so it writes devset's and atxp's SVGs again
  byte for byte from their marks.
