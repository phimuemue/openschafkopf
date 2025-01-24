use crate::primitives::*;
use crate::rules::{card_points::*, payoutdecider::internal_payout, trumpfdecider::*, *};
use crate::util::*;
use std::{cmp::Ordering, fmt};

// TODO Ramsch with Stichzwang

#[derive(Clone, Debug)]
pub enum VDurchmarsch {
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
    odurchmarsch : Option<VDurchmarsch>,
    trumpfdecider: STrumpfDecider,
    ojungfrau: Option<VJungfrau>,
}

impl SRulesRamsch {
    pub fn new(n_price: isize, odurchmarsch: Option<VDurchmarsch>, ojungfrau: Option<VJungfrau>) -> Self {
        Self {
            n_price,
            odurchmarsch,
            trumpfdecider: STrumpfDecider::new(&[ESchlag::Ober, ESchlag::Unter], Some(EFarbe::Herz)),
            ojungfrau,
        }
    }
    pub fn playerindex(&self) -> Option<EPlayerIndex> {
        None // Ramsch is not an actively playable. // TODO? Is EPI3 the active player?
    }
}

impl fmt::Display for SRulesRamsch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ramsch")
    }
}

impl TRules for SRulesRamsch {
    fn trumpfdecider(&self) -> &STrumpfDecider {
        &self.trumpfdecider
    }

    fn stoss_allowed(&self, stichseq: &SStichSequence, hand: &SHand, epi: EPlayerIndex, vecstoss: &[SStoss]) -> bool {
        // TODO? Use SStossParams?
        assert!(vecstoss.is_empty());
        assert_eq!(stichseq.remaining_cards_per_hand()[epi], hand.cards().len());
        false
    }

