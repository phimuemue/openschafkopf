use crate::util::*;

plain_enum_mod!(modelohi, ELoHi {Lo, Hi,});

pub type SInterval<T> = EnumMap<ELoHi, T>;

