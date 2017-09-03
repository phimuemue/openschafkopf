#[macro_use]
pub mod trumpfdecider;
#[macro_use]
pub mod singleplay;
pub mod rulesrufspiel;
pub mod rulessolo;
pub mod rulesramsch;
pub mod rulesbettel;
pub mod ruleset;
pub mod payoutdecider;
pub mod wrappers;
pub mod card_points;

#[cfg(test)]
mod tests;

use primitives::*;
use std::cmp::Ordering;
use std::fmt;
pub use rules::wrappers::*;
use util::*;
use ai::rulespecific::*;

pub fn current_stich_mut(vecstich: &mut [SStich]) -> &mut SStich {
    verify!(vecstich.last_mut()).unwrap()
}

pub fn current_stich(vecstich: &[SStich]) -> &SStich {
    verify!(vecstich.last()).unwrap()
}

pub fn completed_stichs(vecstich: &[SStich]) -> &[SStich] {
    assert!(current_stich(vecstich).size()<4);
    assert_eq!(vecstich[0..vecstich.len()-1].len(), vecstich.len()-1);
    assert!(vecstich[0..vecstich.len()-1].iter().all(|stich| stich.size()==4));
    &vecstich[0..vecstich.len()-1]
}

#[derive(PartialEq, Eq, Hash, Debug)]
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
    pub epi : EPlayerIndex,
}

pub trait TRules : fmt::Display + TAsRules + Sync {
    box_clone_require!(TRules);

    // TTrumpfDecider
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe;
    fn compare_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering;
    fn count_laufende(&self, gamefinishedstiche: &SGameFinishedStiche, ab_winner: &EnumMap<EPlayerIndex, bool>) -> usize;


    fn playerindex(&self) -> Option<EPlayerIndex>;

    fn can_be_played(&self, _hand: &SFullHand) -> bool {
        true // probably, only Rufspiel is prevented in some cases
    }

    fn stoss_allowed(&self, epi: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool;

    fn payout(&self, gamefinishedstiche: &SGameFinishedStiche, tpln_stoss_doubling: (usize, usize), n_stock: isize) -> SAccountBalance;

    fn all_allowed_cards(&self, vecstich: &[SStich], hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        assert!(current_stich(vecstich).size()<4);
        if 0==current_stich(vecstich).size() {
            self.all_allowed_cards_first_in_stich(vecstich, hand)
        } else {
            self.all_allowed_cards_within_stich(vecstich, hand)
        }
    }

    fn all_allowed_cards_first_in_stich(&self, _vecstich: &[SStich], hand: &SHand) -> SHandVector {
        // probably in most cases, every card can be played
        hand.cards().clone()
    }

    fn all_allowed_cards_within_stich(&self, vecstich: &[SStich], hand: &SHand) -> SHandVector {
        // probably in most cases, only the first card of the current stich is decisive
        assert!(!vecstich.is_empty());
        let card_first = *current_stich(vecstich).first();
        let veccard_allowed : SHandVector = hand.cards().iter().cloned()
            .filter(|&card| self.trumpforfarbe(card)==self.trumpforfarbe(card_first))
            .collect();
        if veccard_allowed.is_empty() {
            hand.cards().clone()
        } else {
            veccard_allowed
        }
    }

    fn card_is_allowed(&self, vecstich: &[SStich], hand: &SHand, card: SCard) -> bool {
        self.all_allowed_cards(vecstich, hand).into_iter()
            .any(|card_iterated| card_iterated==card)
    }

    fn winner_index(&self, stich: &SStich) -> EPlayerIndex {
        let mut epi_best = stich.epi_first;
        for (epi, card) in stich.iter().skip(1) {
            if Ordering::Less==self.compare_in_stich(stich[epi_best], *card) {
                epi_best = epi;
            }
        }
        epi_best
    }

    fn compare_in_stich_same_farbe(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        assert_eq!(self.trumpforfarbe(card_fst), self.trumpforfarbe(card_snd));
        assert_eq!(card_fst.farbe(), card_snd.farbe());
        compare_farbcards_same_color(card_fst, card_snd)
    }

    fn compare_in_stich(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        assert_ne!(card_fst, card_snd);
        match (self.trumpforfarbe(card_fst).is_trumpf(), self.trumpforfarbe(card_snd).is_trumpf()) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (true, true) => self.compare_trumpf(card_fst, card_snd),
            (false, false) => {
                if card_fst.farbe() != card_snd.farbe() {
                    Ordering::Greater
                } else {
                    self.compare_in_stich_same_farbe(card_fst, card_snd)
                }
            }
        }
    }

    fn sort_cards_first_trumpf_then_farbe(&self, veccard: &mut [SCard]) {
        veccard.sort_unstable_by(|&card_lhs, &card_rhs| {
            match(self.trumpforfarbe(card_lhs), self.trumpforfarbe(card_rhs)) {
                (VTrumpfOrFarbe::Farbe(efarbe_lhs), VTrumpfOrFarbe::Farbe(efarbe_rhs)) => {
                    if efarbe_lhs==efarbe_rhs {
                        self.compare_in_stich_same_farbe(card_rhs, card_lhs)
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

    fn rulespecific_ai<'rules>(&'rules self) -> Option<Box<TRuleSpecificAI + 'rules>> {
        None
    }
}
box_clone_impl_box!(TRules);

// TODORUST Objects should be upcastable to supertraits
// https://github.com/rust-lang/rust/issues/5665
pub trait TAsRules {
    fn as_rules(&self) -> &TRules;
}

impl<Rules: TRules> TAsRules for Rules {
    fn as_rules(&self) -> &TRules {
        self
    }
}

#[derive(PartialEq, Eq, Clone, PartialOrd, Ord, Debug)]
pub enum VGameAnnouncementPriority {
    // state priorities in ascending order
    RufspielLike,
    SoloLikeSimple(isize),
    SoloLikeSteigern(isize),
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

plain_enum_mod!(modebid, EBid {
    AtLeast,
    Higher,
});

pub trait TActivelyPlayableRules : TRules {
    box_clone_require!(TActivelyPlayableRules);
    fn priority(&self) -> VGameAnnouncementPriority;
    fn with_higher_prio_than(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<Box<TActivelyPlayableRules>> {
        if match ebid {
            EBid::AtLeast => {prio<=&self.priority()},
            EBid::Higher => {prio<&self.priority()},
        } {
            Some(TActivelyPlayableRules::box_clone(self))
        } else {
            self.with_increased_prio(prio, ebid)
        }
    }
    fn with_increased_prio(&self, _prio: &VGameAnnouncementPriority, _ebid: EBid) -> Option<Box<TActivelyPlayableRules>> {
        None
    }
    fn active_playerindex(&self) -> EPlayerIndex {
        verify!(self.playerindex()).unwrap()
    }
}
box_clone_impl_box!(TActivelyPlayableRules);

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
    get_schlag_value(card_fst).cmp(&get_schlag_value(card_snd))
}
