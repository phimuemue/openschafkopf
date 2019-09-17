use std::fmt;
use crate::util::*;

plain_enum_mod!{modefarbe, EFarbe {
    Eichel,
    Gras,
    Herz,
    Schelln,
}}

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

plain_enum_mod!{modeschlag, ESchlag {
    Ass,
    Zehn,
    Koenig,
    Ober,
    Unter,
    S9,
    S8,
    S7,
}}

define_static_value!(pub SStaticSchlagOber, ESchlag, ESchlag::Ober);
define_static_value!(pub SStaticSchlagUnter, ESchlag, ESchlag::Unter);

impl fmt::Display for ESchlag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

plain_enum_mod!{modekurzlang, EKurzLang {
    Kurz,
    Lang,
}}

impl EKurzLang {
    pub fn cards_per_player(self) -> usize {
        match self {
            Self::Kurz => 6,
            Self::Lang => 8,
        }
    }

    fn internal_from_cards_per_player<R, FnOk: FnOnce(EKurzLang)->R, FnErr: FnOnce()->R>(
        n_cards_per_player: usize,
        fn_ok: FnOk,
        fn_err: FnErr,
    ) -> R {
        match n_cards_per_player {
            6 => fn_ok(EKurzLang::Kurz),
            8 => fn_ok(EKurzLang::Lang),
            _ => fn_err(),
        }
    }

    pub fn from_cards_per_player(n_cards_per_player: usize) -> EKurzLang {
        Self::internal_from_cards_per_player(
            n_cards_per_player,
            |ekurzlang| ekurzlang,
            || panic!("Cannot convert {} to EKurzLang.", n_cards_per_player),
        )
    }

    pub fn checked_from_cards_per_player(n_cards_per_player: usize) -> Option<EKurzLang> {
        Self::internal_from_cards_per_player(n_cards_per_player, Some, || None)
    }

    pub fn supports_card(self, card: SCard) -> bool {
        match self {
            Self::Lang => true,
            Self::Kurz => card.schlag()!=ESchlag::S7 && card.schlag()!=ESchlag::S8,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub struct SCard {
    n_internalrepresentation : u8,
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
    SCard{n_internalrepresentation : (efarbe as usize * ESchlag::SIZE + eschlag as usize) as u8}
}

impl SCard {
    pub fn new(efarbe : EFarbe, eschlag : ESchlag) -> SCard {
        let card = SCard{n_internalrepresentation : (efarbe.to_usize() * ESchlag::SIZE + eschlag.to_usize()).as_num()};
        assert_eq!(card, card_new_const(efarbe, eschlag));
        card
    }
    pub fn farbe(self) -> EFarbe {
        EFarbe::from_usize(self.n_internalrepresentation.as_num::<usize>() / ESchlag::SIZE)
    }
    pub fn schlag(self) -> ESchlag {
        ESchlag::from_usize(self.n_internalrepresentation.as_num::<usize>() % ESchlag::SIZE)
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

impl TPlainEnum for SCard {
    const SIZE : usize = EFarbe::SIZE*ESchlag::SIZE;
    fn from_usize(n: usize) -> Self {
        debug_assert!(n < Self::SIZE);
        SCard{n_internalrepresentation: n.as_num::<u8>()}
    }
    fn to_usize(self) -> usize {
        self.n_internalrepresentation.as_num::<usize>()
    }
}
impl<V> TInternalEnumMapType<V> for SCard {
    type InternalEnumMapType = [V; SCard::SIZE];
}

#[cfg(test)]
pub mod card_values {
    use crate::card::*;
    macro_rules! impl_card_val_internal {(($($card:ident,)*), ($($eschlag:ident,)*), $efarbe:ident) => {
        $(pub const $card : SCard = card_new_const(EFarbe::$efarbe, ESchlag::$eschlag);)*
    }}
    macro_rules! impl_card_val {(($($card:ident,)*), $efarbe:ident) => {
        impl_card_val_internal!(($($card,)*), (S7, S8, S9, Zehn, Unter, Ober, Koenig, Ass,), $efarbe);
    }}
    impl_card_val!((E7, E8, E9, EZ, EU, EO, EK, EA,), Eichel);
    impl_card_val!((G7, G8, G9, GZ, GU, GO, GK, GA,), Gras);
    impl_card_val!((H7, H8, H9, HZ, HU, HO, HK, HA,), Herz);
    impl_card_val!((S7, S8, S9, SZ, SU, SO, SK, SA,), Schelln);
}
