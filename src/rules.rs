use card::*;
use stich::*;
use hand::*;
use std::cmp::Ordering;

pub type CHandVector = Vec<CCard>; // TODO: vector with fixed capacity 8

struct PlayerAndFarbe {
    m_eplayerindex : EPlayerIndex,
    m_efarbe : EFarbe
}

#[derive(PartialEq)]
pub enum ETrumpfOrFarbe {
    trumpf,
    farbe (EFarbe),
}

pub trait TRules {

    fn trumpf_or_farbe(&self, card: CCard) -> ETrumpfOrFarbe;

    fn is_trumpf(&self, card: CCard) -> bool {
        ETrumpfOrFarbe::trumpf == self.trumpf_or_farbe(card)
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
        let an_points = [0, 0, 0, 0,];
        for stich in vecstich {
            let mut n_points_stich = 0;
            for (_, card) in stich.indices_and_cards() {
                n_points_stich = n_points_stich + self.points_card(card);
            }
        }
        an_points
    }

    fn is_winner(&self, eplayerindex: EPlayerIndex, vecstich: &Vec<CStich>) -> bool {
        unimplemented!();
        false
    }

    fn is_prematurely_winner(&self, vecstich: &Vec<CStich>) -> [bool; 4] {
        unimplemented!();
        [false, false, false, false]
    }

    fn equivalent_when_on_same_hand(&self, card1: CCard, card2: CCard, vecstich: &Vec<CStich>) -> bool {
        unimplemented!();
        false
    }

    fn all_cards_of_farbe(&self, efarbe: EFarbe) -> Vec<CCard>;

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

    fn compare_in_stich(&self, card_fst: CCard, card_snd: CCard) -> Ordering;

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
        for i in 0..stich.size() {
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
}

