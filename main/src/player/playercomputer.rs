use crate::ai::{
    handiterators::forever_rand_hands,
    gametree::{
        EMinMaxStrategy, explore_snapshots, SMinReachablePayout, SNoVisualization, SSnapshotCacheNone,
    },
    *,
};
use crate::game::*;
use crate::player::*;
use crate::primitives::*;
use crate::rules::{ruleset::*, *};
use crate::util::*;
use std::sync::mpsc;
use itertools::Itertools;

pub struct SPlayerComputer {
    pub ai : SAi,
}

impl TPlayer for SPlayerComputer {
    fn ask_for_doubling(
        &self,
        veccard: &[ECard],
        txb_doubling: mpsc::Sender<bool>,
    ) {
        txb_doubling.send(
            veccard.iter()
                .filter(|card| {
                    ESchlag::Ober==card.schlag() || ESchlag::Unter==card.schlag() || EFarbe::Herz==card.farbe()
                })
                .count() >= 3
            || EFarbe::values().any(|efarbe| {
                veccard.iter()
                    .filter(|card| efarbe==card.farbe())
                    .all_equal()
            })
        ).ok(); // TODO more intelligent doubling strategy
    }

    fn ask_for_card(&self, game: &SGame, txcard: mpsc::Sender<ECard>) {
        txcard.send(self.ai.suggest_card(game, /*fn_visualizer*/SNoVisualization::factory())).ok();
    }

    fn ask_for_game<'rules>(
        &self,
        _epi: EPlayerIndex,
        hand: SFullHand,
        _gameannouncements : &SGameAnnouncements,
        vecrulegroup: &'rules [SRuleGroup],
        expensifiers: &SExpensifiers,
        _otplepiprio: Option<(EPlayerIndex, VGameAnnouncementPriority)>,
        txorules: mpsc::Sender<Option<&'rules dyn TActivelyPlayableRules>>
    ) {
        // TODO: implement a more intelligent decision strategy
        unwrap!(txorules.send(unwrap!(allowed_rules(vecrulegroup, hand)
            .map(|orules| (
                orules,
                orules.map_or(
                    0., // TODO how to rank None?
                    |rules| self.ai.rank_rules(
                        hand,
                        /*epi_rank*/rules.active_playerindex(),
                        rules.upcast(),
                        expensifiers,
                    )[EMinMaxStrategy::Min].avg().as_num::<f64>()
                )
            ))
            .max_by(|&(_orules_lhs, f_payout_avg_lhs), &(_orules_rhs, f_payout_avg_rhs)| {
                assert!(!f_payout_avg_lhs.is_nan());
                assert!(!f_payout_avg_rhs.is_nan());
                unwrap!(f_payout_avg_lhs.partial_cmp(&f_payout_avg_rhs))
            })
        ).0));
    }

    fn ask_for_stoss(
        &self,
        epi: EPlayerIndex,
        rules: &dyn TRules,
        hand: &SHand,
        stichseq: &SStichSequence,
        expensifiers: &SExpensifiers,
        txb: mpsc::Sender<bool>,
    ) {
        let n_samples_per_stoss = 5; // TODO move to ai, make adjustable
        let mut vectplahandf_suspicion = forever_rand_hands(stichseq, hand.clone(), epi, rules, &expensifiers.vecstoss)
            .take(2*n_samples_per_stoss)
            .map(|ahand| {
                let f_rank_rules = rules.playerindex().map_or(0f64, |epi_active| {
                    if epi!=epi_active {
                        self.ai.rank_rules(
                            SFullHand::new(ahand[epi_active].cards(), stichseq.kurzlang()),
                            /*epi_rank*/epi_active,
                            rules,
                            expensifiers,
                        )[EMinMaxStrategy::Min].avg().as_num::<f64>()
                    } else {
                        0f64
                    }
                });
                (ahand, f_rank_rules)
            })
            .collect::<Vec<_>>();
        vectplahandf_suspicion.sort_unstable_by(|&(ref _ahand_l, f_rank_l), &(ref _ahand_r, f_rank_r)|
            unwrap!(f_rank_r.partial_cmp(&f_rank_l))
        );
        vectplahandf_suspicion.truncate(n_samples_per_stoss);
        assert_eq!(n_samples_per_stoss, vectplahandf_suspicion.len());
        unwrap!(txb.send(
            vectplahandf_suspicion.into_iter()
                .map(|(mut ahand, _f_rank_rules)| {
                    explore_snapshots(
                        (&mut ahand, &mut SStichSequence::new(stichseq.kurzlang())),
                        rules,
                        &SBranchingFactor::factory(1, 2),
                        &SMinReachablePayout::new(
                            rules,
                            epi,
                            expensifiers.clone(),
                        ),
                        &SSnapshotCacheNone::factory(), // TODO? use cache
                        &mut SNoVisualization,
                    ).0[EMinMaxStrategy::Min][epi]
                })
                .sum::<isize>().as_num::<f64>()
                / n_samples_per_stoss.as_num::<f64>()
                > 10f64
        ))
    }

    fn name(&self) -> &str {
        "SPlayerComputer" // TODO
    }
}
