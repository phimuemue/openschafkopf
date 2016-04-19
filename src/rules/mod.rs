pub mod rulesrufspiel;
pub mod rulessolo;
pub mod ruleset;

use card::*;
use stich::*;
use hand::*;
use std::cmp::Ordering;
use std::fmt;

#[derive(PartialEq, Eq, Hash)]
pub enum VTrumpfOrFarbe {
    Trumpf,
    Farbe (EFarbe),
}

pub trait TRules : fmt::Display {

    fn trumpf_or_farbe(&self, card: CCard) -> VTrumpfOrFarbe;

    fn playerindex(&self) -> Option<EPlayerIndex>;

    fn can_be_played(&self, _hand: &CHand) -> bool {
        true // probably, only Rufspiel is prevented in some cases
    }

    fn is_trumpf(&self, card: CCard) -> bool {
        VTrumpfOrFarbe::Trumpf == self.trumpf_or_farbe(card)
    }

    fn points_card(&self, card: CCard) -> isize {
        // by default, we assume that we use the usual points
        match card.schlag() {
            ESchlag::S7 | ESchlag::S8 | ESchlag::S9 => 0,
            ESchlag::Unter => 2,
            ESchlag::Ober => 3,
            ESchlag::Koenig => 4,
            ESchlag::Zehn => 10,
            ESchlag::Ass => 11,
        }
    }
    fn points_stich(&self, stich: &SStich) -> isize {
        stich.indices_and_cards()
            .fold(0, |n_sum, (_, card)| n_sum + self.points_card(card))
    }
    fn points_per_player(&self, vecstich: &Vec<SStich>) -> [isize; 4] {
        let mut an_points = [0, 0, 0, 0,];
        for stich in vecstich {
            an_points[self.winner_index(stich)] += stich.indices_and_cards()
                .fold(
                    0,
                    |n_acc, (_eplayerindex, card)| n_acc + self.points_card(card)
                );
        }
        an_points
    }

    fn is_winner(&self, eplayerindex: EPlayerIndex, vecstich: &Vec<SStich>) -> bool;

    fn count_laufende(&self, vecstich: &Vec<SStich>, veceschlag : Vec<ESchlag>, efarbe_trumpf: EFarbe) -> isize {
        let n_trumpf_expected = 4 * veceschlag.len() + 8 - veceschlag.len();
        assert!(0<n_trumpf_expected);
        let mut veccard_trumpf = Vec::with_capacity(n_trumpf_expected);
        for eschlag in veceschlag.iter() {
            for efarbe in EFarbe::all_values().iter() {
                veccard_trumpf.push(CCard::new(*efarbe, *eschlag));
            }
        }
        for eschlag in ESchlag::all_values().iter() {
            if !veceschlag.iter().any(|eschlag_trumpf| *eschlag_trumpf==*eschlag) {
                veccard_trumpf.push(CCard::new(efarbe_trumpf, *eschlag));
            }
        }
        assert_eq!(n_trumpf_expected, veccard_trumpf.len());
        let mapcardeplayerindex = SCardMap::<EPlayerIndex>::new_from_pairs(
            vecstich.iter().flat_map(|stich| stich.indices_and_cards())
        );
        let laufende_relevant = |card: &CCard| {
            self.is_winner(mapcardeplayerindex[*card], vecstich)
        };
        let b_might_have_lauf = laufende_relevant(veccard_trumpf.first().unwrap());
        veccard_trumpf.iter()
            .take_while(|card| b_might_have_lauf==laufende_relevant(card))
            .count() as isize
    }

    fn payout(&self, vecstich: &Vec<SStich>) -> [isize; 4];

    fn all_allowed_cards(&self, vecstich: &Vec<SStich>, hand: &CHand) -> CHandVector {
        assert!(!vecstich.is_empty());
        assert!(vecstich.last().unwrap().size()<4);
        if vecstich.last().unwrap().empty() {
            self.all_allowed_cards_first_in_stich(vecstich, hand)
        } else {
            self.all_allowed_cards_within_stich(vecstich, hand)
        }
    }

    fn all_allowed_cards_first_in_stich(&self, vecstich: &Vec<SStich>, hand: &CHand) -> CHandVector;

    fn all_allowed_cards_within_stich(&self, vecstich: &Vec<SStich>, hand: &CHand) -> CHandVector;

    fn better_card(&self, card_fst: CCard, card_snd: CCard) -> CCard {
        if Ordering::Less==self.compare_in_stich(card_fst, card_snd) {
            card_snd
        } else {
            card_fst
        }
    }

    fn card_is_allowed(&self, vecstich: &Vec<SStich>, hand: &CHand, card: CCard) -> bool {
        self.all_allowed_cards(vecstich, hand).into_iter()
            .any(|card_iterated| card_iterated==card)
    }

    fn winner_index(&self, stich: &SStich) -> EPlayerIndex {
        let mut eplayerindex_best = stich.m_eplayerindex_first;
        for (eplayerindex, card) in stich.indices_and_cards().skip(1) {
            if Ordering::Less==self.compare_in_stich(stich.m_acard[eplayerindex_best], card) {
                eplayerindex_best = eplayerindex;
            }
        }
        eplayerindex_best
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
        ESchlag::S7 => 0,
        ESchlag::S8 => 1,
        ESchlag::S9 => 2,
        ESchlag::Unter => 3,
        ESchlag::Ober => 4,
        ESchlag::Koenig => 5,
        ESchlag::Zehn => 6,
        ESchlag::Ass => 7,
    } };
    if get_schlag_value(card_fst) < get_schlag_value(card_snd) {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

pub fn compare_trumpfcards_solo(card_fst: CCard, card_snd: CCard) -> Ordering {
    match (card_fst.schlag(), card_snd.schlag()) {
        (ESchlag::Ober, ESchlag::Ober) | (ESchlag::Unter, ESchlag::Unter) => {
            assert!(card_fst.schlag()==ESchlag::Ober || card_fst.schlag()==ESchlag::Unter);
            // TODO static_assert not available in rust, right?
            assert!(EFarbe::Eichel < EFarbe::Gras, "Farb-Sorting can't be used here");
            assert!(EFarbe::Gras < EFarbe::Herz, "Farb-Sorting can't be used here");
            assert!(EFarbe::Herz < EFarbe::Schelln, "Farb-Sorting can't be used here");
            if card_snd.farbe() < card_fst.farbe() {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        (ESchlag::Ober, _) => Ordering::Greater,
        (_, ESchlag::Ober) => Ordering::Less,
        (ESchlag::Unter, _) => Ordering::Greater,
        (_, ESchlag::Unter) => Ordering::Less,
        _ => compare_farbcards_same_color(card_fst, card_snd),
    }
}