    fn payout_no_invariant(&self, stichseq: SStichSequenceGameFinished, expensifiers: &SExpensifiers, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, isize> {
        let mapepipointstichcount = &rulestatecache.changing.mapepipointstichcount;
        let points_for_player = |epi| mapepipointstichcount[epi].n_point;
        debug_assert_eq!(
            EPlayerIndex::map_from_fn(points_for_player),
            stichseq.get().completed_stichs_winner_index(self)
                .fold(
                    EPlayerIndex::map_from_fn(|_epi| 0),
                    mutate_return!(|an_points_accu, (stich, epi_winner)| {
                        an_points_accu[epi_winner] += points_stich(stich);
                    })
                )
        );
        let vecepi_most_points = EPlayerIndex::values().max_set_by_key(|epi| points_for_player(*epi));
        let n_points_max = points_for_player(vecepi_most_points[0]);
        let the_one_epi = || -> EPlayerIndex {
            assert!(n_points_max>=61);
            *unwrap!(vecepi_most_points.iter().exactly_one())
        };
        if match self.odurchmarsch {
            None => false,
            Some(VDurchmarsch::All) => {
                120==n_points_max && debug_verify_eq!(
                    mapepipointstichcount[the_one_epi()].n_stich==stichseq.get().kurzlang().cards_per_player(),
                    stichseq.get().completed_stichs_winner_index(self).all(|(_stich, epi_winner)| epi_winner==the_one_epi())
                )
            },
            Some(VDurchmarsch::AtLeast(n_points_durchmarsch)) => {
                assert!(n_points_durchmarsch>=61); // otherwise, it may not be clear who is the durchmarsch winner
                n_points_max>=n_points_durchmarsch
            },
        } {
            // TODO? Jungfrau meaningful?
            internal_payout(
                /*certain win => positive*/{assert!(self.n_price>0); self.n_price},
                &SPlayerParties13::new(the_one_epi()),
            )
        } else {
            let epi_loser : EPlayerIndex = {
                vecepi_most_points.iter().copied().exactly_one().unwrap_or_else(|_err| {
                    // TODO highest stich count loses
                    // TODO most trumpf in stichs loses
                    // TODO combination of all of the tie breakers
                    unwrap!(vecepi_most_points.iter().copied()
                        .map(|epi| {(
                            epi,
                            stichseq.get().completed_cards_by(epi)
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
            let payout_jungfrau_double_all = |n_jungfrau_exponent| {
                internal_payout(
                    /*certain loss*/-self.n_price * 2isize.pow(n_jungfrau_exponent),
                    &SPlayerParties13::new(epi_loser),
                )
            };
            let payout_jungfrau_double_individually = |n_jungfrau_exponent| {
                let mut an_payout = EPlayerIndex::map_from_fn(|_epi| 0);
                for epi in EPlayerIndex::values() {
                    if epi != epi_loser {
                        assert_eq!(an_payout[epi], 0);
                        an_payout[epi] = self.n_price;
                        if mapepipointstichcount[epi].n_stich==0 {
                            assert!(1<=n_jungfrau_exponent);
                            an_payout[epi] *= 2isize.pow(n_jungfrau_exponent);
                        }
                        an_payout[epi_loser] -= an_payout[epi];
                    }
                }
                an_payout
            };
            match self.ojungfrau {
                None => {
                    payout_jungfrau_double_all(0)
                },
                Some(VJungfrau::DoubleAll) => {
                    payout_jungfrau_double_all(count_jungfrau_occurences())
                },
                Some(VJungfrau::DoubleIndividuallyOnce) => {
                    payout_jungfrau_double_individually(1)
                },
                Some(VJungfrau::DoubleIndividuallyMultiple) => {
                    payout_jungfrau_double_individually(count_jungfrau_occurences())
                },
            }
        }.map(|n_payout| n_payout * expensifiers.stoss_doubling_factor())
    }

    fn payouthints(&self, (_ahand, _stichseq): (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), _expensifiers: &SExpensifiers, _rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        // TODO sensible payouthints
        EPlayerIndex::map_from_fn(|_epi| SInterval::from_raw([None, None]))
    }

    fn equivalent_when_on_same_hand(&self) -> SCardsPartition {
        use crate::primitives::ECard::*;
        debug_verify_eq!(
            SCardsPartition::new_from_slices(&[
                &[EO, GO, HO, SO] as &[ECard],
                &[EU, GU, HU, SU],
                &[H9, H8, H7],
                &[E9, E8, E7],
                &[G9, G8, G7],
                &[S9, S8, S7],
            ]),
            SCardsPartition::new_from_slices(
                &self.trumpfdecider.equivalent_when_on_same_hand().into_raw().into_iter()
                    .flat_map(|veccard| payoutdecider::equivalent_when_on_same_hand_point_based(&veccard))
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|veccard| veccard as &[ECard]).collect::<Vec<_>>(),
            )
        )
    }

    fn snapshot_cache<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded>(&self, _rulestatecachefixed: &SRuleStateCacheFixed) -> Box<dyn TSnapshotCache<MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>>>
        where
            MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>: PartialEq+fmt::Debug+Clone,
    {
        super::snapshot_cache::<MinMaxStrategiesHK>(|rulestatecache| {
            let mut payload_point_stich_count = 0;
            let point_stich_count = |epi| {
                let pointstichcount = &rulestatecache.changing.mapepipointstichcount[epi];
                let (mut n_point, mut n_stich) = (pointstichcount.n_point, pointstichcount.n_stich);
                let n_stich_max_supported = verify_eq!(EKurzLang::max_cards_per_player(), 8);
                assert!(n_stich <= n_stich_max_supported);
                if n_stich==n_stich_max_supported {
                    // "8" would occupy 4 bits => would be too much
                    // => encode this as n_point==121 points, n_stich==0
                    assert_eq!(120, n_point);
                    n_point += 1;
                    n_stich = 0;
                }
                (n_point, n_stich)
            };
            for (i_epi, epi) in EPlayerIndex::values()
                .skip(1) // first EPI implicitly clear
                .enumerate()
            {
                let (n_point, n_stich) = point_stich_count(epi);
                set_bits!(payload_point_stich_count, n_point, i_epi*10);
                set_bits!(payload_point_stich_count, n_stich, 7 + i_epi*10);
            }
            payload_point_stich_count
        })
    }

    fn alpha_beta_pruner_lohi_values(&self) -> Option<Box<dyn Fn(&SRuleStateCacheFixed)->EnumMap<EPlayerIndex, ELoHi> + Sync>> {
        None
    }
}
