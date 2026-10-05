# Structure

Read this before writing or splitting a function, before a conditional more than
one level deep, before a boolean, a flag or a type with several states, and
before checking input. It says how a function's body and a value's states are
shaped so that a reader follows the code from the top down and cannot build a
state that means nothing. The rules hold in any language, and the examples are
Rust; where the repository applies python or shell, a section at the end says
what that language adds.

## A Function Does One Thing, at One Level

A function's body reads as the steps its name promises. Each step sits at the
same distance from the machine: "find the square, refuse if it is taken, put the
tile there" is one level, and the arithmetic that turns a column and a row into
an index is another. A body that mixes them makes the reader hold both at once
and tell, line by line, which is which. The step that drops a level becomes a
function whose name is the step, and the body that called it reads as the
sentence its name began.

```rust
use thiserror::Error;

pub const EMPTY: u8 = 0;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    #[error("place error: the square is off the grid")]
    OffGrid,
    #[error("place error: the square holds a tile already")]
    Occupied,
}

#[derive(Debug)]
pub struct Grid {
    columns: u16,
    squares: Vec<u8>,
}

impl Grid {
    // Bad: the rule of the grid and the arithmetic of its storage, in one body.
    pub fn place(&mut self, column: u16, row: u16, tile: u8) -> Result<(), PlaceError> {
        if column >= self.columns {
            return Err(PlaceError::OffGrid);
        }
        let index = usize::from(row)
            .checked_mul(usize::from(self.columns))
            .and_then(|start| start.checked_add(usize::from(column)))
            .ok_or(PlaceError::OffGrid)?;
        let square = self.squares.get_mut(index).ok_or(PlaceError::OffGrid)?;
        if *square != EMPTY {
            return Err(PlaceError::Occupied);
        }
        *square = tile;
        Ok(())
    }
}
```

```rust
use thiserror::Error;

pub const EMPTY: u8 = 0;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    #[error("place error: the square is off the grid")]
    OffGrid,
    #[error("place error: the square holds a tile already")]
    Occupied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub column: u16,
    pub row: u16,
}

#[derive(Debug)]
pub struct Grid {
    columns: u16,
    squares: Vec<u8>,
}

impl Grid {
    pub fn place(&mut self, position: Position, tile: u8) -> Result<(), PlaceError> {
        let square = self.square_mut(position).ok_or(PlaceError::OffGrid)?;
        if *square != EMPTY {
            return Err(PlaceError::Occupied);
        }
        *square = tile;
        Ok(())
    }

    fn square_mut(&mut self, position: Position) -> Option<&mut u8> {
        if position.column >= self.columns {
            return None;
        }
        let start = usize::from(position.row).checked_mul(usize::from(self.columns))?;
        self.squares.get_mut(start.checked_add(usize::from(position.column))?)
    }
}
```

A function extracted to name a step earns its place even with one caller. One
whose name says no more than its body, `fn add_tile(tiles: &mut Vec<u8>, tile:
u8) { tiles.push(tile) }`, is a jump that repays nothing, and is inlined.

Held by review. `clippy::too_many_lines` refuses a function past 100 lines, long
after it stopped doing one thing.

## Refusals First, Then the Main Path

Each refusal and each trivial case is handled where the function starts, and
returns, so what remains runs down the left margin with nothing left to hold.
Nesting puts the main path inside every condition that guards it, and each
refusal far below the test that causes it, where a reader must match an `else`
to its `if` by indentation. An `else` after a `return` adds a level for nothing.
In Rust, `?` returns an error and `let … else` a refusal, each on one line.

The rule is for refusals. A choice between two values, where both are the
function's answer, stays an expression, `if … else` or `match`, as `arrow` below
is: an early `return` there would hide that both arms are the result.

