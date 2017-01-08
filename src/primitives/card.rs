extern crate quickcheck;

use std::fmt;
use std::mem;
use std::ops::{Index, IndexMut};
pub use util::plain_enum::*;
use util::*;

plain_enum_mod!{modefarbe, EFarbe {
    Eichel,
    Gras,
    Herz,
    Schelln,
}}

impl fmt::Display for EFarbe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            &EFarbe::Eichel => "Eichel",
            &EFarbe::Gras => "Gras",
            &EFarbe::Herz => "Herz",
            &EFarbe::Schelln => "Schelln",
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

impl fmt::Display for ESchlag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct SCard {
    m_n_internalrepresentation : i8,
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

impl SCard {
    pub fn new(efarbe : EFarbe, eschlag : ESchlag) -> SCard {
        SCard{m_n_internalrepresentation : (efarbe.to_usize() * 8 + eschlag.to_usize()).as_num()}
    }
    pub fn farbe(&self) -> EFarbe {
        EFarbe::from_usize((self.m_n_internalrepresentation / 8).as_num())
    }
    pub fn schlag(&self) -> ESchlag {
        ESchlag::from_usize((self.m_n_internalrepresentation % 8).as_num())
    }
    pub fn values() -> Vec<SCard> { // TODO Rust: return iterator once we can specify that return type is an iterator
        iproduct!(
            EFarbe::values(),
            ESchlag::values()
        )
        .map(|(efarbe, eschlag)| SCard::new(efarbe, eschlag))
        .collect()
    }
}

impl quickcheck::Arbitrary for SCard {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> SCard {
        SCard::new(EFarbe::arbitrary(g), ESchlag::arbitrary(g))
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

pub struct SCardMap<T> {
    m_at : [T; 32],
}

impl <T> SCardMap<T> {
    pub fn new_from_pairs<'card, ItPair>(itpairtcard : ItPair) -> SCardMap<T>
        where ItPair : Iterator<Item=(T, &'card SCard)>
    {
        let mut mapcardt : SCardMap<T>;
        unsafe {
            mapcardt = mem::uninitialized();
        }
        for (t, card) in itpairtcard {
            mapcardt[*card] = t;
        }
        mapcardt
    }
}

impl <T> Index<SCard> for SCardMap<T> {
    type Output = T;

    fn index(&self, card: SCard) -> &T {
        &self.m_at[card.m_n_internalrepresentation.as_num::<usize>()]
    }
}

impl <T> IndexMut<SCard> for SCardMap<T> {
    fn index_mut(&mut self, card: SCard) -> &mut Self::Output {
        &mut self.m_at[card.m_n_internalrepresentation.as_num::<usize>()]
    }
}
