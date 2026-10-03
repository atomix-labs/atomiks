//! `Option` of an atom, spending one spare repr on `None`.

use core::any::type_name;

use super::Atom;
use crate::message::{Message, refuse};
use crate::primitive::Primitive;
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

/// The repr `None` takes beside `T`'s: one below its range, else one above it.
///
/// Refuses the build where `T` leaves no such repr, or where its validity or repr cannot give that
/// one to `None`. Every caller evaluates it in a `const {}` block or a constant, so a refused
/// `Option<T>` never builds: the impl's soundness rests on that.
const fn none_repr<T: Atom>() -> u128 {
    if T::MIN_REPR > T::MAX_REPR {
        refuse_option::<T>(
            &Message::new()
                .text("the value's `MIN_REPR`, ")
                .number(T::MIN_REPR)
                .text(", is above its `MAX_REPR`, ")
                .number(T::MAX_REPR)
                .text(": make the range span every repr `to_repr` returns"),
        );
    }
    let top = u128::MAX.unbounded_shr(128_u32.wrapping_sub(<T::Repr as Primitive>::BITS));
    let none = if T::MIN_REPR > 0 {
        T::MIN_REPR.wrapping_sub(1)
    } else {
        if T::MAX_REPR >= top {
            let rest = concat!(
                "`, so `None` has none left: ",
                "use a type with a spare repr, such as `NonZero<u64>`"
            );
            refuse_option::<T>(
                &Message::new()
                    .text("its values fill every repr of `")
                    .name(type_name::<T::Repr>(), rest.len().saturating_add(FRAME_LEN))
                    .text(rest),
            );
        }
        T::MAX_REPR.wrapping_add(1)
    };
    if none > top {
        let rest = "` cannot hold: keep `MIN_REPR` and `MAX_REPR` within its range";
        refuse_option::<T>(
            &Message::new()
                .text("`None` would take repr ")
                .number(none)
                .text(", which `")
                .name(type_name::<T::Repr>(), rest.len().saturating_add(FRAME_LEN))
                .text(rest),
        );
    }
    if <T::Validity as Validity>::NONE_TAKES_ZERO && none != 0 {
        refuse_option::<T>(
            &Message::new()
                .text("the value's validity gives `None` repr 0, ")
                .text("so its `MIN_REPR` must be 1, not ")
                .number(T::MIN_REPR)
                .text(": make it 1, or use a validity without a zero niche"),
        );
    }
    if none != 0 && !<T::Repr as Primitive>::IS_BITS_EXACT {
        refuse_option::<T>(
            &Message::new()
                .text("`None` would take repr ")
                .number(none)
                .text(", but a pointer repr marks `None` only with null: ")
                .text("make null no value and `MIN_REPR` 1, as `NonNull` does"),
        );
    }
    none
}

// SAFETY: `None` takes the repr just outside `T`'s range, and the range claimed spans both;
// `from_repr` tests for it first, exactly: a pointer's `is_bits` decides only zero, and
// `none_repr` refuses a nonzero `None` on such a repr. It takes zero wherever `T`'s validity
// says (`none_repr` refuses otherwise), so the validity is `T`'s once that repr is spent; an
// `Option` of a value that may cross threads may too.
#[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
const unsafe impl<T: [const] Atom> Atom for Option<T> {
    type Repr = T::Repr;
    type Validity = <T::Validity as Validity>::Optional;
    const MIN_REPR: u128 = if T::MIN_REPR > 0 { none_repr::<T>() } else { T::MIN_REPR };
    const MAX_REPR: u128 = if T::MIN_REPR > 0 { T::MAX_REPR } else { none_repr::<T>() };
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