```rust
use thiserror::Error;

pub const EMPTY: u8 = 0;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    #[error("place error: a blank is no tile")]
    BlankTile,
    #[error("place error: the square is off the grid")]
    OffGrid,
    #[error("place error: the square holds a tile already")]
    Occupied,
}

#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    // Bad: the main path three levels in, and each refusal far from its test.
    pub fn place(&mut self, index: usize, tile: u8) -> Result<(), PlaceError> {
        if tile == EMPTY {
            Err(PlaceError::BlankTile)
        } else {
            if let Some(square) = self.squares.get_mut(index) {
                if *square == EMPTY {
                    *square = tile;
                    return Ok(());
                }
                return Err(PlaceError::Occupied);
            }
            Err(PlaceError::OffGrid)
        }
    }
}
```

```rust
use thiserror::Error;

pub const EMPTY: u8 = 0;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    #[error("place error: a blank is no tile")]
    BlankTile,
    #[error("place error: the square is off the grid")]
    OffGrid,
    #[error("place error: the square holds a tile already")]
    Occupied,
}

#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    pub fn place(&mut self, index: usize, tile: u8) -> Result<(), PlaceError> {
        if tile == EMPTY {
            return Err(PlaceError::BlankTile);
        }
        let square = self.squares.get_mut(index).ok_or(PlaceError::OffGrid)?;
        if *square != EMPTY {
            return Err(PlaceError::Occupied);
        }
        *square = tile;
        Ok(())
    }
}
```

Held by review, and in part by clippy: `clippy::manual_let_else` refuses a
`match` or an `if let` that could be a `let … else`, `clippy::redundant_else` an
`else` after a `return`, `clippy::collapsible_if` an `if` whose body is only
another, and `clippy::question_mark` a test for `None` that `?` would write. The
bad block above passes them all.

## Branch on a Value Once

A value with several cases is taken apart in one `match`, each case an arm, so
every case is written once, side by side, and a new case is added in one place.
A chain of `if … else if` that tests the same value again hides which cases it
covers, lets two tests overlap, and ends in an `else` that takes whatever comes
next, silently.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    North,
    East,
    South,
    West,
}

// Bad: the side tested three times, and a fifth side would be drawn as a
// left arrow.
#[must_use]
pub fn arrow(side: Side) -> char {
    if side == Side::North {
        '^'
    } else if side == Side::South {
        'v'
    } else if side == Side::East {
        '>'
    } else {
        '<'
    }
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    North,
    East,
    South,
    West,
}

#[must_use]
pub const fn arrow(side: Side) -> char {
    match side {
        Side::North => '^',
        Side::East => '>',
        Side::South => 'v',
        Side::West => '<',
    }
}
```

The `match` names every variant of an enum the crate owns: a wildcard arm, `_ =>
'<'`, would take a fifth side as silently as the chain's last `else`, and a new
variant would compile at every match that ought to decide what to do with it.
Variants that share an arm are joined with `|`, as `Self::Empty | Self::Claimed
{ .. }` below.

Held by review, and by `writing-rust`'s `references/api-design.md`, "A Match
Names Every Variant", with the lint that refuses a wildcard.

## A Value in One of Several States Is an Enum

When a thing is in one of several states, each with its own data, the states are
the variants of an enum, and each variant carries what that state has. A set of
flags and optional fields can say every combination, and most of them mean
nothing: claimed and sealed at once, sealed with no tile, claimed by nobody.
Every reader of such a struct must know which combinations are real, and every
writer must keep them so; an enum makes the others impossible to build, and a
`match` on it handles each state that is.

```rust
// Bad: four fields for three states: sixteen shapes, thirteen of them meaningless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Square {
    pub claimed: bool,
    pub sealed: bool,
    pub editor: Option<u32>,
    pub tile: Option<u8>,
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Square {
    Empty,
    Claimed { editor: EditorId },
    Sealed { tile: u8, editor: EditorId },
}

