//! A value whose name comes near a place's derives `Atom`, and is a field of a derived atom: the
//! derive refuses a field written as an atomic or a cell by its exact name and arguments.

use atomix::{Atom, Atomic};

/// An element's atomic number: a value, not an atomic.
#[derive(Clone, Copy, Atom)]
pub struct AtomicNumber(pub u8);

/// What fills a cell of a grid: a value of no type argument, as no `core::cell::Cell` is.
#[derive(Clone, Copy, Atom)]
pub enum Cell {
    /// Nothing.
    Empty,
    /// A wall.
    Wall,
}

/// A tile of the grid.
#[derive(Clone, Copy, Atom)]
pub struct Tile {
    /// Its element.
    pub number: AtomicNumber,
    /// What fills it.
    pub cell: Cell,
    /// Whether it was seen.
    pub seen: bool,
}

/// The tile under the cursor.
pub static TILE: Atomic<Tile> =
    Atomic::new(Tile { number: AtomicNumber(1), cell: Cell::Wall, seen: false });

fn main() {}
