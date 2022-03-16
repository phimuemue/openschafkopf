use crate::primitives::*;
use crate::rules::{card_points::*, payoutdecider::{internal_payout, equivalent_when_on_same_hand_point_based}, trumpfdecider::*, *};
use crate::util::*;
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

    fn payout_no_invariant(&self, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), _n_stock: isize, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, isize> {
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
                        .reduce(|tplepiocard_fst, tplepiocard_snd| {
                            match (tplepiocard_fst.1, tplepiocard_snd.1) {
                                (Some(card_trumpf_fst), Some(card_trumpf_snd)) => {
                                    if Ordering::Less==unwrap!(self.compare_cards(card_trumpf_fst, card_trumpf_snd)) {
                                        tplepiocard_snd
                                    } else {
                                        tplepiocard_fst
                                    }
                                },
                                (Some(_), None) => tplepiocard_fst,
                                (None, Some(_)) => tplepiocard_snd,
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
        ).map(|n_payout| payout_including_stoss_doubling(*n_payout, tpln_stoss_doubling))
    }

    fn payouthints(&self, _stichseq: &SStichSequence, _ahand: &EnumMap<EPlayerIndex, SHand>, _tpln_stoss_doubling: (usize, usize), _n_stock: isize, _rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        // TODO sensible payouthints
        EPlayerIndex::map_from_fn(|_epi| SInterval::from_raw([None, None]))
    }

    fn equivalent_when_on_same_hand(&self) -> Option<SEnumChains<SCard>> {
        use crate::primitives::card_values::*;
        debug_verify_eq!(
            Some(SEnumChains::new_from_slices(&[
                &[EO, GO, HO, SO] as &[SCard],
                &[EU, GU, HU, SU],
                &[H9, H8, H7],
                &[E9, E8, E7],
                &[G9, G8, G7],
                &[S9, S8, S7],
            ])),
            {
                let (mapefarbeveccard, veccard_trumpf) = STrumpfDeciderRamsch::equivalent_when_on_same_hand();
                let vecveccard = mapefarbeveccard.into_raw().into_iter().chain(Some(veccard_trumpf).into_iter())
                    .flat_map(|veccard| equivalent_when_on_same_hand_point_based(&veccard))
                    .collect::<Vec<_>>();
                Some(SEnumChains::new_from_slices(
                    &vecveccard.iter()
                        .map(|veccard| &veccard as &[SCard]).collect::<Vec<_>>(),
                ))
            }
        )
    }
}
