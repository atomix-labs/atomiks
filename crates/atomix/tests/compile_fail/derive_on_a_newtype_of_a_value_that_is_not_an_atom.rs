//! A newtype takes its repr from its field, so one over a value that is no `Atom` is refused once,
//! at the field, and its impl, which assumes the field's, raises no error of its own.

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
struct Ticker([u8; 4]);

fn main() {}
