extern crate quickcheck;

use std::fmt;
use std::mem;
use std::ops::{Index, IndexMut};

pub use self::EFarbe::*;
#[derive(PartialEq, Eq, Debug, Copy, Clone, PartialOrd, Ord, Hash)]
pub enum EFarbe {
    Eichel,
    Gras,
    Herz,
    Schelln,
}

impl EFarbe {
    pub fn all_values() -> [EFarbe; 4] {
        [EFarbe::Eichel, EFarbe::Gras, EFarbe::Herz, EFarbe::Schelln,]
    }
}

impl quickcheck::Arbitrary for EFarbe {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> EFarbe {
        *EFarbe::all_values().iter()
            .nth(
                g.gen_range(
                    0,
                    EFarbe::all_values().iter().count()
                )
            ).unwrap()
    }
}

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

pub use self::ESchlag::*;
#[derive(PartialEq, Eq, Debug, Copy, Clone, PartialOrd, Ord)]
pub enum ESchlag {
    Ass,
    Zehn,
    Koenig,
    Ober,
    Unter,
    S9,
    S8,
    S7,
}

impl ESchlag {
    pub fn all_values() -> [ESchlag; 8] {
        [ ESchlag::Ass, ESchlag::Zehn, ESchlag::Koenig, ESchlag::Ober, ESchlag::Unter, ESchlag::S9, ESchlag::S8, ESchlag::S7, ]
    }
}

impl quickcheck::Arbitrary for ESchlag {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> ESchlag {
        *ESchlag::all_values().iter()
            .nth(
                g.gen_range(
                    0,
                    ESchlag::all_values().iter().count()
                )
            ).unwrap()
    }
}

impl fmt::Display for ESchlag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct CCard {
    m_n_internalrepresentation : i8,
}

impl fmt::Display for CCard {
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

impl CCard {
    pub fn new(efarbe : EFarbe, eschlag : ESchlag) -> CCard {
        CCard{m_n_internalrepresentation : efarbe as i8 * 8 + eschlag as i8}
    }
    pub fn farbe(&self) -> EFarbe {
        unsafe{(mem::transmute(self.m_n_internalrepresentation / 8))}
    }
    pub fn schlag(&self) -> ESchlag {
        unsafe{(mem::transmute(self.m_n_internalrepresentation % 8))}
    }
    // TODO: inspect if those are really needed and remove if necessary
    // fn image_filename(&self) -> String {
    //     return format!("../img/{}.png", self)
    // }
    // fn image_size() -> (i32, i32) {
    //     (114, 201)
    // }
}

impl quickcheck::Arbitrary for CCard {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> CCard {
        CCard::new(EFarbe::arbitrary(g), ESchlag::arbitrary(g))
    }
}

#[test]
fn test_farbe_schlag_enumerators() {
    assert_eq!(EFarbe::all_values().iter().count(), 4);
    assert_eq!(ESchlag::all_values().iter().count(), 8);
}

#[test]
fn test_card_ctor() {
    for &efarbe in EFarbe::all_values().iter() {
        for &eschlag in ESchlag::all_values().iter() {
            assert_eq!(CCard::new(efarbe, eschlag).farbe(), efarbe);
            assert_eq!(CCard::new(efarbe, eschlag).schlag(), eschlag);
        }
    }
}

pub struct SCardMap<T> {
    m_at : [T; 32],
}

impl <T> SCardMap<T> {
    pub fn new_from_pairs<ItPair>(itpairtcard : ItPair) -> SCardMap<T>
        where ItPair : Iterator<Item=(T, CCard)>
    {
        let mut mapcardt : SCardMap<T>;
        unsafe {
            mapcardt = mem::uninitialized();
        }
        for (t, card) in itpairtcard {
            mapcardt[card] = t;
        }
        mapcardt
    }
}

impl <T> Index<CCard> for SCardMap<T> {
    type Output = T;

    fn index(&self, card: CCard) -> &T {
        &self.m_at[card.m_n_internalrepresentation as usize]
    }
}

impl <T> IndexMut<CCard> for SCardMap<T> {
    fn index_mut(&mut self, card: CCard) -> &mut Self::Output {
        &mut self.m_at[card.m_n_internalrepresentation as usize]
    }
}
