//! deranged orders `None` below every value, but from 0 it keeps `None` at 255, above them all: the
//! repr's order is not the value's, so an optional ranged integer has no `AtomOrd`, which an
//! atomic's `fetch_max` and `fetch_min` need.

use atomiks_core::AtomOrd;
use deranged::OptionRangedU8;

/// Compiles only where `T`'s reprs order as its values.
const fn orders_as_its_reprs<T: AtomOrd>() {}

fn main() {
    orders_as_its_reprs::<OptionRangedU8<0, 10>>();
}
