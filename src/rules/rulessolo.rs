use card::*;
use stich::*;
use hand::*;
use rules::*;
use std::fmt;
use std::cmp::Ordering;

pub struct CRulesSolo {
    pub m_eplayerindex : EPlayerIndex,
    pub m_efarbe : EFarbe,
}

impl fmt::Display for CRulesSolo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-Solo", self.m_efarbe)
    }
}

impl TRules for CRulesSolo {
    fn trumpf_or_farbe(&self, card: SCard) -> VTrumpfOrFarbe {
        if card.schlag()==ESchlag::Ober || card.schlag()==ESchlag::Unter || card.farbe()==self.m_efarbe {
            VTrumpfOrFarbe::Trumpf
        } else {
            VTrumpfOrFarbe::Farbe(card.farbe())
        }
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.m_eplayerindex)
    }

    fn is_winner(&self, eplayerindex: EPlayerIndex, vecstich: &Vec<SStich>) -> bool {
        assert!(8==vecstich.len());
        if eplayerindex==self.m_eplayerindex {
            self.points_per_player(vecstich)[self.m_eplayerindex]>=61
        } else {
            self.points_per_player(vecstich)[self.m_eplayerindex]<=60
        }
    }

    fn payout(&self, vecstich: &Vec<SStich>) -> [isize; 4] {
        let n_laufende = self.count_laufende(vecstich, vec!(ESchlag::Ober, ESchlag::Unter), self.m_efarbe);
        create_playerindexmap(|eplayerindex| {
            (/*n_payout_solo*/ 50
             + {if n_laufende<3 {0} else {n_laufende}} * 10
            ) * {
                if self.is_winner(eplayerindex, vecstich) {
                    1
                } else {
                    -1
                }
            } * {
                if self.m_eplayerindex==eplayerindex {
                    3
                } else {
                    1
                }
            }
        } )
    }

    fn all_allowed_cards_first_in_stich(&self, _vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
        hand.cards().clone()
    }

    fn all_allowed_cards_within_stich(&self, vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        let card_first = vecstich.last().unwrap().first_card();
        let veccard_allowed : SHandVector = hand.cards().iter()
            .filter(|&&card| self.trumpf_or_farbe(card)==self.trumpf_or_farbe(card_first))
            .cloned()
            .collect();
        if veccard_allowed.is_empty() {
            hand.cards().clone()
        } else {
            veccard_allowed
        }
    }

    fn compare_in_stich_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        compare_trumpfcards_solo(card_fst, card_snd)
    }
}
