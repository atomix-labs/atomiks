//! The ranged integers need no `#![feature]`: in a `static`, in code generic over their bounds,
//! under a newtype's derives, and as a pattern, matched or not.

use std::collections::BTreeSet;

use atomiks_core::{RangedI8, RangedU64};

/// A thread's id as a lock's owner, from 3: a lock keeps 1 and 2 for its own states.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct OwnerId(RangedU64<3>);

/// The first id handed out.
static FIRST: OwnerId = OwnerId(RangedU64::MIN);
/// The id after it, built in a constant.
const SECOND: RangedU64<3> = RangedU64::MIN.saturating_add(1);

/// How far a price moved in all, from moves of `MIN` to `MAX` ticks each.
fn net<const MIN: i8, const MAX: i8>(moves: &[RangedI8<MIN, MAX>]) -> i32 {
    moves.iter().map(|price_move| i32::from(price_move.get())).sum()
}

/// Halves `value`, keeping it within the range.
fn halve<const MIN: u64, const MAX: u64>(value: RangedU64<MIN, MAX>) -> RangedU64<MIN, MAX> {
    RangedU64::new_saturating(value.get() / 2)
}

fn main() {
    let owners = BTreeSet::from([OwnerId(SECOND), FIRST]);
    assert_eq!(owners.first(), Some(&FIRST), "ordered as the integers");
    let moves = [RangedI8::<-5, 5>::MIN, RangedI8::MAX, RangedI8::MAX];
    assert_eq!(net(&moves), 5, "-5 + 5 + 5");
    assert_eq!(halve(RangedU64::<3, 9>::MAX).get(), 4, "9 halved");
    assert_eq!(halve(RangedU64::<3, 9>::MIN).get(), 3, "and 3 halved, held to the range");
    match FIRST.0 {
        SECOND => panic!("the first id is 3, not 4"),
        _ => {},
    }
    match RangedU64::<3>::new(4) {
        Some(SECOND) => {},
        _ => panic!("4 is the second id"),
    }
}
