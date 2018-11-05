use primitives::card::*;
use std::fmt;
use arrayvec::ArrayVec;

pub type SHandVector = ArrayVec<[SCard; 8]>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SHand {
    veccard: SHandVector,
}

impl SHand {
    pub fn new_from_hand(&self, card_played: SCard) -> SHand {
        SHand {
            veccard : self
                .veccard
                .iter()
                .cloned()
                .filter(|&card| card!=card_played)
                .collect()
        }
    }
    pub fn new_from_vec(veccard: SHandVector) -> SHand {
        SHand {veccard}
    }
    pub fn contains(&self, card_check: SCard) -> bool {
        self.contains_pred(|&card| card==card_check)
    }
    pub fn contains_pred(&self, pred: impl Fn(&SCard)->bool) -> bool {
        self.veccard
            .iter()
            .any(pred)
    }
    pub fn play_card(&mut self, card_played: SCard) {
        self.veccard.retain(|&mut card| card!=card_played)
    }

    pub fn cards(&self) -> &SHandVector {
        &self.veccard
    }

    pub fn cards_mut(&mut self) -> &mut SHandVector {
        &mut self.veccard
    }
}

impl fmt::Display for SHand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for card in self.veccard.iter() {
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
