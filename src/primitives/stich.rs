use primitives::*;
use std::fmt;
use util::*;

pub type SStich = SPlayersInRound<SCard>;

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
        let veccard = verify!(cardvector::parse_cards::<Vec<_>>("e7 e8 e9 ek")).unwrap();
        for epi_first in EPlayerIndex::values() {
            for n_size in 0..5 {
                let mut stich = SStich::new(epi_first);
                for i_card in 0..n_size {
                    stich.push(veccard[i_card]);
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
        stich.push(SCard::new(EFarbe::Eichel, ESchlag::Unter));
        stich.push(SCard::new(EFarbe::Gras, ESchlag::S7));
        assert!(stich[EPlayerIndex::EPI2]==SCard::new(EFarbe::Eichel, ESchlag::Unter));
        assert!(stich[EPlayerIndex::EPI3]==SCard::new(EFarbe::Gras, ESchlag::S7));
        assert_eq!(stich.iter().count(), 2);
    }
}
