use crate::primitives::*;
use crate::util::*;
use std::fmt;

pub type SStich = SPlayersInRound<ECard, EPlayerIndex>;

#[derive(Copy, Clone)]
pub struct SFullStich<'stich>(&'stich SStich);

impl<'stich> SFullStich<'stich> {
    pub fn new(stich: &'stich SStich) -> Self {
        debug_assert!(stich.is_full());
        Self(stich)
    }
    pub fn get(&self) -> &SStich {
        self.0
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
                    assert_eq!(stich[epi], *card);
                }
            }
        }
    }
    {
        let mut stich = SStich::new(EPlayerIndex::EPI2);
        stich.push(ECard::new(EFarbe::Eichel, ESchlag::Unter));
        stich.push(ECard::new(EFarbe::Gras, ESchlag::S7));
        assert!(stich[EPlayerIndex::EPI2]==ECard::new(EFarbe::Eichel, ESchlag::Unter));
        assert!(stich[EPlayerIndex::EPI3]==ECard::new(EFarbe::Gras, ESchlag::S7));
        assert_eq!(stich.iter().count(), 2);
    }
}
