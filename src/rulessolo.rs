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
    fn trumpf_or_farbe(&self, card: CCard) -> VTrumpfOrFarbe {
        if card.schlag()==eschlagO || card.schlag()==eschlagU || card.farbe()==self.m_efarbe {
            VTrumpfOrFarbe::Trumpf
        } else {
            VTrumpfOrFarbe::Farbe(card.farbe())
        }
    }

    fn is_prematurely_winner(&self, vecstich: &Vec<CStich>) -> [bool; 4] {
        let an_points = self.points_per_player(vecstich);
        let mut ab_winner = [false, false, false, false,];
        for i_player in 0..4 {
            ab_winner[i_player] = an_points[i_player] >= 61;
        }
        ab_winner
    }

    fn is_winner(&self, eplayerindex: EPlayerIndex, vecstich: &Vec<CStich>) -> bool {
        assert!(8==vecstich.len());
        if eplayerindex==self.m_eplayerindex {
            self.points_per_player(vecstich)[self.m_eplayerindex]>=61
        } else {
            self.points_per_player(vecstich)[self.m_eplayerindex]<=60
        }
    }

    fn payout(&self, vecstich: &Vec<CStich>) -> [isize; 4] {
        let mut an_payout = [0, 0, 0, 0];
        for eplayerindex in 0..4 {
            an_payout[eplayerindex] = /*n_payout_solo*/ 50 * {
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
            };
        }
        for eplayerindex in 0..4 {
            println!("{} :", an_payout[eplayerindex]);
        }
        an_payout
    }

    fn all_allowed_cards_first_in_stich(&self, vecstich: &Vec<CStich>, hand: &CHand) -> CHandVector {
        hand.cards().clone()
    }

    fn all_allowed_cards_within_stich(&self, vecstich: &Vec<CStich>, hand: &CHand) -> CHandVector {
        assert!(!vecstich.is_empty());
        let card_first = vecstich.last().unwrap().first_card();
        let mut veccard_allowed = CHandVector::new();
        {
            let mut allow_card = |card| {
                veccard_allowed.push(card);
            };
            if self.is_trumpf(card_first) {
                // trumpf
                for card in hand.cards().iter()
                    .filter(|&&card| self.is_trumpf(card))
                    .cloned()
                {
                    allow_card(card);
                }
            } else {
                // other farbe
                for card in hand.cards().iter()
                    .filter(|&&card| !self.is_trumpf(card) && card.farbe()==card_first.farbe())
                    .cloned()
                {
                    allow_card(card);
                }
            }
        }
        if veccard_allowed.is_empty() {
            hand.cards().to_vec()
        } else {
            veccard_allowed
        }
    }

    fn compare_in_stich_trumpf(&self, card_fst: CCard, card_snd: CCard) -> Ordering {
        compare_trumpfcards_solo(card_fst, card_snd)
    }
}
