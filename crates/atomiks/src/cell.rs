//! A cell for data that is not one atomic word, shaped as loom's, so the same code is the model.
//!
//! Off loom a window or guard is only a pointer; under loom each is tracked, and a conflicting
//! access is reported.

#[cfg(loom)]
pub use loom::cell::{ConstPtr, MutPtr, UnsafeCell};

#[cfg(not(loom))]
pub use self::native::{ConstPtr, MutPtr, UnsafeCell};

#[cfg(not(loom))]
mod native {
    //! Core's `UnsafeCell` behind loom's surface.

    use core::cell;

    /// A cell whose contents are reached through raw pointers, in windows the caller keeps
    /// exclusive or shared.
    ///
    /// Off loom it has `T`'s layout.
    #[repr(transparent)]
    #[derive(Debug, Default)]
    pub struct UnsafeCell<T: ?Sized>(cell::UnsafeCell<T>);

    impl<T> UnsafeCell<T> {
        /// A cell holding `value`.
        #[inline]
        #[must_use]
        pub const fn new(value: T) -> Self {
            Self(cell::UnsafeCell::new(value))
        }

        /// The value, consuming the cell.
        #[inline]
        #[must_use]
        pub fn into_inner(self) -> T {
            self.0.into_inner()
        }

        /// The contents' address, from the cell's, without a reference to the cell; off loom only.
        #[inline]
        #[must_use]
        pub const fn raw_get(this: *const Self) -> *mut T {
            cell::UnsafeCell::raw_get(this.cast::<cell::UnsafeCell<T>>())
        }
    }

    impl<T: ?Sized> UnsafeCell<T> {
        /// Calls `f` with a pointer for reading; `f` must not keep it.
        #[inline]
        pub fn with<R, F: FnOnce(*const T) -> R>(&self, f: F) -> R {
            f(self.0.get().cast_const())
        }

        /// Calls `f` with a pointer for writing; `f` must not keep it.
        #[inline]
        pub fn with_mut<R, F: FnOnce(*mut T) -> R>(&self, f: F) -> R {
            f(self.0.get())
        }

        /// A guard for reading, open while it lives.
        #[inline]
        #[must_use]
        pub const fn get(&self) -> ConstPtr<T> {
            ConstPtr(self.0.get().cast_const())
        }

        /// A guard for writing, open while it lives.
        #[inline]
        #[must_use]
        pub const fn get_mut(&self) -> MutPtr<T> {
            MutPtr(self.0.get())
        }

        /// The contents' address, off loom only.
        #[inline]
        #[must_use]
        pub const fn as_ptr(&self) -> *mut T {
            self.0.get()
        }
    }

    impl<T> From<T> for UnsafeCell<T> {
        #[inline]
        fn from(value: T) -> Self {
            Self::new(value)
        }
    }

    /// A guard for reading a cell.
    #[repr(transparent)]
    #[derive(Debug)]
    pub struct ConstPtr<T: ?Sized>(*const T);

    impl<T: ?Sized> ConstPtr<T> {
        /// The contents, shared.
        ///
        /// # Safety
        /// The cell is alive and has not moved, and nothing writes the contents while the borrow
        /// lives.
        #[expect(unsafe_code, reason = "a shared borrow of contents the caller keeps unwritten")]
        #[inline]
        #[must_use]
        pub const unsafe fn deref(&self) -> &T {
            // SAFETY: the pointer came from the cell, which the caller keeps alive and in place,
            // with writers out.
            unsafe { &*self.0 }
        }

        /// Calls `f` with the pointer.
        #[inline]
        pub fn with<R, F: FnOnce(*const T) -> R>(&self, f: F) -> R {
            f(self.0)
        }
    }

    /// A guard for writing a cell.
    #[repr(transparent)]
    #[derive(Debug)]
    pub struct MutPtr<T: ?Sized>(*mut T);

    impl<T: ?Sized> MutPtr<T> {
        /// The contents, exclusively.
        ///
        /// # Safety
        /// The cell is alive and has not moved, and nothing else reads or writes the contents while
        /// the borrow lives. Each call makes a new exclusive borrow, so two that overlap break the
        /// contract.
        #[expect(unsafe_code, reason = "an exclusive borrow of contents the caller keeps unshared")]
        #[expect(
            clippy::mut_from_ref,
            reason = "loom's shape; exclusivity is the caller's contract"
        )]
        #[inline]
        #[must_use]
        pub const unsafe fn deref(&self) -> &mut T {
            // SAFETY: the pointer came from the cell, which the caller keeps alive and in place,
            // with every other access out.
            unsafe { &mut *self.0 }
        }

        /// Calls `f` with the pointer.
        #[inline]
        pub fn with<R, F: FnOnce(*mut T) -> R>(&self, f: F) -> R {
            f(self.0)
        }
    }
}
