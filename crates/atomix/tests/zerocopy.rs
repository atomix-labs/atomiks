//! A page of memory a process lays out to share, as a header of atomics with zerocopy: read from
//! the page's bytes and stored to through them, and read by value or from zeros.

#![cfg(all(feature = "derive", feature = "zerocopy-08"))]
// The derives are absent under loom, whose cells are not plain memory.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    use core::num::NonZero;

    use atomix::ordering::{Acquire, Release};
    use atomix::{Atom, AtomBitwise, Atomic, AtomicU64};
    use zerocopy::{FromBytes, FromZeros, IntoBytes, KnownLayout};

    /// A page's permissions, a bit each.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom, AtomBitwise)]
    struct Permissions(u64);

    /// A shared page's header. Every repr of each field decodes, so it reads from any bytes, and
    /// each field is 8 bytes wide, so `repr(C)` leaves no padding for `IntoBytes` to refuse.
    #[derive(FromBytes, IntoBytes, KnownLayout)]
    #[repr(C)]
    struct Header {
        /// The writer's sequence number.
        seq: AtomicU64,
        /// The owner's id, or `None` while the page is free.
        owner: Atomic<Option<NonZero<u64>>>,
        /// The page's permissions.
        permissions: Atomic<Permissions>,
    }

    #[test]
    fn a_header_reads_from_a_page_and_stores_through_it() {
        // Words, so the page is aligned as `Header` is.
        let mut page = [7_u64, 0, 0b101];
        {
            let header = Header::mut_from_bytes(page.as_mut_bytes()).expect("24 aligned bytes");
            // Shared from here on, as every thread that uses the page shares it.
            let header: &Header = header;
            assert_eq!(header.seq.load(Acquire), 7, "the sequence number");
            assert_eq!(header.owner.load(Acquire), None, "no owner: zero");
            assert_eq!(header.permissions.load(Acquire), Permissions(0b101), "the permissions");
            header.owner.store(NonZero::new(42), Release);
            header.permissions.or(Permissions(0b010), Release);
        }
        assert_eq!(page, [7, 42, 0b111], "each store, in the page");
    }

    #[test]
    fn a_header_reads_by_value_and_from_zeros() {
        let copy = Header::read_from_bytes([7_u64, 42, 0b101].as_bytes()).expect("24 bytes");
        assert_eq!(copy.owner.into_inner(), NonZero::new(42), "the owner, read by value");
        let zeroed = Header::new_zeroed();
        assert_eq!(
            (zeroed.seq.into_inner(), zeroed.owner.into_inner(), zeroed.permissions.into_inner()),
            (0, None, Permissions(0)),
            "a zeroed header: 0, free, no permissions"
        );
    }
}
