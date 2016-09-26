pub mod rulesrufspiel;
pub mod rulessolo;
pub mod rulesramsch;
pub mod ruleset;
pub mod trumpfdecider;
pub mod wrappers;

#[cfg(test)]
mod tests;

use primitives::*;
use std::cmp::Ordering;
use std::fmt;
pub use rules::wrappers::*;

#[derive(PartialEq, Eq, Hash)]
pub enum VTrumpfOrFarbe {
    Trumpf,
    Farbe (EFarbe),
}

pub enum ESchneiderSchwarz {
    Nothing,
    Schneider,
    Schwarz,
}

pub struct SStoss {
    pub m_eplayerindex : EPlayerIndex,
}

pub fn points_to_schneiderschwarz_and_winners<FnIsPlayerParty, Rules>(
    vecstich: &Vec<SStich>,
    rules: &Rules,
    fn_is_player_party: FnIsPlayerParty,
) -> (ESchneiderSchwarz, [bool; 4])
    where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
          Rules: TRules,
{
    assert_eq!(vecstich.len(), 8);
    let n_points_player_party : isize = vecstich.iter()
        .filter(|stich| fn_is_player_party(rules.winner_index(stich)))
        .map(|stich| rules.points_stich(stich))
        .sum();
    let b_player_party_wins = n_points_player_party>=61;
    (
        if b_player_party_wins {
            if vecstich.iter().all(|stich| fn_is_player_party(rules.winner_index(stich))) {
                ESchneiderSchwarz::Schwarz
            } else if n_points_player_party>90 {
                ESchneiderSchwarz::Schneider
            } else {
                ESchneiderSchwarz::Nothing
            }
        } else {
            if vecstich.iter().all(|stich| !fn_is_player_party(rules.winner_index(stich))) {
                ESchneiderSchwarz::Schwarz
            } else if n_points_player_party<=30 {
                ESchneiderSchwarz::Schneider
            } else {
                ESchneiderSchwarz::Nothing
            }
        },
        create_playerindexmap(|eplayerindex| {
            fn_is_player_party(eplayerindex)==b_player_party_wins
        })
    )
}

pub trait TRules : fmt::Display + TAsRules {

    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe;

    fn playerindex(&self) -> Option<EPlayerIndex>;

    fn can_be_played(&self, _hand: &SFullHand) -> bool {
        true // probably, only Rufspiel is prevented in some cases
    }

    fn stoss_allowed(&self, eplayerindex: EPlayerIndex, vecstoss: &Vec<SStoss>, hand: &SHand) -> bool;

    fn is_trumpf(&self, card: SCard) -> bool {
        VTrumpfOrFarbe::Trumpf == self.trumpforfarbe(card)
    }

    fn points_card(&self, card: SCard) -> isize {
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
        stich.iter()
            .map(|(_, card)| self.points_card(*card))
            .sum()
    }
    fn points_per_player(&self, vecstich: &Vec<SStich>, eplayerindex: EPlayerIndex) -> isize {
        vecstich.iter()
            .filter(|stich| eplayerindex==self.winner_index(stich))
            .map(|stich| self.points_stich(stich))
            .sum()
    }

    fn payout(&self, gamefinishedstiche: &SGameFinishedStiche) -> [isize; 4];

    fn all_allowed_cards(&self, vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        assert!(vecstich.last().unwrap().size()<4);
        if 0==vecstich.last().unwrap().size() {
            self.all_allowed_cards_first_in_stich(vecstich, hand)
        } else {
            self.all_allowed_cards_within_stich(vecstich, hand)
        }
    }

    fn all_allowed_cards_first_in_stich(&self, vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector;

    fn all_allowed_cards_within_stich(&self, vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector;

    fn card_is_allowed(&self, vecstich: &Vec<SStich>, hand: &SHand, card: SCard) -> bool {
        self.all_allowed_cards(vecstich, hand).into_iter()
            .any(|card_iterated| card_iterated==card)
    }

    fn winner_index(&self, stich: &SStich) -> EPlayerIndex {
        let mut eplayerindex_best = stich.m_eplayerindex_first;
        for (eplayerindex, card) in stich.iter().skip(1) {
            if Ordering::Less==self.compare_in_stich(stich[eplayerindex_best], *card) {
                eplayerindex_best = eplayerindex;
            }
        }
        eplayerindex_best
    }

    fn compare_in_stich_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering;

    fn compare_in_stich_farbe(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        if card_fst.farbe() != card_snd.farbe() {
            Ordering::Greater
        } else {
            compare_farbcards_same_color(card_fst, card_snd)
        }
    }

    fn compare_in_stich(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        assert!(card_fst!=card_snd);
        match (self.is_trumpf(card_fst), self.is_trumpf(card_snd)) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (true, true) => self.compare_in_stich_trumpf(card_fst, card_snd),
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

#[derive(PartialEq, Eq)]
pub enum VGameAnnouncementPriority {
    RufspielLike,
    SinglePlayLike(isize), // lower index means higher priority
}

impl PartialOrd for VGameAnnouncementPriority {
    fn partial_cmp(&self, priority: &VGameAnnouncementPriority) -> Option<Ordering> {
        Some(match (self, priority) {
            (&VGameAnnouncementPriority::RufspielLike, _) => Ordering::Less,
            (_, &VGameAnnouncementPriority::RufspielLike) => Ordering::Greater,
            (&VGameAnnouncementPriority::SinglePlayLike(n_prio_lhs), &VGameAnnouncementPriority::SinglePlayLike(n_prio_rhs)) => n_prio_rhs.cmp(&n_prio_lhs), // Note lower index means higher priority
        })
    }
}

impl Ord for VGameAnnouncementPriority {
    fn cmp(&self, priority: &VGameAnnouncementPriority) -> Ordering {
        self.partial_cmp(priority).unwrap()
    }
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
