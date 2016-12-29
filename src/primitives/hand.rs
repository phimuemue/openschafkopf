use primitives::card::*;
use std::fmt;
use arrayvec::ArrayVec;

pub type SHandVector = ArrayVec<[SCard; 8]>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SHand {
    m_veccard: SHandVector,
}

impl SHand {
    pub fn new_from_hand(&self, card_played: SCard) -> SHand {
        SHand {
            m_veccard : self
                .m_veccard
                .iter()
                .cloned()
                .filter(|&card| card!=card_played)
                .collect()
        }
    }
    pub fn new_from_vec(veccard: SHandVector) -> SHand {
        SHand {m_veccard : veccard}
    }
    pub fn contains(&self, card_check: SCard) -> bool {
        self.contains_pred(|&card| card==card_check)
    }
    pub fn contains_pred<Pred>(&self, pred: Pred) -> bool
        where Pred: Fn(&SCard) -> bool
    {
        self.m_veccard
            .iter()
            .any(pred)
    }
    pub fn play_card(&mut self, card_played: SCard) {
        self.m_veccard.retain(|&mut card| card!=card_played)
    }

    pub fn cards(&self) -> &SHandVector {
        &self.m_veccard
    }

    pub fn cards_mut(&mut self) -> &mut SHandVector {
        &mut self.m_veccard
    }
}

impl fmt::Display for SHand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for card in self.m_veccard.iter() {
            write!(f, "{}, ", card)?;
        }
        write!(f, "")
    }
}

#[test]
fn test_hand() {
    let hand = SHand::new_from_vec(
        vec![
            SCard::new(EFarbe::Eichel, ESchlag::Unter),
            SCard::new(EFarbe::Herz, ESchlag::Koenig),
            SCard::new(EFarbe::Schelln, ESchlag::S7),
        ].into_iter().collect()
    );
    let hand2 = hand.new_from_hand(SCard::new(EFarbe::Herz, ESchlag::Koenig));
    assert_eq!(hand.cards().len()-1, hand2.cards().len());
    assert!(hand2.cards()[0]==SCard::new(EFarbe::Eichel, ESchlag::Unter));
    assert!(hand2.cards()[1]==SCard::new(EFarbe::Schelln, ESchlag::S7));
}
