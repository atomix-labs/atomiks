//! A fieldless enum derives `Atom` without parameters: none could change what it stores.

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
enum Level<const N: usize> {
    Low,
    High,
}

fn main() {}
