# Naming

Read this before naming or renaming anything: a module, a type, a function, a
method, a variable, a field, a constant or a file. A name is read at every use,
far from its definition, by someone who has not read the body, so it must say
alone what the thing is or does. The rules here hold in any language, and the
examples are Rust; where the repository applies rust-lints, python or shell, a
section at the end says what that language adds.

## A Name Says What the Thing Is, in the Domain's Words

`data`, `info`, `item`, `value`, `obj`, `result`, `tmp`, `manager`, `handler`,
`processor`, `service`, `util`, `helper`, `process`, `handle` and `do_` fit
anything, so they tell a reader nothing, and they fit the next thing someone
adds as well, which is how a `Manager` ends up holding half a crate. The
domain's word says what the thing holds and what it may not: a `square` holds a
tile, a `row` holds squares, `place` puts a tile on a square. Where no domain
word fits, the thing is not one thing yet: a `TileManager` that places, clears
and draws is three operations that belong on the grid they act on.

```rust
// Bad: a manager of data, whose one method processes info.
#[derive(Debug, Default)]
pub struct TileManager {
    data: Vec<u8>,
}

impl TileManager {
    pub fn process(&mut self, info: u8) {
        self.data.push(info);
    }
}
```

```rust
#[derive(Debug, Default)]
pub struct Row {
    squares: Vec<u8>,
}

impl Row {
    pub fn push(&mut self, tile: u8) {
        self.squares.push(tile);
    }
}
```

Held by review: `clippy::disallowed_names` refuses only its own short list,
`foo`, `baz` and `quux`, and nothing refuses `data`.

## A Name Does Not Repeat Its Context

A reader already sees the module a type is in, the receiver a method is called
on, and the type a value has. `grid::GridCursor` reads "grid" twice at every
use, `grid.grid_cols()` twice at every call, and a field `grid_rows` on a `Grid`
twice at every read; the repeated word pushes the part that differs to the end.
A type in a name, `tile_vec` or `name_str`, says what the signature already says
and goes stale when the type changes.

```rust
// Bad: every name says "grid" again, inside the grid.
mod grid {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GridCursor {
        pub grid_col: u16,
        pub grid_row: u16,
    }

    impl GridCursor {
        #[must_use]
        pub const fn grid_origin() -> Self {
            Self { grid_col: 0, grid_row: 0 }
        }
    }
}

pub use crate::grid::GridCursor;
```

```rust
mod grid {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Cursor {
        pub col: u16,
        pub row: u16,
    }

    impl Cursor {
        pub const ORIGIN: Self = Self { col: 0, row: 0 };
    }
}

pub use crate::grid::Cursor;
```

Where a name is exported flat from a crate's root and read without its module,
it keeps the word it needs to stand alone: `tiles::Cursor` reads well, and a
crate with two cursors names each for what it points at.

Held by review. `clippy::struct_field_names` and `clippy::enum_variant_names`
refuse fields and variants that repeat their type's name, but only in a type the
crate does not export; `clippy::module_name_repetitions` is not among the lints
the workspace turns on.

## A Function's Name Says All It Does

A caller reads the name, not the body, and trusts it. A name that reads as a
question, `tile_at`, `is_full`, promises to change nothing; a name for a command
says what it changes. One that needs "and", `place_and_record`, does two things,
and is two functions, or one whose name says the one thing the two make
together. One whose body writes, waits, allocates or reaches the file system
where its name says none of that misleads, and the fault it causes is found
last.

```rust
#[derive(Debug, Default)]
pub struct Grid {
    squares: Vec<u8>,
    reads: u32,
}

impl Grid {
    // Bad: a question that also counts, so a caller asking twice changes the
    // grid twice.
    pub fn tile_at(&mut self, at: usize) -> Option<u8> {
        self.reads = self.reads.saturating_add(1);
        self.squares.get(at).copied()
    }
}
```

```rust
#[derive(Debug, Default)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    #[must_use]
    pub fn tile_at(&self, at: usize) -> Option<u8> {
        self.squares.get(at).copied()
    }
}
```

Held by review, which reads the body against the name. In Rust, a method named
as a question that takes `&mut self` is where to look first.

## A Name's Length Follows Its Reach

A name used in three lines is read with its definition in view, so it can be
short: `at` in a closure, `row` in a loop over rows. A name that crosses a
function or a module is read where its definition is out of sight, so it is
whole words. An abbreviation is used only as the domain writes it, `id`, `url`,
`io`, `utf8`, never one made up to save letters, `cnt`, `plc`, `calc_tl`, which
the reader must decode at each use.

```rust
// Bad: a public name cut short, and a local one made long.
#[must_use]
pub fn cnt_emp_sq(sq: &[Option<u8>]) -> usize {
    sq.iter().filter(|each_square_in_the_row| each_square_in_the_row.is_none()).count()
}
```

```rust
#[must_use]
pub fn empty_squares(squares: &[Option<u8>]) -> usize {
    squares.iter().filter(|square| square.is_none()).count()
}
```

Held by review. `clippy::many_single_char_names` and `clippy::similar_names`
catch the worst of it among local bindings; nothing sees an abbreviation.

## A Literal That Means Something Has a Name

A number or a string that stands for something, a grid's widest row, the tile
that means an empty square, is a named constant, so its meaning is written once
beside its value, and a change to the value is one edit. A literal that is only
itself, the `0` a count starts from, the `1` a step adds, stays a literal.

