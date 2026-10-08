//! `get_mut` lends the cell as `&mut T`, so `T` must be its own repr; `with_mut` lends the rest.

use core::num::NonZero;

use atomix_core::Atomic;

fn main() {
    let mut id = Atomic::new(NonZero::<u64>::MIN);
    let _ = id.get_mut();
    let mut letter = Atomic::new('a');
    let _ = letter.get_mut();
}
