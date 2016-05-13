use card::*;
use stich::*;
use hand::*;
use rules::*;
use std::fmt;
use std::cmp::Ordering;

pub trait TActiveSinglePlayCore {
    fn is_trumpf(&self, card: SCard) -> bool;
    fn count_laufende(&self, vecstich: &Vec<SStich>, ab_winner: &[bool; 4]) -> isize;
    fn compare_trumpfcards_solo(card_fst: SCard, card_snd: SCard) -> Ordering;
    fn to_string(&self) -> String;
}

pub struct SRulesActiveSinglePlay<ActiveSinglePlayCore> 
    where ActiveSinglePlayCore: TActiveSinglePlayCore,
{
    pub m_eplayerindex : EPlayerIndex,
    pub m_core : ActiveSinglePlayCore,
}

impl<ActiveSinglePlayCore> fmt::Display for SRulesActiveSinglePlay<ActiveSinglePlayCore> 
    where ActiveSinglePlayCore: TActiveSinglePlayCore,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.m_core.to_string())
    }
}

pub struct SCoreSolo {
    pub m_efarbe: EFarbe,
}

impl TActiveSinglePlayCore for SCoreSolo {
    fn is_trumpf(&self, card: SCard) -> bool {
        card.schlag()==ESchlag::Ober || card.schlag()==ESchlag::Unter || card.farbe()==self.m_efarbe 
    }

    fn count_laufende(&self, vecstich: &Vec<SStich>, ab_winner: &[bool; 4]) -> isize {
        count_laufende(vecstich, vec!(ESchlag::Ober, ESchlag::Unter), self.m_efarbe, &ab_winner)
    }

    fn compare_trumpfcards_solo(card_fst: SCard, card_snd: SCard) -> Ordering {
        compare_trumpfcards_solo(card_fst, card_snd)
    }

    fn to_string(&self) -> String {
        format!("{}-Solo", self.m_efarbe)
    }
}

impl<ActiveSinglePlayCore> TRules for SRulesActiveSinglePlay<ActiveSinglePlayCore> 
    where ActiveSinglePlayCore: TActiveSinglePlayCore,
{
    fn trumpf_or_farbe(&self, card: SCard) -> VTrumpfOrFarbe {
        if self.m_core.is_trumpf(card) {
            VTrumpfOrFarbe::Trumpf
        } else {
            VTrumpfOrFarbe::Farbe(card.farbe())
        }
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.m_eplayerindex)
    }

    fn is_winner(&self, eplayerindex: EPlayerIndex, vecstich: &Vec<SStich>) -> bool {
        assert!(8==vecstich.len());
        if eplayerindex==self.m_eplayerindex {
            self.points_per_player(vecstich)[self.m_eplayerindex]>=61
        } else {
            self.points_per_player(vecstich)[self.m_eplayerindex]<=60
        }
    }

    fn payout(&self, vecstich: &Vec<SStich>) -> [isize; 4] {
        let ab_winner = create_playerindexmap(|eplayerindex| self.is_winner(eplayerindex, vecstich));
        let n_laufende = self.m_core.count_laufende(vecstich, &ab_winner);
        create_playerindexmap(|eplayerindex| {
            (/*n_payout_solo*/ 50
             + {if n_laufende<3 {0} else {n_laufende}} * 10
            ) * {
                if ab_winner[eplayerindex] {
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
            }
        } )
    }

    fn all_allowed_cards_first_in_stich(&self, _vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
        hand.cards().clone()
    }

    fn all_allowed_cards_within_stich(&self, vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        let card_first = vecstich.last().unwrap().first_card();
        let veccard_allowed : SHandVector = hand.cards().iter()
            .filter(|&&card| self.trumpf_or_farbe(card)==self.trumpf_or_farbe(card_first))
            .cloned()
            .collect();
        if veccard_allowed.is_empty() {
            hand.cards().clone()
        } else {
            veccard_allowed
        }
    }

    fn compare_in_stich_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        ActiveSinglePlayCore::compare_trumpfcards_solo(card_fst, card_snd)
    }
}

