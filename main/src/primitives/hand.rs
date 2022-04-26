use crate::primitives::card::*;
use arrayvec::ArrayVec;
use std::fmt;
use plain_enum::TPlainEnum;

pub type SHandVector = ArrayVec<SCard, 8>;

#[derive(Clone, Debug)]
pub struct SHand {
    veccard: SHandVector,
}

impl SHand {
    #[cfg(debug_assertions)]
    fn assert_invariant(&self) {
        let mut setcardb = SCard::map_from_fn(|_card| false); // TODO enumset
        for card in self.veccard.iter() {
            assert!(!setcardb[*card]); // TODO? introduce assign::assign_other with return value bool?
            setcardb[*card] = true;
        }
    }

    pub fn new_from_vec(veccard: SHandVector) -> SHand {
        let hand = SHand {veccard};
        #[cfg(debug_assertions)]hand.assert_invariant();
        hand
    }
    pub fn new_from_iter(itcard: impl IntoIterator<Item=SCard>) -> SHand {
        Self::new_from_vec(itcard.into_iter().collect())
    }
    pub fn contains(&self, card_check: SCard) -> bool {
        self.contains_pred(|&card| card==card_check)
    }
    pub fn contains_pred(&self, pred: impl Fn(&SCard)->bool) -> bool {
        self.veccard
            .iter()
            .any(pred)
    }
    pub fn play_card(&mut self, card: SCard) {
        debug_assert!(self.contains(card));
        self.veccard.retain(|card_in_hand| *card_in_hand!=card);
        #[cfg(debug_assertions)]self.assert_invariant();
    }
    pub fn add_card(&mut self, card: SCard) {
        debug_assert!(!self.contains(card));
        self.veccard.push(card);
        #[cfg(debug_assertions)]self.assert_invariant();
    }

    pub fn cards(&self) -> &SHandVector {
        &self.veccard
    }
}

impl fmt::Display for SHand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (SDisplayCardSlice(&self.veccard)).fmt(f)
    }
}

#[test]
fn test_hand() {
    let hand = SHand::new_from_iter([
        SCard::new(EFarbe::Eichel, ESchlag::Unter),
        SCard::new(EFarbe::Herz, ESchlag::Koenig),
        SCard::new(EFarbe::Schelln, ESchlag::S7),
    ]);
    let hand2 = {
        let mut hand2 = hand.clone();
        hand2.play_card(SCard::new(EFarbe::Herz, ESchlag::Koenig));
        hand2
    };
    assert_eq!(hand.cards().len()-1, hand2.cards().len());
    assert!(hand2.cards()[0]==SCard::new(EFarbe::Eichel, ESchlag::Unter));
    assert!(hand2.cards()[1]==SCard::new(EFarbe::Schelln, ESchlag::S7));
}
