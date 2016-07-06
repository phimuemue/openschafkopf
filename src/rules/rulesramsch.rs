use primitives::*;
use rules::*;
use rules::trumpfdecider::*;
use std::fmt;
use std::cmp::Ordering;
use itertools::Itertools;

pub struct SRulesRamsch {}

impl fmt::Display for SRulesRamsch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ramsch")
    }
}

pub type STrumpfDeciderRamsch = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, STrumpfDeciderFarbe<
    SFarbeDesignatorHerz>>>;

impl TRules for SRulesRamsch {
    fn trumpf_or_farbe(&self, card: SCard) -> VTrumpfOrFarbe {
        STrumpfDeciderRamsch::trumpf_or_farbe(card)
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        None
    }

    fn payout(&self, vecstich: &Vec<SStich>) -> [isize; 4] {
        let an_points = create_playerindexmap(|eplayerindex| {
            self.points_per_player(vecstich, eplayerindex)
        });
        let n_points_max = an_points.iter().max().unwrap().clone();
        let veceplayerindex_most_points : Vec<EPlayerIndex> = (0..4)
            .map(|eplayerindex| eplayerindex as usize)
            .filter(|eplayerindex| n_points_max==an_points[*eplayerindex])
            .collect();
        let n_price = 10;
        let eplayerindex_loser : EPlayerIndex = {
            if 1==veceplayerindex_most_points.len() {
                veceplayerindex_most_points[0]
            } else {
                veceplayerindex_most_points.into_iter()
                    .map(|eplayerindex| {(
                        eplayerindex,
                        vecstich.iter()
                            .map(|stich| stich[eplayerindex])
                            .filter(|card| self.is_trumpf(*card))
                            // TODO: introduce max_by_cmp
                            .fold1(|card_fst, card_snd| STrumpfDeciderRamsch::better_trumpf(card_fst, card_snd))
                    )})
                    .fold1(|paireplayerindexocard_fst, paireplayerindexocard_snd| {
                        match (paireplayerindexocard_fst.1, paireplayerindexocard_snd.1) {
                            (Some(card_trumpf_fst), Some(card_trumpf_snd)) => {
                                if Ordering::Less==STrumpfDeciderRamsch::compare_trumpfcards_solo(card_trumpf_fst, card_trumpf_snd) {
                                    paireplayerindexocard_snd
                                } else {
                                    paireplayerindexocard_fst
                                }
                            },
                            (Some(_), None) => paireplayerindexocard_fst,
                            (None, Some(_)) => paireplayerindexocard_snd,
                            // If two ore more players have the maximum number of points,
                            // at least one of them must have had at least one trumpf.
                            (None, None) => panic!("Two losing players with same points, but none of them with trumpf."),
                        }
                    })
                    .unwrap()
                    .0
            }
        };
        create_playerindexmap(|eplayerindex| {
            if eplayerindex_loser==eplayerindex {
                -3 * n_price
            } else {
                n_price
            }
        })
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
        STrumpfDeciderRamsch::compare_trumpfcards_solo(card_fst, card_snd)
    }
}