impl Square {
    #[must_use]
    pub const fn tile(self) -> Option<u8> {
        match self {
            Self::Sealed { tile, .. } => Some(tile),
            Self::Empty | Self::Claimed { .. } => None,
        }
    }
}
```

A state that decides which calls are valid at all, an editing board and a sealed
one, is a type parameter instead, so a wrong call does not compile:
`writing-rust`'s `references/api-design.md`, "State Lives in the Type".

Held by review. `clippy::struct_excessive_bools` refuses a struct of four bools
or more, and not the two above.

## A Choice Is an Enum or Two Functions, Never a Boolean Parameter

A `true` at a call site says nothing of what it chose: `grid.place(index, tile,
true)` could mean overwrite, wrap or check, and a reader must open the
definition to learn which. A choice the caller makes is an enum whose variants
name each side, so the call reads `grid.place(index, tile, Clash::Replace)`.
Where the two sides do different work, rather than one step differently, they
are two functions, `place` and `replace`, each named for what it does, and
neither carries the other's branch. Two booleans side by side are worse again:
`(true, false)` and `(false, true)` look alike and swap unseen.

```rust
use thiserror::Error;

pub const EMPTY: u8 = 0;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    #[error("place error: the square is off the grid")]
    OffGrid,
    #[error("place error: the square holds a tile already")]
    Occupied,
}

#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    // Bad: `grid.place(index, tile, true)` reads as nothing.
    pub fn place(&mut self, index: usize, tile: u8, overwrite: bool) -> Result<(), PlaceError> {
        let square = self.squares.get_mut(index).ok_or(PlaceError::OffGrid)?;
        if *square != EMPTY && !overwrite {
            return Err(PlaceError::Occupied);
        }
        *square = tile;
        Ok(())
    }
}
```

```rust
use thiserror::Error;

pub const EMPTY: u8 = 0;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    #[error("place error: the square is off the grid")]
    OffGrid,
    #[error("place error: the square holds a tile already")]
    Occupied,
}

/// What `place` does with a square that holds a tile already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clash {
    Refuse,
    Replace,
}

#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    pub fn place(&mut self, index: usize, tile: u8, clash: Clash) -> Result<(), PlaceError> {
        let square = self.squares.get_mut(index).ok_or(PlaceError::OffGrid)?;
        if *square != EMPTY && clash == Clash::Refuse {
            return Err(PlaceError::Occupied);
        }
        *square = tile;
        Ok(())
    }
}
```

The rule is for a `bool` that picks what the function does. A `bool` that is the
value being set or stored is data, not a choice, and stays a `bool`:
`square.set_visible(visible)` stores what it is given, and a caller that holds
the value as a `bool`, read from a file or a toggle, passes it straight on.
Fowler's "Flag Argument" makes the same exception, for a setter fed from a
boolean source.

Held by review. `clippy::fn_params_excessive_bools` refuses a function of four
boolean parameters or more; one, or two, pass it.

## Check Input Once, Where It Enters

Input from outside, a file, an argument, a request, is checked once, where it
enters, and turned into a type that can only hold what passed: a position on the
grid, a tile that is not blank. Past that point code takes the type, so holding
one is the proof, and no function inside checks again or can forget to. A
function that checks and then hands the raw value on, `validate(&input)?` and
then `input` used, makes every function after it trust a check it cannot see.

In Rust, `writing-rust`'s `references/api-design.md`, "Check at the Boundary,
and Let the Type Carry the Proof", gives the form, `FromStr` or `TryFrom` with a
typed error, and its example.

Held by review, as `writing-rust`'s API design says.

## No Check for a State the Types Rule Out

A check tells its reader that the state it tests can happen, so they go looking
for how, and find nothing. A guard against zero on a `NonZeroU8`, a `None` arm
for a value set a line before, a second bounds check on a position already
checked, a fallback for an error the type cannot hold: each is a branch that
never runs, and each drags a failure into a signature that has none, as the
`bool` below. Where an invariant is one the types cannot hold, it is stated
once, where it is relied on, and not tested again at every use.

```rust
use core::num::NonZeroU8;

#[derive(Debug, Default)]
pub struct Row {
    squares: Vec<NonZeroU8>,
}

