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
    pub const fn cards_per_player(self) -> usize {
        match self {
            Self::Kurz => 6,
            Self::Lang => 8,
        }
    }

    pub const fn max_cards_per_player() -> usize {
        8 // TODO could we compute this via EKurzLang::values().map(cards_per_player)?
    }

    pub fn from_cards_per_player(n_cards_per_player: usize) -> Option<EKurzLang> {
        match n_cards_per_player {
            6 => Some(EKurzLang::Kurz),
            8 => Some(EKurzLang::Lang),
            _ => None,
        }
    }

    pub fn supports_card(self, card: ECard) -> bool {
        match self {
            Self::Lang => true,
            Self::Kurz => card.schlag()!=ESchlag::S7 && card.schlag()!=ESchlag::S8,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub enum ECard {
    EA, EZ, EK, EO, EU, E9, E8, E7,
    GA, GZ, GK, GO, GU, G9, G8, G7,
    HA, HZ, HK, HO, HU, H9, H8, H7,
    SA, SZ, SK, SO, SU, S9, S8, S7,
}

impl serde::Serialize for ECard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for ECard {
    fn deserialize<D>(deserializer: D) -> Result<ECard, D::Error>
        where
            D: serde::Deserializer<'de>,
    {
        crate::util::parser::parse_trimmed(
            &String::deserialize(deserializer)?,
            crate::primitives::cardvector::card_parser(),
        ).map_err(serde::de::Error::custom)
    }
}

#[test]
fn test_serialization() {
    macro_rules! test_card(($($card:ident)*) => {
        $(
            let card = ECard::$card;
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

impl fmt::Debug for ECard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for ECard {
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

impl ECard {
    pub const fn new(efarbe : EFarbe, eschlag : ESchlag) -> ECard {
        unsafe {
            std::mem::transmute(efarbe as u8 * (ESchlag::SIZE as u8) + eschlag as u8)
        }
    }
    pub const fn farbe(self) -> EFarbe {
        unsafe{ std::mem::transmute(self as usize / ESchlag::SIZE) } // TODO(plain_enum) from_usize/to_usize const
    }
    pub const fn schlag(self) -> ESchlag {
        unsafe{ std::mem::transmute(self as usize % ESchlag::SIZE) } // TODO(plain_enum) from_usize/to_usize const
    }
    pub fn values(ekurzlang: EKurzLang) -> impl Iterator<Item=ECard>+Clone {
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
            Some(ECard::new(efarbe, eschlag))
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
    macro_rules! explicit_test{($($efarbe:ident, $eschlag:ident, $card:ident)+) => {{
        $({
            const CARD : ECard = ECard::new(EFarbe::$efarbe, ESchlag::$eschlag);
            assert_eq!(CARD, ECard::$card);
            const EFARBE : EFarbe = CARD.farbe();
            assert_eq!(EFARBE, EFarbe::$efarbe);
            const ESCHLAG : ESchlag = CARD.schlag();
            assert_eq!(ESCHLAG, ESchlag::$eschlag);
        })+
        $({
            let card = ECard::new(EFarbe::$efarbe, ESchlag::$eschlag);
            assert_eq!(card, ECard::$card);
            assert_eq!(card.farbe(), EFarbe::$efarbe);
            assert_eq!(card.schlag(), ESchlag::$eschlag);
        })+
    }}}
    explicit_test!(
        Eichel, S7, E7
        Eichel, S8, E8
        Eichel, S9, E9
        Eichel, Zehn, EZ
        Eichel, Unter, EU
        Eichel, Ober, EO
        Eichel, Koenig, EK
        Eichel, Ass, EA
        Gras, S7, G7
        Gras, S8, G8
        Gras, S9, G9
        Gras, Zehn, GZ
        Gras, Unter, GU
        Gras, Ober, GO
        Gras, Koenig, GK
        Gras, Ass, GA
        Herz, S7, H7
        Herz, S8, H8
        Herz, S9, H9
        Herz, Zehn, HZ
        Herz, Unter, HU
        Herz, Ober, HO
        Herz, Koenig, HK
        Herz, Ass, HA
        Schelln, S7, S7
        Schelln, S8, S8
        Schelln, S9, S9
        Schelln, Zehn, SZ
        Schelln, Unter, SU
        Schelln, Ober, SO
        Schelln, Koenig, SK
        Schelln, Ass, SA
    )
}

unsafe impl PlainEnum for ECard {
    const SIZE : usize = EFarbe::SIZE*ESchlag::SIZE;
    type EnumMapArray<T> = [T; ECard::SIZE];
    unsafe fn from_usize(n: usize) -> Self {
        debug_assert!(n < Self::SIZE);
        std::mem::transmute(n.as_num::<u8>())
    }
    fn to_usize(self) -> usize {
        (self as u8).as_num::<usize>()
    }
}
