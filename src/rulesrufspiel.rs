use card::*;
use stich::*;
use hand::*;
use rules::*;
use std::cmp::Ordering;

pub struct CRulesRufspiel {
    pub m_eplayerindex : EPlayerIndex,
    pub m_efarbe : EFarbe, // TODO possibly wrap with ENonHerzFarbe or similar
}

impl CRulesRufspiel {
    fn rufsau(&self) -> CCard {
        CCard::new(self.m_efarbe, eschlagA)
    }

    fn determine_mitspieler(&self, vecstich: &Vec<CStich>) -> Option<EPlayerIndex> {
        // TODO: currently not recognizing weglaufen, right?
        vecstich.iter()
            .flat_map(|stich| stich.indices_and_cards())
            .find(|&(_, card)| card==self.rufsau())
            .map(|(i_player, _)| i_player)
    }

    fn check_points_to_win(&self, i_player: EPlayerIndex, i_mitspieler: EPlayerIndex, an_points: &[isize; 4]) -> bool {
        // TODO: this method is always fishy when I read it (passing in i_mitspieler does not seem
        // to be the best idea)
        let n_points_player_party = an_points[self.m_eplayerindex as usize] + an_points[i_mitspieler as usize];
        if i_player==self.m_eplayerindex || i_player==i_mitspieler {
            n_points_player_party >= 61
        } else {
            n_points_player_party <= 60
        }
    }
}

impl TRules for CRulesRufspiel {
    fn trumpf_or_farbe(&self, card: CCard) -> ETrumpfOrFarbe {
        if (card.schlag()==eschlagO || card.schlag()==eschlagU || card.farbe()==efarbeHERZ) {
            ETrumpfOrFarbe::trumpf
        } else {
            ETrumpfOrFarbe::farbe(card.farbe())
        }
    }

    fn is_prematurely_winner(&self, vecstich: &Vec<CStich>) -> [bool; 4] {
        let an_points = self.points_per_player(vecstich);
        let mut ab_winner = [false, false, false, false,];
        if let Some(i_mitspieler) = self.determine_mitspieler(vecstich) {
            for i_player in 0..3 {
                ab_winner[i_player] = self.check_points_to_win(i_player, i_mitspieler, &an_points);
            }
        } else {
            for i_player in 0..3 {
                ab_winner[i_player] = an_points[i_player] >= 61;
            }
        }
        ab_winner
    }

    fn is_winner(&self, eplayerindex: EPlayerIndex, vecstich: &Vec<CStich>) -> bool {
        assert!(8==vecstich.len());
        self.check_points_to_win(eplayerindex, self.determine_mitspieler(vecstich).unwrap(), &self.points_per_player(vecstich))
    }

    fn equivalent_when_on_same_hand(&self, card1: CCard, card2: CCard, vecstich: &Vec<CStich>) -> bool {
        if TRules::equivalent_when_on_same_hand(self, card1, card2, vecstich) { // TODO: does this work?!
            return true;
        }
        if !self.is_trumpf(card1) && !self.is_trumpf(card2) && self.points_card(card1) == self.points_card(card2) {
            let count_farbe = |efarbe| {
                vecstich.iter()
                    .flat_map(|stich| stich.indices_and_cards())
                    .map(|(_, card)| card)
                    .filter(|&card| match self.trumpf_or_farbe(card) {
                                ETrumpfOrFarbe::trumpf => false,
                                ETrumpfOrFarbe::farbe(efarbeCard) => efarbeCard==efarbe
                            } )
                    .count()
            };
            // if card1 and card2 are the only remaining cards of their colors and both have 0 points, they are equivalent
            if 5==count_farbe(card1.farbe()) && 5==count_farbe(card2.farbe()) {
                return true;
            }
        }
        false
    }

