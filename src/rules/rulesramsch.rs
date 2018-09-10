use primitives::*;
use rules::{
    *,
    trumpfdecider::*,
    payoutdecider::SStossDoublingPayoutDecider,
    card_points::*,
};
use std::{
    fmt,
    cmp::Ordering,
};
use itertools::Itertools;
use util::*;

#[derive(Clone, Debug)]
pub enum VDurchmarsch {
    None,
    All,
    AtLeast(isize),
}

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
    SStaticSchlagUnter, STrumpfDeciderFarbe<
    SStaticFarbeHerz>>>;

impl TRules for SRulesRamsch {
    box_clone_impl_by_clone!(TRules);
    impl_rules_trumpf!(STrumpfDeciderRamsch);

    fn stoss_allowed(&self, _epi: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool {
        assert!(vecstoss.is_empty());
        EKurzLang::from_cards_per_player(hand.cards().len());
        false
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        None
    }

    fn payout(&self, gamefinishedstiche: SGameFinishedStiche, tpln_stoss_doubling: (usize, usize), _n_stock: isize) -> SAccountBalance {
        let apply_doubling_stoss_stock = |epi_single, n_factor_single| {
            SAccountBalance::new(
                SStossDoublingPayoutDecider::payout(
                    &EPlayerIndex::map_from_fn(|epi| {
                        if epi_single==epi {
                            3 * self.n_price * n_factor_single
                        } else {
                            -self.n_price * n_factor_single
                        }
                    }),
                    {
                        assert_eq!(tpln_stoss_doubling.0, 0); // SRulesRamsch does not allow stoss
                        tpln_stoss_doubling
                    },
                ),
                0,
            )
        };
        let an_points = gamefinishedstiche.get().iter()
            .fold(
                EPlayerIndex::map_from_fn(|_epi| 0),
                |mut an_points_accu, stich| {
                    an_points_accu[self.winner_index(stich)] += points_stich(stich);
                    an_points_accu
                }
            );
        let n_points_max = verify!(an_points.iter().max()).unwrap();
        let vecepi_most_points = EPlayerIndex::values()
            .filter(|epi| n_points_max==&an_points[*epi])
            .collect::<Vec<_>>();
        let the_one_epi = || -> EPlayerIndex {
            assert!(*n_points_max>=61);
            assert_eq!(1, vecepi_most_points.len());
            vecepi_most_points[0]
        };
        if { match self.durchmarsch {
            VDurchmarsch::All if 120==*n_points_max =>
                gamefinishedstiche.get().iter().all(|stich| self.winner_index(stich)==the_one_epi()),
            VDurchmarsch::All | VDurchmarsch::None =>
                false,
            VDurchmarsch::AtLeast(n_points_durchmarsch) => {
                assert!(n_points_durchmarsch>=61); // otherwise, it may not be clear who is the durchmarsch winner
                *n_points_max>=n_points_durchmarsch
            },
        } } {
            apply_doubling_stoss_stock(the_one_epi(), 1)
        } else {
            let epi_loser : EPlayerIndex = {
                if 1==vecepi_most_points.len() {
                    vecepi_most_points[0]
                } else {
                    verify!(vecepi_most_points.iter().cloned()
                        .map(|epi| {(
                            epi,
                            gamefinishedstiche.get().iter()
                                .map(|stich| stich[epi])
                                .filter(|card| self.trumpforfarbe(*card).is_trumpf())
                                .max_by(|card_fst, card_snd| {
                                    assert!(self.trumpforfarbe(*card_fst).is_trumpf());
                                    assert!(self.trumpforfarbe(*card_snd).is_trumpf());
                                    self.compare_trumpf(*card_fst, *card_snd)
                                })
                        )})
                        .fold1(|pairepiocard_fst, pairepiocard_snd| {
                            match (pairepiocard_fst.1, pairepiocard_snd.1) {
                                (Some(card_trumpf_fst), Some(card_trumpf_snd)) => {
                                    if Ordering::Less==self.compare_trumpf(card_trumpf_fst, card_trumpf_snd) {
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
                        .unwrap()
                        .0
                }
            };
            apply_doubling_stoss_stock(epi_loser, -1)
        }
    }
}
