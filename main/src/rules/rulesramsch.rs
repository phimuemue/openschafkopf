use crate::primitives::*;
use crate::rules::{card_points::*, payoutdecider::internal_payout, trumpfdecider::*, *};
use crate::util::*;
use std::{cmp::Ordering, fmt};

#[derive(Clone, Debug)]
pub enum VDurchmarsch {
    None,
    All,
    AtLeast(isize),
}

#[derive(Clone, Debug)]
pub enum VJungfrau {
    DoubleAll,
    DoubleIndividuallyOnce,
    DoubleIndividuallyMultiple,
}

#[derive(Clone, Debug)]
pub struct SRulesRamsch {
    n_price : isize,
    durchmarsch : VDurchmarsch,
    trumpfdecider: STrumpfDeciderRamsch,
    ojungfrau: Option<VJungfrau>,
}

impl SRulesRamsch {
    pub fn new(n_price: isize, durchmarsch: VDurchmarsch, ojungfrau: Option<VJungfrau>) -> Self {
        Self {
            n_price,
            durchmarsch,
            trumpfdecider: STrumpfDeciderRamsch::default(),
            ojungfrau,
        }
    }
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
        let mapepipointstichcount = &rulestatecache.changing.mapepipointstichcount;
        let points_for_player = |epi| mapepipointstichcount[epi].n_point;
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
        let vecepi_most_points = EPlayerIndex::values().max_set_by_key(|epi| points_for_player(*epi));
        let n_points_max = points_for_player(vecepi_most_points[0]); // TODO use all_equal_item
        let the_one_epi = || -> EPlayerIndex {
            assert!(n_points_max>=61);
            *unwrap!(vecepi_most_points.iter().exactly_one())
        };
        if match self.durchmarsch {
            VDurchmarsch::All if 120==n_points_max =>
                debug_verify_eq!(
                    mapepipointstichcount[the_one_epi()].n_stich==gamefinishedstiche.get().kurzlang().cards_per_player(),
                    gamefinishedstiche.get().completed_stichs_winner_index(self).all(|(_stich, epi_winner)| epi_winner==the_one_epi())
                ),
            VDurchmarsch::All | VDurchmarsch::None =>
                false,
            VDurchmarsch::AtLeast(n_points_durchmarsch) => {
                assert!(n_points_durchmarsch>=61); // otherwise, it may not be clear who is the durchmarsch winner
                n_points_max>=n_points_durchmarsch
            },
        } {
            // TODO? Jungfrau meaningful?
            internal_payout(
                self.n_price,
                &SPlayerParties13::new(the_one_epi()),
                /*b_primary_party_wins*/true,
            ).map(|n_payout| payout_including_stoss_doubling(*n_payout, tpln_stoss_doubling))
        } else {
            let epi_loser : EPlayerIndex = {
                vecepi_most_points.iter().copied().exactly_one().unwrap_or_else(|_err| {
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
                })
            };
            let count_jungfrau_occurences = || {
                mapepipointstichcount.iter()
                    .filter(|pointstichcount| pointstichcount.n_stich==0)
                    .count()
                    .as_num::<u32>()
            };
            match self.ojungfrau {
                None => {
                    internal_payout(
                        self.n_price,
                        &SPlayerParties13::new(epi_loser),
                        /*b_primary_party_wins*/false,
                    ).map(|n_payout| payout_including_stoss_doubling(*n_payout, tpln_stoss_doubling))
                },
                Some(VJungfrau::DoubleAll) => {
                    internal_payout(
                        self.n_price * 2isize.pow(count_jungfrau_occurences()),
                        &SPlayerParties13::new(epi_loser),
                        /*b_primary_party_wins*/false,
                    ).map(|n_payout| payout_including_stoss_doubling(*n_payout, tpln_stoss_doubling))
                },
                Some(VJungfrau::DoubleIndividuallyOnce) => {
                    let mut an_payout = EPlayerIndex::map_from_fn(|_epi| 0);
                    for epi in EPlayerIndex::values() {
                        if epi != epi_loser {
                            assert_eq!(an_payout[epi], 0);
                            an_payout[epi] = self.n_price;
                            if mapepipointstichcount[epi].n_stich==0 {
                                an_payout[epi] *= 2;
                            }
                            an_payout[epi_loser] -= an_payout[epi];
                        }
                    }
                    an_payout.map(|n_payout| payout_including_stoss_doubling(*n_payout, tpln_stoss_doubling))
                },
                Some(VJungfrau::DoubleIndividuallyMultiple) => {
                    let mut an_payout = EPlayerIndex::map_from_fn(|_epi| 0);
                    let n_jungfrau_occurences = count_jungfrau_occurences();
                    for epi in EPlayerIndex::values() {
                        if epi != epi_loser {
                            assert_eq!(an_payout[epi], 0);
                            an_payout[epi] = self.n_price;
                            if mapepipointstichcount[epi].n_stich==0 {
                                assert!(1<=n_jungfrau_occurences);
                                an_payout[epi] *= 2isize.pow(n_jungfrau_occurences);
                            }
                            an_payout[epi_loser] -= an_payout[epi];
                        }
                    }
                    an_payout.map(|n_payout| payout_including_stoss_doubling(*n_payout, tpln_stoss_doubling))
                },
            }
        }
    }

    fn payouthints(&self, _stichseq: &SStichSequence, _ahand: &EnumMap<EPlayerIndex, SHand>, _tpln_stoss_doubling: (usize, usize), _n_stock: isize, _rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        // TODO sensible payouthints
        EPlayerIndex::map_from_fn(|_epi| SInterval::from_raw([None, None]))
    }

    fn equivalent_when_on_same_hand(&self) -> SEnumChains<SCard> {
        use crate::primitives::card_values::*;
        debug_verify_eq!(
            SEnumChains::new_from_slices(&[
                &[EO, GO, HO, SO] as &[SCard],
                &[EU, GU, HU, SU],
                &[H9, H8, H7],
                &[E9, E8, E7],
                &[G9, G8, G7],
                &[S9, S8, S7],
            ]),
            {
                let (mapefarbeveccard, veccard_trumpf) = self.trumpfdecider.equivalent_when_on_same_hand();
                let vecveccard = mapefarbeveccard.into_raw().into_iter().chain(Some(veccard_trumpf).into_iter())
                    .flat_map(|veccard| payoutdecider::equivalent_when_on_same_hand_point_based(&veccard))
                    .collect::<Vec<_>>();
                SEnumChains::new_from_slices(
                    &vecveccard.iter()
                        .map(|veccard| veccard as &[SCard]).collect::<Vec<_>>(),
                )
            }
        )
    }
}
