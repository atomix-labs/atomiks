//! What a test checks of a value's `Atom` impl, built-in or derived: its repr and validity, the
//! bits it takes as a field, and which reprs decode.

use core::fmt::Debug;

use atomiks_core::validity::Validity;
use atomiks_core::{Atom, Primitive, ReprRange};

/// Compiles only where `T`'s repr is `R` and its validity `V`.
pub(crate) const fn repr_and_validity_are<T: Atom<Repr = R, Validity = V>, R, V: Validity>() {}

/// How many bits a packed value takes for a field of `reprs`.
pub(crate) fn field_width<R: Primitive>(reprs: ReprRange<R>) -> u32 {
    reprs.field_layout().width()
}

/// Checks that `T` decodes each of `reprs` as the one of `values` whose repr it is, alike
/// unchecked, and refuses it where none's is.
pub(crate) fn decodes_exactly<T, R, const COUNT: usize>(values: [T; COUNT], reprs: R)
where
    T: Atom + PartialEq + Debug,
    R: IntoIterator<Item = T::Repr>,
    T::Repr: PartialEq + Debug,
{
    let encoded = values.map(Atom::to_repr);
    for repr in reprs {
        let expected = encoded.iter().position(|&each| each == repr).map(|index| values[index]);
        assert_eq!(T::from_repr(repr), expected, "{repr:?} decodes as the value it is, alone");
        assert_eq!(decoded_unchecked::<T>(repr), expected, "{repr:?} decodes alike unchecked");
    }
}

/// The value `repr` decodes as without the check, where it decodes with it; else `None`.
fn decoded_unchecked<T: Atom>(repr: T::Repr) -> Option<T> {
    T::from_repr(repr)?;
    // SAFETY: `repr` decodes, as `from_repr` just showed.
    #[expect(unsafe_code, reason = "the unchecked decode under test")]
    let value = unsafe { T::from_repr_unchecked(repr) };
    Some(value)
}

/// Gives `$callback` every byte's name, `B000` to `B255`, in order, so a test declares an enum of
/// a variant for each.
macro_rules! with_every_byte {
    ($callback:ident) => {
        $callback! {
            B000 B001 B002 B003 B004 B005 B006 B007 B008 B009 B010 B011 B012 B013 B014 B015
            B016 B017 B018 B019 B020 B021 B022 B023 B024 B025 B026 B027 B028 B029 B030 B031
            B032 B033 B034 B035 B036 B037 B038 B039 B040 B041 B042 B043 B044 B045 B046 B047
            B048 B049 B050 B051 B052 B053 B054 B055 B056 B057 B058 B059 B060 B061 B062 B063
            B064 B065 B066 B067 B068 B069 B070 B071 B072 B073 B074 B075 B076 B077 B078 B079
            B080 B081 B082 B083 B084 B085 B086 B087 B088 B089 B090 B091 B092 B093 B094 B095
            B096 B097 B098 B099 B100 B101 B102 B103 B104 B105 B106 B107 B108 B109 B110 B111
            B112 B113 B114 B115 B116 B117 B118 B119 B120 B121 B122 B123 B124 B125 B126 B127
            B128 B129 B130 B131 B132 B133 B134 B135 B136 B137 B138 B139 B140 B141 B142 B143
            B144 B145 B146 B147 B148 B149 B150 B151 B152 B153 B154 B155 B156 B157 B158 B159
            B160 B161 B162 B163 B164 B165 B166 B167 B168 B169 B170 B171 B172 B173 B174 B175
            B176 B177 B178 B179 B180 B181 B182 B183 B184 B185 B186 B187 B188 B189 B190 B191
            B192 B193 B194 B195 B196 B197 B198 B199 B200 B201 B202 B203 B204 B205 B206 B207
            B208 B209 B210 B211 B212 B213 B214 B215 B216 B217 B218 B219 B220 B221 B222 B223
            B224 B225 B226 B227 B228 B229 B230 B231 B232 B233 B234 B235 B236 B237 B238 B239
            B240 B241 B242 B243 B244 B245 B246 B247 B248 B249 B250 B251 B252 B253 B254 B255
        }
    };
}

pub(crate) use with_every_byte;
