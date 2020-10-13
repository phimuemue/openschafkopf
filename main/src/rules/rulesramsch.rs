use crate::primitives::*;
use crate::rules::{card_points::*, payoutdecider::internal_payout, trumpfdecider::*, *};
use crate::util::*;
use itertools::Itertools;
use std::{cmp::Ordering, fmt};

#[derive(Clone, Debug)]
pub enum VDurchmarsch {
    None,
    All,
    AtLeast(isize),
}

// TODO add Jungfrau (needed for Sauspiel analysis, in particular)

#[derive(new, Clone, Debug)]
pub struct SRulesRamsch {
    n_price : isize,
    durchmarsch : VDurchmarsch,
}

impl fmt::Display for SRulesRamsch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ramsch")
    }
}

pub type STrumpfDeciderRamsch = STrumpfDeciderSchlag<
    SStaticSchlagOber, STrumpfDeciderSchlag<
    SStaticSchlagUnter,
    SStaticFarbeHerz>>;

impl TRulesNoObj for SRulesRamsch {
    impl_rules_trumpf_noobj!(STrumpfDeciderRamsch);
}

impl TRules for SRulesRamsch {
    impl_rules_trumpf!();

    fn stoss_allowed(&self, _epi: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool {
        assert!(vecstoss.is_empty());
        EKurzLang::from_cards_per_player(hand.cards().len());
        false
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        None
    }

    fn payoutinfos(&self, gamefinishedstiche: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SPayoutInfo> {
        let points_for_player = |epi| rulestatecache.changing.mapepipointstichcount[epi].n_point;
        debug_assert_eq!(
            EPlayerIndex::map_from_fn(points_for_player),
            gamefinishedstiche.get().completed_stichs_winner_index(self)
                .fold(
                    EPlayerIndex::map_from_fn(|_epi| 0),
                    mutate_return!(|an_points_accu, (stich, epi_winner)| {
                        an_points_accu[epi_winner] += points_stich(stich);
                    })
                )
        );
        let n_points_max = unwrap!(EPlayerIndex::values().map(points_for_player).max());
        let vecepi_most_points = EPlayerIndex::values()
            .filter(|epi| n_points_max==points_for_player(*epi))
            .collect::<Vec<_>>();
        let the_one_epi = || -> EPlayerIndex {
            assert!(n_points_max>=61);
            assert_eq!(1, vecepi_most_points.len());
            vecepi_most_points[0]
        };
        let (epi_single, b_epi_single_wins) = if match self.durchmarsch {
            VDurchmarsch::All if 120==n_points_max =>
                debug_verify_eq!(
                    rulestatecache.changing.mapepipointstichcount[the_one_epi()].n_stich==gamefinishedstiche.get().kurzlang().cards_per_player(),
                    gamefinishedstiche.get().completed_stichs_winner_index(self).all(|(_stich, epi_winner)| epi_winner==the_one_epi())
                ),
            VDurchmarsch::All | VDurchmarsch::None =>
                false,
            VDurchmarsch::AtLeast(n_points_durchmarsch) => {
                assert!(n_points_durchmarsch>=61); // otherwise, it may not be clear who is the durchmarsch winner
                n_points_max>=n_points_durchmarsch
            },
        } {
            (the_one_epi(), true)
        } else {
            let epi_loser : EPlayerIndex = {
                if 1==vecepi_most_points.len() {
                    vecepi_most_points[0]
                } else {
                    unwrap!(vecepi_most_points.iter().copied()
                        .map(|epi| {(
                            epi,
                            gamefinishedstiche.get().completed_stichs().iter()
                                .map(|stich| stich[epi])
                                .filter(|card| self.trumpforfarbe(*card).is_trumpf())
                                .max_by(|card_fst, card_snd| {
                                    assert!(self.trumpforfarbe(*card_fst).is_trumpf());
                                    assert!(self.trumpforfarbe(*card_snd).is_trumpf());
                                    unwrap!(self.compare_cards(*card_fst, *card_snd))
                                })
                        )})
                        .fold1(|pairepiocard_fst, pairepiocard_snd| {
                            match (pairepiocard_fst.1, pairepiocard_snd.1) {
                                (Some(card_trumpf_fst), Some(card_trumpf_snd)) => {
                                    if Ordering::Less==unwrap!(self.compare_cards(card_trumpf_fst, card_trumpf_snd)) {
                                        pairepiocard_snd
                                    } else {
                                        pairepiocard_fst
                                    }
                                },
                                (Some(_), None) => pairepiocard_fst,
                                (None, Some(_)) => pairepiocard_snd,
                                // If two ore more players have the maximum number of points,
                                // at least one of them must have had at least one trumpf.
                                (None, None) => panic!("Two losing players with same points, but none of them with trumpf."),
                            }
                        }))
                        .0
                }
            };
            (epi_loser, false)
        };
        internal_payout(
            self.n_price,
            &SPlayerParties13::new(epi_single),
            b_epi_single_wins,
        )
            .map(|n_payout| SPayoutInfo::new(*n_payout, EStockAction::Ignore))
    }

    fn payouthints(&self, _stichseq: &SStichSequence, _ahand: &EnumMap<EPlayerIndex, SHand>, _rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SPayoutHint> {
        // TODO sensible payouthints
        EPlayerIndex::map_from_fn(|_epi| SPayoutHint::new((None, None)))
    }

}
