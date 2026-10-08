//! The derive reads a marker by its name, `PhantomData`, and lays it out in no bits, so a field of
//! another type under that name would decode from bits it never stored: its codecs take core's
//! `PhantomData`, and the build fails.

use core::num::NonZeroU8 as PhantomData;

use atomix::Atom;

#[derive(Clone, Copy, Atom)]
struct Order {
    quantity: u8,
    live: bool,
    venue: PhantomData,
}

fn main() {}
