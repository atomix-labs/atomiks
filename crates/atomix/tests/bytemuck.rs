//! A ring buffer's slot of derived values, zeroed with bytemuck, since every field's zero decodes.

#![cfg(all(feature = "derive", feature = "bytemuck"))]
// `Zeroable` for an atomic is absent under loom, whose cells are not plain memory.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    use atomix::ordering::Acquire;
    use atomix::validity::{Partial, ZeroValid};
    use atomix::{Atom, Atomic, RangedU32};
    use bytemuck::Zeroable;

    /// Compiles only where `T`'s validity is `V`.
    const fn validity_is<T: Atom<Validity = V>, V>() {}

    /// An owner's id, from 1.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    struct OwnerId(RangedU32<1>);

    /// A slot's lock. Its discriminants are stated, so `Uninit` takes zero, with a tag beside the
    /// id, whose 32 bits leave room for it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[repr(u8)]
    enum Lock {
        /// Never set up.
        Uninit = 0,
        /// Free to take.
        Free   = 1,
        /// Held by its owner.
        Owned(OwnerId) = 2,
    }

    /// What a slot holds. `repr(u8)`, since Rust allows discriminants beside fields only with one.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
    #[repr(u8)]
    enum State {
        /// Nothing written.
        Empty   = 0,
        /// Being written, in a lap.
        Writing {
            /// The lap.
            lap: u32,
        } = 1,
        /// Written, in a lap.
        Ready {
            /// The lap.
            lap: u32,
        } = 2,
    }

    /// A ring buffer's slot, all zeros when new: `Lock::Uninit` and `State::Empty`.
    #[derive(Zeroable)]
    #[repr(C)]
    struct Slot {
        /// The lock.
        lock: Atomic<Lock>,
        /// What the slot holds.
        state: Atomic<State>,
    }

    #[test]
    fn an_atomic_of_an_enum_whose_zero_discriminant_has_no_field_is_zeroable() {
        // A ranged id promises nothing of zero, but the lock's zero is `Uninit`'s tag alone.
        validity_is::<OwnerId, Partial>();
        validity_is::<Lock, ZeroValid>();
        validity_is::<State, ZeroValid>();
        assert_eq!(Atomic::<Lock>::zeroed().into_inner(), Lock::Uninit, "`Uninit`, at zero");
        assert_eq!(Atomic::<State>::zeroed().into_inner(), State::Empty, "`Empty`, at zero");
    }

    #[test]
    fn a_slot_of_atomics_whose_zero_decodes_is_zeroable() {
        let slot = Slot::zeroed();
        assert_eq!(slot.lock.load(Acquire), Lock::Uninit, "the lock, never set up");
        assert_eq!(slot.state.load(Acquire), State::Empty, "and nothing written");
    }
}
