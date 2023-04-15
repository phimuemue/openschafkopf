use crate::primitives::*;
use crate::util::*;
use std::{
    borrow::Borrow,
    fmt,
    ops::Index,
};

pub type SStich = SPlayersInRound<ECard, EPlayerIndex>;

#[derive(Copy, Clone, Debug)]
pub struct SFullStich<Stich>(Stich);

impl<Stich: Borrow<SStich>> SFullStich<Stich> {
    pub fn new(stich: Stich) -> Self {
        debug_assert!(stich.borrow().is_full());
        Self(stich)
    }
    pub fn get(&self) -> &SStich {
        self.0.borrow()
    }
    pub fn as_ref(&self) -> SFullStich<&SStich> {
        SFullStich::new(self.get())
    }
    pub fn iter(&self) -> SPlayersInRoundIterator<std::slice::Iter<ECard>> {
        self.get().iter()
    }
}
impl<Stich: Borrow<SStich>> Index<EPlayerIndex> for SFullStich<Stich> {
    type Output = ECard;
    fn index(&self, epi : EPlayerIndex) -> &ECard {
        unwrap!(self.get().get(epi))
    }
}
impl<Stich: Borrow<SStich>> Borrow<SStich> for SFullStich<Stich> {
    fn borrow(&self) -> &SStich {
        self.get()
    }
}

impl fmt::Display for SStich {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for epi in EPlayerIndex::values() {
            if epi==self.epi_first {
                write!(f, ">")?;
            } else {
                write!(f, " ")?;
            }
            match self.get(epi) {
                None => {write!(f, "__")?;}
                Some(card) => {write!(f, "{}", card)?;}
            }
        }
        write!(f, "")
    }
}

#[test]
fn test_stich() {
    // TODO? use quicktest or similar
    {
        use crate::card::ECard::*;
        let acard = [E7, E8, E9, EK];
        for epi_first in EPlayerIndex::values() {
            for n_size in 0..5 {
                let mut stich = SStich::new(epi_first);
                for &card in acard.iter().take(n_size) {
                    stich.push(card);
                }
                assert_eq!(stich.size(), n_size);
                assert_eq!(stich.first_playerindex(), epi_first);
                assert_eq!(stich.size(), stich.iter().count());
                for (epi, card) in stich.iter() {
                    assert_eq!(stich.get(epi), Some(card));
                    assert_eq!(stich.get(epi), Some(card));
                }
            }
        }
    }
    {
        let mut stich = SStich::new(EPlayerIndex::EPI2);
        stich.push(ECard::new(EFarbe::Eichel, ESchlag::Unter));
        stich.push(ECard::new(EFarbe::Gras, ESchlag::S7));
        assert!(stich.get(EPlayerIndex::EPI2)==Some(&ECard::new(EFarbe::Eichel, ESchlag::Unter)));
        assert!(stich.get(EPlayerIndex::EPI3)==Some(&ECard::new(EFarbe::Gras, ESchlag::S7)));
        assert_eq!(stich.iter().count(), 2);
    }
}
