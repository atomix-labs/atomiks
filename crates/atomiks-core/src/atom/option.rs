//! `Option` of an atom, spending one spare repr on `None`.

use core::any::type_name;

use super::Atom;
use crate::message::{Message, refuse};
use crate::primitive::Primitive;
use crate::range::ReprRange;
use crate::validity::Validity;

/// The length of what `refuse_option` writes around its advice once `T`'s name is cut to `…`, so
/// an advice that names a repr leaves room for it.
const FRAME_LEN: usize = "`Option<…>`: ".len();

/// Refuses the build of `Option<T>` with `advice`.
///
/// Cuts `T`'s name short where the advice would not fit after it.
const fn refuse_option<T>(advice: &Message) -> ! {
    let close = ">`: ";
    let advice = advice.as_str();
    refuse(
        &Message::new()
            .text("`Option<")
            .name(type_name::<T>(), close.len().saturating_add(advice.len()))
            .text(close)
            .text(advice),
    )
}

/// The repr `None` takes: zero where `T`'s validity says zero does not decode, wherever `T`'s range
/// lies, or where `T::Repr` is a pointer, which marks `None` only with null; else the repr beside
/// `T`'s range that [`ReprRange::spare_for_none`] picks.
///
/// Refuses the build where zero decodes and `T`'s range holds every repr, or a pointer's holds
/// null. Every caller evaluates it in a `const {}` block or a constant, so a refused `Option<T>`
/// never builds: the impl's soundness rests on that.
const fn none_repr<T: Atom>() -> u128 {
    if <T::Validity as Validity>::NONE_TAKES_ZERO {
        return 0;
    }
    let Some(spare) = T::REPRS.spare_for_none() else {
        let rest = concat!(
            "`, so `None` has none left: ",
            "use a type with a spare repr, such as `NonZero<u64>`"
        );
        refuse_option::<T>(
            &Message::new()
                .text("its range holds every repr of `")
                .name(type_name::<T::Repr>(), rest.len().saturating_add(FRAME_LEN))
                .text(rest),
        );
    };
    if !<T::Repr as Primitive>::IS_BITS_EXACT {
        if T::REPRS.contains(0) {
            refuse_option::<T>(&Message::new().text(concat!(
                "a pointer repr marks `None` only with null, but null lies in the value's range: ",
                "leave it out, as `NonNull`'s `ReprRange::NONZERO` does"
            )));
        }
        return 0;
    }
    spare
}

// SAFETY: `None` takes a repr no value takes: zero where `T`'s validity says zero does not decode,
// else one outside `T`'s range; the range claimed is `T`'s grown to hold it. `from_repr` tests for
// it first, exactly: a pointer's `is_bits` decides only zero, and `none_repr` gives a pointer's
// `None` no other repr. It takes zero wherever `T`'s validity says, so the validity is `T`'s once
// that repr is spent; an `Option` of a value that may cross threads may too.
#[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
const unsafe impl<T: [const] Atom> Atom for Option<T> {
    type Repr = T::Repr;
    type Validity = <T::Validity as Validity>::Optional;
    const REPRS: ReprRange<T::Repr> = T::REPRS.including(none_repr::<T>());
    #[inline]
    fn to_repr(self) -> T::Repr {
        match self {
            Some(value) => value.to_repr(),
            None => <T::Repr as Primitive>::from_bits(const { none_repr::<T>() }),
        }
    }
    #[inline]
    fn from_repr(repr: T::Repr) -> Option<Self> {
        if repr.is_bits(const { none_repr::<T>() }) {
            return Some(None);
        }
        match T::from_repr(repr) {
            Some(value) => Some(Some(value)),
            None => None,
        }
    }
    #[inline]
    unsafe fn from_repr_unchecked(repr: T::Repr) -> Self {
        if repr.is_bits(const { none_repr::<T>() }) {
            return None;
        }
        // SAFETY: the caller's repr decodes, and it is not `None`'s, so it decodes as `T`.
        Some(unsafe { T::from_repr_unchecked(repr) })
    }
}
