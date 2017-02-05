#[macro_use]
pub mod trumpfdecider;
pub mod rulesrufspiel;
pub mod rulessolo;
pub mod rulesramsch;
pub mod ruleset;
pub mod payoutdecider;
pub mod wrappers;
mod card_points;

#[cfg(test)]
mod tests;

use primitives::*;
use std::cmp::Ordering;
use std::fmt;
pub use rules::wrappers::*;
use util::*;

pub fn current_stich(vecstich: &[SStich]) -> &SStich {
    assert!(!vecstich.is_empty());
    vecstich.last().unwrap()
}

pub fn completed_stichs(vecstich: &[SStich]) -> &[SStich] {
    assert!(current_stich(vecstich).size()<4);
    assert_eq!(vecstich[0..vecstich.len()-1].len(), vecstich.len()-1);
    assert!(vecstich[0..vecstich.len()-1].iter().all(|stich| stich.size()==4));
    &vecstich[0..vecstich.len()-1]
}

#[derive(PartialEq, Eq, Hash)]
pub enum VTrumpfOrFarbe {
    Trumpf,
    Farbe (EFarbe),
}

impl VTrumpfOrFarbe {
    pub fn is_trumpf(&self) -> bool {
        match *self {
            VTrumpfOrFarbe::Trumpf => true,
            VTrumpfOrFarbe::Farbe(_efarbe) => false,
        }
    }
}

pub struct SStoss {
    pub m_epi : EPlayerIndex,
}

pub trait TRules : fmt::Display + TAsRules + Sync {
    // TTrumpfDecider
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe;
    fn compare_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering;
    fn count_laufende(&self, gamefinishedstiche: &SGameFinishedStiche, ab_winner: &EnumMap<EPlayerIndex, bool>) -> usize;


    fn playerindex(&self) -> Option<EPlayerIndex>;

    fn can_be_played(&self, _hand: &SFullHand) -> bool {
        true // probably, only Rufspiel is prevented in some cases
    }

    fn stoss_allowed(&self, epi: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool;

    fn payout(&self, gamefinishedstiche: &SGameFinishedStiche, n_stoss: usize, n_doubling: usize, n_stock: isize) -> SAccountBalance;

    fn all_allowed_cards(&self, vecstich: &[SStich], hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        assert!(vecstich.last().unwrap().size()<4);
        if 0==vecstich.last().unwrap().size() {
            self.all_allowed_cards_first_in_stich(vecstich, hand)
        } else {
            self.all_allowed_cards_within_stich(vecstich, hand)
        }
    }

    fn all_allowed_cards_first_in_stich(&self, vecstich: &[SStich], hand: &SHand) -> SHandVector;

    fn all_allowed_cards_within_stich(&self, vecstich: &[SStich], hand: &SHand) -> SHandVector;

    fn card_is_allowed(&self, vecstich: &[SStich], hand: &SHand, card: SCard) -> bool {
        self.all_allowed_cards(vecstich, hand).into_iter()
            .any(|card_iterated| card_iterated==card)
    }

    fn winner_index(&self, stich: &SStich) -> EPlayerIndex {
        let mut epi_best = stich.m_epi_first;
        for (epi, card) in stich.iter().skip(1) {
            if Ordering::Less==self.compare_in_stich(stich[epi_best], *card) {
                epi_best = epi;
            }
        }
        epi_best
    }

    fn compare_in_stich_farbe(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        if card_fst.farbe() != card_snd.farbe() {
            Ordering::Greater
        } else {
            compare_farbcards_same_color(card_fst, card_snd)
        }
    }

    fn compare_in_stich(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        assert!(card_fst!=card_snd);
        match (self.trumpforfarbe(card_fst).is_trumpf(), self.trumpforfarbe(card_snd).is_trumpf()) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (true, true) => self.compare_trumpf(card_fst, card_snd),
            (false, false) => self.compare_in_stich_farbe(card_fst, card_snd),
        }
    }

    fn sort_cards_first_trumpf_then_farbe(&self, veccard: &mut [SCard]) {
        veccard.sort_by(|&card_lhs, &card_rhs| {
            match(self.trumpforfarbe(card_lhs), self.trumpforfarbe(card_rhs)) {
                (VTrumpfOrFarbe::Farbe(efarbe_lhs), VTrumpfOrFarbe::Farbe(efarbe_rhs)) => {
                    if efarbe_lhs==efarbe_rhs {
                        self.compare_in_stich_farbe(card_rhs, card_lhs)
                    } else {
                        efarbe_lhs.cmp(&efarbe_rhs)
                    }
                }
                (_, _) => { // at least one of them is trumpf
                    self.compare_in_stich(card_rhs, card_lhs)
                }
            }
        });
    }
}

// TODO Rust: Objects should be upcastable to supertraits
// https://github.com/rust-lang/rust/issues/5665
pub trait TAsRules {
    fn as_rules(&self) -> &TRules;
}

impl<Rules: TRules> TAsRules for Rules {
    fn as_rules(&self) -> &TRules {
        self
    }
}

#[derive(PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum VGameAnnouncementPriority {
    // state priorities in ascending order
    RufspielLike,
    SoloLikeSimple(isize),
    SoloTout(isize),
    SoloSie,
}

#[test]
fn test_gameannouncementprio() {
    use self::VGameAnnouncementPriority::*;
    assert!(RufspielLike==RufspielLike);
    assert!(RufspielLike<SoloLikeSimple(0));
    assert!(RufspielLike<SoloTout(0));
    assert!(RufspielLike<SoloSie);
    assert!(SoloLikeSimple(0)>RufspielLike);
    assert!(SoloLikeSimple(0)==SoloLikeSimple(0));
    assert!(SoloLikeSimple(0)<SoloTout(0));
    assert!(SoloLikeSimple(0)<SoloSie);
    assert!(SoloTout(0)>RufspielLike);
    assert!(SoloTout(0)>SoloLikeSimple(0));
    assert!(SoloTout(0)==SoloTout(0));
    assert!(SoloTout(0)<SoloSie);
    assert!(SoloSie>RufspielLike);
    assert!(SoloSie>SoloLikeSimple(0));
    assert!(SoloSie>SoloTout(0));
    assert!(SoloSie==SoloSie);
    assert!(SoloLikeSimple(0)<SoloLikeSimple(1));
    assert!(SoloTout(0)<SoloTout(1));
}

pub trait TActivelyPlayableRules : TRules {
    fn priority(&self) -> VGameAnnouncementPriority;
}

pub fn compare_farbcards_same_color(card_fst: SCard, card_snd: SCard) -> Ordering {
    let get_schlag_value = |card: SCard| { match card.schlag() {
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
