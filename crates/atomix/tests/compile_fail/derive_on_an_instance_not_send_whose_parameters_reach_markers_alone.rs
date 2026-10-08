//! A packed value laid out once may still not cross threads where a marker's parameter keeps it
//! from it: its impl is bounded `Send` and `Sync`, so its atomic is refused at the instance.

use core::marker::PhantomData;

use atomix::{Atom, Atomic};

#[derive(Clone, Copy, Atom)]
struct Handle<T> {
    index: u32,
    live: bool,
    kind: PhantomData<T>,
}

fn main() {
    let _ = Atomic::new(Handle::<*const u8> { index: 0, live: false, kind: PhantomData });
}
