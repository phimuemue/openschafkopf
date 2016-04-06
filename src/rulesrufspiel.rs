use card::*;
use stich::*;
use hand::*;
use rules::*;
use std::fmt;
use std::cmp::Ordering;

pub struct CRulesRufspiel {
    pub m_eplayerindex : EPlayerIndex,
    pub m_efarbe : EFarbe, // TODO possibly wrap with ENonHerzFarbe or similar
}

impl fmt::Display for CRulesRufspiel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rufspiel mit der {}-Sau von {}", self.m_efarbe, self.m_eplayerindex)
    }
}

impl CRulesRufspiel {
    fn rufsau(&self) -> CCard {
        CCard::new(self.m_efarbe, ESchlag::Ass)
    }

    fn is_ruffarbe(&self, card: CCard) -> bool {
        if !self.is_trumpf(card) {
            card.farbe()==self.m_efarbe
        } else {
            false
        }
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
    fn can_be_played(&self, hand: &CHand) -> bool {
        let it = || {hand.cards().iter().filter(|&card| self.is_ruffarbe(*card))};
        it().all(|card| card.schlag()!=ESchlag::Ass)
        && 0<it().count()
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.m_eplayerindex)
    }

    fn trumpf_or_farbe(&self, card: CCard) -> VTrumpfOrFarbe {
        if card.schlag()==ESchlag::Ober || card.schlag()==ESchlag::Unter || card.farbe()==EFarbe::Herz {
            VTrumpfOrFarbe::Trumpf
        } else {
            VTrumpfOrFarbe::Farbe(card.farbe())
        }
    }

    fn is_prematurely_winner(&self, vecstich: &Vec<CStich>) -> [bool; 4] {
        let an_points = self.points_per_player(vecstich);
        if let Some(i_mitspieler) = self.determine_mitspieler(vecstich) {
            create_playerindexmap(|eplayerindex| {
                self.check_points_to_win(eplayerindex, i_mitspieler, &an_points)
            } )
        } else {
            create_playerindexmap(|eplayerindex| {
                an_points[eplayerindex] >= 61
            } )
        }
    }

    fn is_winner(&self, eplayerindex: EPlayerIndex, vecstich: &Vec<CStich>) -> bool {
        assert!(8==vecstich.len());
        self.check_points_to_win(eplayerindex, self.determine_mitspieler(vecstich).unwrap(), &self.points_per_player(vecstich))
    }

    fn payout(&self, vecstich: &Vec<CStich>) -> [isize; 4] {
        assert_eq!(vecstich.len(), 8);
        let n_laufende = self.count_laufende(vecstich, vec!(ESchlag::Ober, ESchlag::Unter), EFarbe::Herz);
        create_playerindexmap(|eplayerindex| {
            (/*n_payout_rufspiel_default*/ 10 
             + {if n_laufende<3 {0} else {n_laufende}} * 10
            ) * {
                if self.is_winner(eplayerindex, vecstich) {
                    1
                } else {
                    -1
                }
            }
        } )
    }

    fn equivalent_when_on_same_hand(&self, card1: CCard, card2: CCard, vecstich: &Vec<CStich>) -> bool {
        if equivalent_when_on_same_hand_default(card1, card2, vecstich) { // TODO: see if TRules::equivalent_when_on_same_hand works at some point
            return true;
        }
        if !self.is_trumpf(card1) && !self.is_trumpf(card2) && self.points_card(card1) == self.points_card(card2) {
            let count_farbe = |efarbe| {
                vecstich.iter()
                    .flat_map(|stich| stich.indices_and_cards())
                    .map(|(_, card)| card)
                    .filter(|&card| match self.trumpf_or_farbe(card) {
                                VTrumpfOrFarbe::Trumpf => false,
                                VTrumpfOrFarbe::Farbe(efarbe_card) => efarbe_card==efarbe
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

    fn all_allowed_cards_first_in_stich(&self, vecstich: &Vec<CStich>, hand: &CHand) -> CHandVector {
        assert!(!vecstich.is_empty());
        if // do we already know who had the rufsau?
            !vecstich.iter()
                .take_while(|stich| 4==stich.size()) // process full stichs
                .fold(/*b_rufsau_known_initial*/false, |b_rufsau_known_before_stich, stich| {
                    if b_rufsau_known_before_stich {
                        // already known
                        true
                    } else if self.is_ruffarbe(stich.first_card()) {
                        // gesucht or weggelaufen
                        true
                    } else {
                        // We explicitly traverse all cards because it may be allowed 
                        // (by exotic rules) to schmier rufsau even if not gesucht.
                        stich.indices_and_cards().any(|(_, card)| card==self.rufsau())
                    }
                } )
        {
            // Remark: Player must have 4 cards of ruffarbe on his hand *at this point of time* (i.e. not only at the beginning!)
            if !hand.contains(self.rufsau()) 
                || 4 <= hand.cards().iter()
                    .filter(|&card| self.is_ruffarbe(*card))
                    .count()
            {
                hand.cards().to_vec()
            } else {
                hand.cards().iter()
                    .filter(|&&card| !self.is_trumpf(card) || card.farbe()!=self.m_efarbe || card.schlag()==ESchlag::Ass)
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
        if hand.cards().len()<=1 {
            hand.cards().to_vec()
        } else {
            let card_first = vecstich.last().unwrap().first_card();
            if self.is_ruffarbe(card_first) && hand.contains(self.rufsau()) {
                // special case: gesucht
                // TODO Consider the following distribution of cards:
                // 0: GA GZ GK G8 ...   <- opens first stich
                // 1, 2: ..             <- mainly irrelevant
                // 3: G7 G9 ...         <- plays with GA
                // The first two stichs are as follows:
                //      e7        ..
                //   e9   g9    ..  >g7
                //     >g8        ..
                // Is player 0 obliged to play GA? We implement it this way for now.
                vec![self.rufsau()]
            } else {
                let veccard_allowed : Vec<CCard> = hand.cards().iter()
                    .filter(|&&card| 
                        self.rufsau()!=card 
                        && self.trumpf_or_farbe(card)==self.trumpf_or_farbe(card_first)
                    )
                    .cloned()
                    .collect();
                if veccard_allowed.is_empty() {
                    hand.cards().iter().cloned().filter(|&card| self.rufsau()!=card).collect()
                } else {
                    veccard_allowed
                }
            }
        }
    }

    fn compare_in_stich_trumpf(&self, card_fst: CCard, card_snd: CCard) -> Ordering {
        compare_trumpfcards_solo(card_fst, card_snd)
    }

}
