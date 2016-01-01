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
        self.points_per_player(vecstich)[eplayerindex]>=61
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
        match (card_fst.schlag(), card_snd.schlag()) {
            (eschlagO, eschlagO) | (eschlagU, eschlagU) => {
                assert!(card_fst.schlag()==eschlagO || card_fst.schlag()==eschlagU);
                // TODO static_assert not available in rust, right?
                assert!(efarbeEICHEL < efarbeGRAS, "Farb-Sorting can't be used here");
                assert!(efarbeGRAS < efarbeHERZ, "Farb-Sorting can't be used here");
                assert!(efarbeHERZ < efarbeSCHELLN, "Farb-Sorting can't be used here");
                if card_snd.farbe() < card_fst.farbe() {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            (eschlagO, _) => Ordering::Greater,
            (_, eschlagO) => Ordering::Less,
            (eschlagU, _) => Ordering::Greater,
            (_, eschlagU) => Ordering::Less,
            _ => compare_farbcards_same_color(card_fst, card_snd),
        }
    }


    fn compare_in_stich_farbe(&self, card_fst: CCard, card_snd: CCard) -> Ordering {
        if card_fst.farbe() != card_snd.farbe() {
            Ordering::Greater
        } else {
            compare_farbcards_same_color(card_fst, card_snd)
        }
    }

}
