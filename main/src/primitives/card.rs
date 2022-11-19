use crate::util::*;
use std::{
    fmt,
};
use serde::{Serializer};

plain_enum_mod!(modefarbe, EFarbe {
    Eichel,
    Gras,
    Herz,
    Schelln,
});

define_static_value!(pub SStaticFarbeEichel, EFarbe, EFarbe::Eichel);
define_static_value!(pub SStaticFarbeGras, EFarbe, EFarbe::Gras);
define_static_value!(pub SStaticFarbeHerz, EFarbe, EFarbe::Herz);
define_static_value!(pub SStaticFarbeSchelln, EFarbe, EFarbe::Schelln);

impl fmt::Display for EFarbe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            Self::Eichel => "Eichel",
            Self::Gras => "Gras",
            Self::Herz => "Herz",
            Self::Schelln => "Schelln",
        } )
    }
}

plain_enum_mod!(modeschlag, ESchlag {
    Ass,
    Zehn,
    Koenig,
    Ober,
    Unter,
    S9,
    S8,
    S7,
});

define_static_value!(pub SStaticSchlagOber, ESchlag, ESchlag::Ober);
define_static_value!(pub SStaticSchlagUnter, ESchlag, ESchlag::Unter);

impl fmt::Display for ESchlag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

plain_enum_mod!(modekurzlang, EKurzLang {
    Kurz,
    Lang,
});

impl EKurzLang {
    pub fn cards_per_player(self) -> usize {
        match self {
            Self::Kurz => 6,
            Self::Lang => 8,
        }
    }

    pub fn from_cards_per_player(n_cards_per_player: usize) -> Option<EKurzLang> {
        match n_cards_per_player {
            6 => Some(EKurzLang::Kurz),
            8 => Some(EKurzLang::Lang),
            _ => None,
        }
    }

    pub fn supports_card(self, card: SCard) -> bool {
        match self {
            Self::Lang => true,
            Self::Kurz => card.schlag()!=ESchlag::S7 && card.schlag()!=ESchlag::S8,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub enum SCard {
    EA, EZ, EK, EO, EU, E9, E8, E7,
    GA, GZ, GK, GO, GU, G9, G8, G7,
    HA, HZ, HK, HO, HU, H9, H8, H7,
    SA, SZ, SK, SO, SU, S9, S8, S7,
}

impl serde::Serialize for SCard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for SCard {
    fn deserialize<D>(deserializer: D) -> Result<SCard, D::Error>
        where
            D: serde::Deserializer<'de>,
    {
        crate::util::parser::parse_trimmed(
            &String::deserialize(deserializer)?,
            "card",
            crate::primitives::cardvector::card_parser(),
        ).map_err(serde::de::Error::custom)
    }
}

#[test]
fn test_serialization() {
    macro_rules! test_card(($($card:ident)*) => {
        $(
            let card = card_values::$card;
            serde_test::assert_tokens(&card, &[
                serde_test::Token::Str(stringify!($card)),
            ]);
        )*
    });
    test_card!(
        E7 E8 E9 EZ EU EO EK EA
        G7 G8 G9 GZ GU GO GK GA
        H7 H8 H9 HZ HU HO HK HA
        S7 S8 S9 SZ SU SO SK SA
    );
}

impl fmt::Debug for SCard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for SCard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", 
            match self.farbe() {
                EFarbe::Eichel => "E",
                EFarbe::Gras => "G",
                EFarbe::Herz => "H",
                EFarbe::Schelln => "S",
            },
            match self.schlag() {
                ESchlag::S7 => "7",
                ESchlag::S8 => "8",
                ESchlag::S9 => "9",
                ESchlag::Zehn => "Z",
                ESchlag::Unter => "U",
                ESchlag::Ober => "O",
                ESchlag::Koenig => "K",
                ESchlag::Ass => "A",
            }
        )
    }
}

const fn card_new_const(efarbe: EFarbe, eschlag: ESchlag) -> SCard { // TODO (plain_enum: to_usize should be const fn)
    unsafe {
        std::mem::transmute(efarbe as u8 * (ESchlag::SIZE as u8) + eschlag as u8)
    }
}

impl SCard {
    pub fn new(efarbe : EFarbe, eschlag : ESchlag) -> SCard {
        card_new_const(efarbe, eschlag)
    }
    pub fn farbe(self) -> EFarbe {
        unsafe{ EFarbe::from_usize(self.to_usize() / ESchlag::SIZE) }
    }
    pub fn schlag(self) -> ESchlag {
        unsafe{ ESchlag::from_usize(self.to_usize() % ESchlag::SIZE) }
    }
    pub fn values(ekurzlang: EKurzLang) -> impl Iterator<Item=SCard> {
        use itertools::iproduct;
        iproduct!(
            EFarbe::values(),
            ESchlag::values()
        )
        .filter_map(move |(efarbe, eschlag)| {
            match ekurzlang { // prefer matching on custom enums over simple if/else
                EKurzLang::Kurz => if ESchlag::S7==eschlag || ESchlag::S8==eschlag {
                    return None;
                },
                EKurzLang::Lang => (),
            }
            Some(SCard::new(efarbe, eschlag))
        })
    }
}

#[test]
fn test_farbe_schlag_enumerators() {
    assert_eq!(EFarbe::values().count(), 4);
    assert_eq!(ESchlag::values().count(), 8);
}

#[test]
fn test_card_ctor() {
    for efarbe in EFarbe::values() {
        for eschlag in ESchlag::values() {
            assert_eq!(SCard::new(efarbe, eschlag).farbe(), efarbe);
            assert_eq!(SCard::new(efarbe, eschlag).schlag(), eschlag);
        }
    }
}

impl PlainEnum for SCard {
    const SIZE : usize = EFarbe::SIZE*ESchlag::SIZE;
    type EnumMapArray<T> = [T; SCard::SIZE];
    unsafe fn from_usize(n: usize) -> Self {
        debug_assert!(n < Self::SIZE);
        std::mem::transmute(n.as_num::<u8>())
    }
    fn to_usize(self) -> usize {
        (self as u8).as_num::<usize>()
    }
}

pub mod card_values {
    use crate::card::*;
    macro_rules! impl_card_val_internal {(($($card:ident,)*), ($($eschlag:ident,)*), $efarbe:ident) => {
        $(
            #[allow(dead_code)]
            pub const $card : SCard = card_new_const(EFarbe::$efarbe, ESchlag::$eschlag);
        )*
    }}
    macro_rules! impl_card_val {(($($card:ident,)*), $efarbe:ident) => {
        impl_card_val_internal!(($($card,)*), (S7, S8, S9, Zehn, Unter, Ober, Koenig, Ass,), $efarbe);
    }}
    impl_card_val!((E7, E8, E9, EZ, EU, EO, EK, EA,), Eichel);
    impl_card_val!((G7, G8, G9, GZ, GU, GO, GK, GA,), Gras);
    impl_card_val!((H7, H8, H9, HZ, HU, HO, HK, HA,), Herz);
    impl_card_val!((S7, S8, S9, SZ, SU, SO, SK, SA,), Schelln);
}
