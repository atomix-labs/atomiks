//! Covariance would let a store leave a short-lived value where a `'static` view reads it.

use core::marker::PhantomData;

use atomix_core::Atomic;

/// A value with a lifetime.
type R<'a> = PhantomData<&'a ()>;

fn shrink<'a>(x: &'a Atomic<R<'static>>) -> &'a Atomic<R<'a>> {
    x
}

fn main() {
    let _ = shrink(&Atomic::new(PhantomData));
}
