use card::*;
use stich::*;
use hand::*;
use std::cmp::Ordering;
use std::fmt;

pub type CHandVector = Vec<CCard>; // TODO: vector with fixed capacity 8

#[derive(PartialEq)]
pub enum VTrumpfOrFarbe {
    Trumpf,
    Farbe (EFarbe),
}

pub fn equivalent_when_on_same_hand_default (_: CCard, _: CCard, _: &Vec<CStich>) -> bool {
    unimplemented!();
    // TODO implement default version
    false
}

pub trait TRules : fmt::Display {

    fn trumpf_or_farbe(&self, card: CCard) -> VTrumpfOrFarbe;

    fn can_be_played(&self, _hand: &CHand) -> bool {
        true // probably, only Rufspiel is prevented in some cases
    }

    fn is_trumpf(&self, card: CCard) -> bool {
        VTrumpfOrFarbe::Trumpf == self.trumpf_or_farbe(card)
    }

    fn points_card(&self, card: CCard) -> isize {
        // by default, we assume that we use the usual points
        match card.schlag() {
            eschlag7 | eschlag8 | eschlag9 => 0,
            eschlagU => 2,
            eschlagO => 3,
            eschlagK => 4,
            eschlagZ => 10,
            eschlagA => 11,
        }
    }
    fn points_stich(&self, stich: &CStich) -> isize {
        stich.indices_and_cards()
            .fold(0, |n_sum, (_, card)| n_sum + self.points_card(card))
    }
    fn points_per_player(&self, vecstich: &Vec<CStich>) -> [isize; 4] {
        let mut an_points = [0, 0, 0, 0,];
        for stich in vecstich {
            let mut n_points_stich = 0;
            for (_, card) in stich.indices_and_cards() {
                n_points_stich = n_points_stich + self.points_card(card);
            }
            an_points[self.winner_index(stich)] = an_points[self.winner_index(stich)] + n_points_stich;
        }
        an_points
    }

    fn is_winner(&self, eplayerindex: EPlayerIndex, vecstich: &Vec<CStich>) -> bool;

    fn is_prematurely_winner(&self, vecstich: &Vec<CStich>) -> [bool; 4];

    fn truempfe_in_order(&self, veceschlag : Vec<ESchlag>, efarbe_trumpf: EFarbe) -> Vec<CCard> {
        let n_trumpf_expected = 4 * veceschlag.len() + 8 - veceschlag.len();
        assert!(0<n_trumpf_expected);
        let mut veccard = Vec::with_capacity(n_trumpf_expected);
        for eschlag in veceschlag.iter() {
            for efarbe in EFarbe::all_values().iter() {
                veccard.push(CCard::new(*efarbe, *eschlag));
            }
        }
        for eschlag in ESchlag::all_values().iter() {
            if !veceschlag.iter().any(|eschlag_trumpf| *eschlag_trumpf==*eschlag) {
                veccard.push(CCard::new(efarbe_trumpf, *eschlag));
            }
        }
        assert_eq!(n_trumpf_expected, veccard.len());
        veccard
    }

    fn payout(&self, vecstich: &Vec<CStich>) -> [isize; 4];

    // impls of equivalent_when_on_same_hand may use equivalent_when_on_same_hand_default
    fn equivalent_when_on_same_hand(&self, card1: CCard, card2: CCard, vecstich: &Vec<CStich>) -> bool {
        equivalent_when_on_same_hand_default(card1, card2, vecstich)
    }

    fn all_allowed_cards(&self, vecstich: &Vec<CStich>, hand: &CHand) -> CHandVector {
        if vecstich.last().unwrap().empty() {
            self.all_allowed_cards_first_in_stich(vecstich, hand)
        } else {
            self.all_allowed_cards_within_stich(vecstich, hand)
        }
    }

    fn all_allowed_cards_first_in_stich(&self, vecstich: &Vec<CStich>, hand: &CHand) -> CHandVector;

    fn all_allowed_cards_within_stich(&self, vecstich: &Vec<CStich>, hand: &CHand) -> CHandVector;

    fn better_card(&self, card_fst: CCard, card_snd: CCard) -> CCard {
        if Ordering::Less==self.compare_in_stich(card_fst, card_snd) {
            card_snd
        } else {
            card_fst
        }
    }

    fn compare_less_equivalence(&self, card_fst: CCard, card_snd: CCard) -> bool {
        if card_fst.schlag()==card_snd.schlag() {
            card_fst.farbe() < card_snd.farbe()
        } else {
            let n_points_fst = self.points_card(card_fst);
            let n_points_snd = self.points_card(card_snd);
            if n_points_fst==n_points_snd {
                card_fst.farbe() < card_snd.farbe()
                    || card_fst.farbe() == card_snd.farbe() && card_fst.schlag() < card_snd.schlag()
            } else {
                n_points_fst < n_points_snd
            }
        }
    }

    fn card_is_allowed(&self, vecstich: &Vec<CStich>, hand: &CHand, card: CCard) -> bool {
        self.all_allowed_cards(vecstich, hand).into_iter()
            .any(|card_iterated| card_iterated==card)
    }

    //fn DefaultStrategy() -> std::shared_ptr<CStrategy>;
    
    fn winner_index(&self, stich: &CStich) -> EPlayerIndex {
        let mut eplayerindex_best = stich.m_eplayerindex_first;
        for i in 1..stich.size() {
            let eplayerindex_current = (stich.m_eplayerindex_first + i)%4;
            if Ordering::Less==self.compare_in_stich(stich.m_acard[eplayerindex_best], stich.m_acard[eplayerindex_current]) {
                eplayerindex_best = eplayerindex_current;
            }
        }
        eplayerindex_best
    }
    fn best_card_in_stich(&self, stich: &CStich) -> CCard {
        return stich.m_acard[self.winner_index(stich) as usize];
    }

    fn compare_in_stich_trumpf(&self, card_fst: CCard, card_snd: CCard) -> Ordering;

    fn compare_in_stich_farbe(&self, card_fst: CCard, card_snd: CCard) -> Ordering {
        if card_fst.farbe() != card_snd.farbe() {
            Ordering::Greater
        } else {
            compare_farbcards_same_color(card_fst, card_snd)
        }
    }

    fn compare_in_stich(&self, card_fst: CCard, card_snd: CCard) -> Ordering {
        assert!(card_fst!=card_snd);
        match (self.is_trumpf(card_fst), self.is_trumpf(card_snd)) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (true, true) => self.compare_in_stich_trumpf(card_fst, card_snd),
            (false, false) => self.compare_in_stich_farbe(card_fst, card_snd),
        }
    }
}

pub fn compare_farbcards_same_color(card_fst: CCard, card_snd: CCard) -> Ordering {
    let get_schlag_value = |card: CCard| { match card.schlag() {
        eschlag7 => 0,
        eschlag8 => 1,
        eschlag9 => 2,
        eschlagU => 3,
        eschlagO => 4,
        eschlagK => 5,
        eschlagZ => 6,
        eschlagA => 7,
    } };
    if get_schlag_value(card_fst) < get_schlag_value(card_snd) {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

pub fn compare_trumpfcards_solo(card_fst: CCard, card_snd: CCard) -> Ordering {
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
