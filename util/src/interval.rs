use plain_enum::*;

plain_enum_mod!(modelohi, ELoHi {Lo, Hi,});

pub type SInterval<T> = EnumMap<ELoHi, T>;

impl std::ops::Neg for ELoHi {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Self::Lo => Self::Hi,
            Self::Hi => Self::Lo,
        }
    }
}