    fn all_cards_of_farbe(&self, efarbe: EFarbe) -> Vec<CCard> {
        assert!(efarbeHERZ!=efarbe); // Herz not a farbe in rufspiel
        ESchlag::all_values().iter()
            .filter(|&&eschlag| eschlagO!=eschlag && eschlagU!=eschlag)
            .map(|&eschlag| CCard::new(efarbe, eschlag))
            .collect::<Vec<_>>()
    }

    fn all_allowed_cards_first_in_stich(&self, vecstich: &Vec<CStich>, hand: &CHand) -> CHandVector {
        assert!(!vecstich.is_empty());
        if // do we already know who had the rufsau?
            !vecstich.iter()
                .take_while(|stich| 4==stich.size()) // process full stichs
                .fold(/*b_rufsau_known_initial*/false, |b_rufsau_known_before_stich, stich| {
                    if b_rufsau_known_before_stich {
                        // already known
                        true
                    } else if !self.is_trumpf(stich.first_card()) && stich.first_card().farbe()==self.m_efarbe {
                        // gesucht or weggelaufen
                        true
                    } else {
                        // We explicitly traverse all cards because it may be allowed 
                        // (by exotic rules) to schmier rufsau even if not gesucht.
                        stich.indices_and_cards().any(|(_, card)| card==self.rufsau())
                    }
                } )
        {
            let mut n_cards_ruffarbe = 0;
            let mut b_contains_rufsau = false;
            for &card in hand.cards() {
                if !self.is_trumpf(card) && self.m_efarbe==card.farbe() {
                    n_cards_ruffarbe = n_cards_ruffarbe + 1;
                    if eschlagA==card.schlag() {
                        b_contains_rufsau = true;
                    }
                }
            }
            // Remark: Player must have 4 cards of ruffarbe on his hand *at this point of time* (i.e. not only at the beginning!)
            if !b_contains_rufsau || 4<=n_cards_ruffarbe {
                hand.cards().to_vec()
            } else {
                hand.cards().iter()
                    .filter(|&&card| !self.is_trumpf(card) || card.farbe()!=self.m_efarbe || card.schlag()==eschlagA)
                    .cloned()
                    .collect::<CHandVector>()
            }
        }
        else {
            hand.cards().to_vec()
        }
    }

    fn all_allowed_cards_within_stich(&self, vecstich: &Vec<CStich>, hand: &CHand) -> CHandVector {
        assert!(!vecstich.is_empty());
        let card_first = vecstich.last().unwrap().first_card();
        let mut veccard_allowed = CHandVector::new();
        {
            let mut allow_card = |card| {
                veccard_allowed.push(card);
            };
            if !self.is_trumpf(card_first) && self.m_efarbe==card_first.farbe() && hand.contains(self.rufsau()) {
                // special case: gesucht
                allow_card(self.rufsau());
            } else if self.is_trumpf(card_first) {
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

    fn compare_in_stich(&self, card_fst: CCard, card_snd: CCard) -> Ordering {
        assert!(card_fst!=card_snd);
        let get_rufspiel_farbe_value = |card: CCard| {
            match card.schlag() {
                eschlag7 => 0,
                eschlag8 => 1,
                eschlag9 => 2,
                eschlagK => 3,
                eschlagZ => 4,
                eschlagA => 5,
                _ => unreachable!(),
            }
        };
        match (self.is_trumpf(card_fst), self.is_trumpf(card_snd)) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (true, true) => {
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
                    _ => if get_rufspiel_farbe_value(card_fst) < get_rufspiel_farbe_value(card_snd) {
                        Ordering::Less
                    } else {
                        Ordering::Greater
                    },
                }
            },
            (false, false) => {
                if card_fst.farbe() != card_snd.farbe() {
                    Ordering::Greater
                } else {
                    if get_rufspiel_farbe_value(card_fst) < get_rufspiel_farbe_value(card_snd) {
                        Ordering::Less
                    } else {
                        Ordering::Greater
                    }
                }
            },
        }
    }


}