impl Row {
    // Bad: a `NonZeroU8` is never zero, so the guard never returns, and the
    // `bool` reports a failure that cannot happen.
    pub fn push(&mut self, tile: NonZeroU8) -> bool {
        if tile.get() == 0 {
            return false;
        }
        self.squares.push(tile);
        true
    }
}
```

```rust
use core::num::NonZeroU8;

#[derive(Debug, Default)]
pub struct Row {
    squares: Vec<NonZeroU8>,
}

impl Row {
    pub fn push(&mut self, tile: NonZeroU8) {
        self.squares.push(tile);
    }
}
```

How an invariant is stated in Rust, an `expect` whose message names it or a
`debug_assert!`, is `writing-rust`'s `references/errors.md`.

Held by review. `clippy::absurd_extreme_comparisons` and rustc's
`unused_comparisons` refuse `x < 0` on an unsigned `x`; no lint knows what a
type of the crate's own rules out.

## Where a Comment Says What, the Code Says It

A comment that says what the next lines do repeats them, and nothing checks that
it still does: the code changes and the comment stays. A name for the value, a
function for the step, a type for the unit say it instead, and every use checks
them. A banner inside a function, `// Step 2: check the square`, marks a
function that wants its steps named: either the steps are small and the code
says them already, or each becomes a function. The comment that stays says why,
which the code cannot, and follows `writing-prose`.

```rust
use thiserror::Error;

pub const EMPTY: u8 = 0;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    #[error("place error: the square is off the grid")]
    OffGrid,
    #[error("place error: the square holds a tile already")]
    Occupied,
}

#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    pub fn place(&mut self, index: usize, tile: u8) -> Result<(), PlaceError> {
        // Bad: each banner says what the line under it says.
        // Step 1: find the square.
        let square = self.squares.get_mut(index).ok_or(PlaceError::OffGrid)?;
        // Step 2: refuse a square that is taken.
        if *square != EMPTY {
            return Err(PlaceError::Occupied);
        }
        // Step 3: place the tile.
        *square = tile;
        Ok(())
    }
}
```

```rust
use thiserror::Error;

pub const EMPTY: u8 = 0;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    #[error("place error: the square is off the grid")]
    OffGrid,
    #[error("place error: the square holds a tile already")]
    Occupied,
}

#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    pub fn place(&mut self, index: usize, tile: u8) -> Result<(), PlaceError> {
        let square = self.squares.get_mut(index).ok_or(PlaceError::OffGrid)?;
        if *square != EMPTY {
            return Err(PlaceError::Occupied);
        }
        *square = tile;
        Ok(())
    }
}
```

Held by review.

## In the Shell

A script is read top to bottom more than any other code, so the rules hold
harder there:

- **Refusals first**: `[[ -f $grid ]] || die "no grid at $grid"` at the top of a
  function, and the main path after, unindented.
- **One `case` on a value**, over an `if … elif` chain that tests it again.
- **A choice is a named option parsed once**, `--replace`, into a variable whose
  value names the choice, `clash=replace`, never a positional `true` or `1`.
- **A step is a function named for it**, and the script's main part reads as the
  list of steps.

```bash
# Bad: a positional flag, and the refusal at the bottom.
place() {
    if [[ -f $1 ]]; then
        if [[ $3 == true ]] || ! grep -q "^$2 " "$1"; then
            echo "$2 $4" >> "$1"
        fi
    else
        echo "no grid" >&2
        return 1
    fi
}
```

```bash
place() {
    local grid=$1 index=$2 tile=$3 clash=$4
    [[ -f $grid ]] || { echo "place: no grid at $grid" >&2; return 1; }
    if [[ $clash == refuse ]] && grep -q "^$index " "$grid"; then
        echo "place: square $index holds a tile already" >&2
        return 1
    fi
    awk -v square="$index" '$1 != square' "$grid" > "$grid.new"
    echo "$index $tile" >> "$grid.new"
    mv "$grid.new" "$grid"
}
```

Held by review.