```rust
// Bad: what 64 and 0 mean is left for the reader to guess, in each place.
#[must_use]
pub fn has_room(cols: u16, squares: &[u8]) -> bool {
    cols <= 64 && squares.contains(&0)
}
```

```rust
#[derive(Debug)]
pub struct Grid;

impl Grid {
    /// The widest row a grid is laid with.
    pub const MAX_COLS: u16 = 64;
    /// The tile an empty square holds.
    pub const EMPTY: u8 = 0;

    #[must_use]
    pub fn has_room(cols: u16, squares: &[u8]) -> bool {
        cols <= Self::MAX_COLS && squares.contains(&Self::EMPTY)
    }
}
```

Held by review. Where the constant lives in Rust, on its type, is
`writing-rust`'s `references/naming.md`, "An Associated Constant Names the
Value".

## A Condition Is Named for What Is True

A boolean's name says the state that holds when it is true, `visible`,
`is_full()`, and the code asks `if grid.is_full()`, never `if
!grid.is_not_full()`. A negated name is read through a negation at every use,
and a double one through two. A condition of several parts gets a name, a local
or a method, so the reader meets what it means before how it is computed.

```rust
pub const EMPTY: u8 = 0;

#[derive(Debug)]
pub struct Square {
    pub hidden: bool,
    pub tile: u8,
}

// Bad: the name is the negation of what the caller wants, and the compound
// condition says how, not what.
#[must_use]
pub const fn is_drawn(square: &Square, cols: u16, col: u16) -> bool {
    !square.hidden && square.tile != EMPTY && col < cols
}
```

```rust
pub const EMPTY: u8 = 0;

#[derive(Debug)]
pub struct Square {
    pub visible: bool,
    pub tile: u8,
}

impl Square {
    #[must_use]
    pub const fn has_visible_tile(&self) -> bool {
        self.visible && self.tile != EMPTY
    }
}

#[must_use]
pub const fn is_drawn(square: &Square, cols: u16, col: u16) -> bool {
    let on_grid = col < cols;
    on_grid && square.has_visible_tile()
}
```

Held by review. `clippy::nonminimal_bool` refuses a negation it can simplify
away, `!(a != b)`, and `clippy::if_not_else` an `if !x { … } else { … }`;
neither sees a negated name.

## One Word for One Thing

The domain's word for a thing is used for it everywhere, in names, docs,
messages and tests, and a different thing gets a different word. A second word
for one thing makes a reader look for a second thing; one word for two things
makes them take one for the other. A codebase that has settled a word keeps it,
and a new name uses it.

In Rust, `writing-rust`'s `references/naming.md`, "One Word for One Thing",
shows it, and its "A Type's Name Says Its Role" gives the words a type's name
ends with, which hold here in any language.

Held by review, as `writing-rust`'s naming says.

## A Module Is Named for What It Holds

A module is a short noun for the concept it holds, `grid`, `tile`, `walk`, so a
reader knows where to look for a thing and where a new thing goes. `utils`,
`helpers`, `common`, `misc` and `shared` name nothing, so they take in whatever
comes next, and grow until nobody knows what is in them. A function that seems
to have no home belongs on the type it works on, beside the code that calls it,
or in a module named for the concept it serves.

In Rust, `writing-rust`'s `references/layout.md`, "A Module Is Named for What It
Holds", gives the example and the names the workspace fixes: `errors.rs`,
`testing.rs`, `consts.rs` and `tests/testing/mod.rs`.

Held by review, as `writing-rust`'s layout says.

## In Rust

Casing, RFC 430's, is rustc's to hold, with `non_snake_case`,
`non_camel_case_types` and `non_upper_case_globals`.

Everything else Rust asks of a name is `writing-rust`'s, in its
`references/naming.md`:

- "A Getter Is Its Noun, Without `get_`".
- "`as_`, `to_` and `into_` Say What a Conversion Costs".
- "`try_` Can Refuse, `_with` Takes a Closure, `_in` Takes an Allocator".
- "`new` Builds, `create` Lays, `open` Binds".
- "A Boolean Reads as a Question", for `is_`, `has_` and `can_`.
- "Iterators Are `iter`, `iter_mut` and `into_iter`".
- "A Type's Name Says Its Role: `Spec`, `Config`, `Guard`, `Error`".
- "An Acronym Is One Word", and "A Type Parameter Is One Capital Letter".

Read it before naming anything a caller sees.

## In the Shell

A variable the environment gives or a script exports is `UPPER_CASE`; a script's
own variables and its functions are `lower_case`, so a reader tells at a glance
what came from outside. Every variable a function sets is `local`, or it leaks
into the caller's scope under a name the caller may use. A function is a verb
for what it does, `place_tile`, and a script's file name says what running it
does.

```bash
# Bad: a function's variables leak, and nothing tells an input from a local.
f() {
    TILE=$1
    RESULT=$(grep -c "$TILE" grid.txt)
    echo "$RESULT"
}
```

```bash
count_tile() {
    local tile=$1
    grep -c -- "$tile" "${GRID_FILE:?}"
}
```

Held by review: ShellCheck's `SC2034` refuses a variable nothing reads, but no
check sees a missing `local` or a name's case.
