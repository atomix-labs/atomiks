//! The cell, as production code reaches it.

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    use atomiks::cell::UnsafeCell;

    #[test]
    fn the_windows_and_guards_reach_the_contents() {
        let cell = UnsafeCell::new(1_u64);
        // SAFETY: the closure's pointer is the only access to the cell while it runs.
        #[expect(unsafe_code, reason = "writing through the pointer the cell lends")]
        cell.with_mut(|value| unsafe { *value = 2 });
        // SAFETY: as above.
        #[expect(unsafe_code, reason = "reading through the pointer the cell lends")]
        let read = cell.with(|value| unsafe { *value });
        assert_eq!(read, 2, "the shared window sees the exclusive one's write");
        {
            let writer = cell.get_mut();
            // SAFETY: the cell lives in place, and the guard is its only access while it lives.
            #[expect(unsafe_code, reason = "writing through the guard the cell lends")]
            unsafe {
                *writer.deref() = 3;
            }
        }
        let seen = {
            let reader = cell.get();
            // SAFETY: the cell lives in place, and the guard is its only access while it lives.
            #[expect(unsafe_code, reason = "reading through the guard the cell lends")]
            unsafe {
                *reader.deref()
            }
        };
        assert_eq!(seen, 3, "the shared guard sees the exclusive one's write");
        assert_eq!(cell.into_inner(), 3, "and so does unwrapping the cell");
    }

    #[test]
    fn the_pointers_reach_the_contents() {
        let cell = UnsafeCell::new(1_u64);
        // SAFETY: the pointer is the live cell's own, and nothing else reaches the cell.
        #[expect(unsafe_code, reason = "writing through the address the cell gives")]
        unsafe {
            *cell.as_ptr() = 2;
        }
        let contents = UnsafeCell::raw_get(&raw const cell);
        assert_eq!(contents, cell.as_ptr(), "both name the contents");
        // SAFETY: as above.
        #[expect(unsafe_code, reason = "reading through the address the cell gives")]
        let read = unsafe { *contents };
        assert_eq!(read, 2, "the write through `as_ptr`");
    }
}
