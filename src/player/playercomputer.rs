use crate::primitives::*;
use crate::player::*;
use crate::rules::{
    *,
    ruleset::*,
};
use crate::game::*;
use crate::ai::{
    *,
    handiterators::forever_rand_hands2,
    suspicion::{explore_snapshots, SMinReachablePayout, SMinReachablePayoutParams},
};
use crate::util::*;
use std::sync::mpsc;

pub struct SPlayerComputer {
    pub ai : Box<TAi>,
}

impl TPlayer for SPlayerComputer {
    fn ask_for_doubling(
        &self,
        veccard: &[SCard],
        txb_doubling: mpsc::Sender<bool>,
    ) {
        txb_doubling.send(
            veccard.iter()
                .filter(|card| {
                    ESchlag::Ober==card.schlag() || ESchlag::Unter==card.schlag() || EFarbe::Herz==card.farbe()
                })
                .count() >= 3
            || EFarbe::values().any(|efarbe| {
                4==veccard.iter()
                    .filter(|card| efarbe==card.farbe())
                    .count()
            })
        ).ok(); // TODO more intelligent doubling strategy
    }

    fn ask_for_card(&self, game: &SGame, txcard: mpsc::Sender<SCard>) {
        txcard.send(self.ai.suggest_card(game, /*ostr_file_out*/None)).ok();
    }

    fn ask_for_game<'rules>(
        &self,
        _epi: EPlayerIndex,
        hand: SFullHand,
        gameannouncements : &SGameAnnouncements,
        vecrulegroup: &'rules [SRuleGroup],
        n_stock: isize,
        _opairepiprio: Option<(EPlayerIndex, VGameAnnouncementPriority)>,
        txorules: mpsc::Sender<Option<&'rules TActivelyPlayableRules>>
    ) {
        // TODO: implement a more intelligent decision strategy
        verify!(txorules.send(verify!(allowed_rules(vecrulegroup, hand)
            .map(|orules| (
                orules,
                orules.map_or(
                    0., // TODO how to rank None?
                    |rules| self.ai.rank_rules(
                        hand,
                        /*epi_first*/gameannouncements.first_playerindex(),
                        /*epi_rank*/rules.active_playerindex(),
                        rules.upcast(),
                        n_stock
                    )
                )
            ))
            .max_by(|&(_orules_lhs, f_payout_avg_lhs), &(_orules_rhs, f_payout_avg_rhs)| {
                assert!(!f_payout_avg_lhs.is_nan());
                assert!(!f_payout_avg_rhs.is_nan());
                verify!(f_payout_avg_lhs.partial_cmp(&f_payout_avg_rhs)).unwrap()
            }))
            .unwrap()
            .0
        )).unwrap();
    }

    fn ask_for_stoss(
        &self,
        epi: EPlayerIndex,
        doublings: &SDoublings,
        rules: &TRules,
        hand: &SHand,
        vecstoss: &[SStoss],
        n_stock: isize,
        txb: mpsc::Sender<bool>,
    ) {
        let n_samples_per_stoss = 5; // TODO move to ai, make adjustable
        let ekurzlang = EKurzLang::from_cards_per_player(hand.cards().len());
        let mut vecpairahandf_suspicion = forever_rand_hands2(/*stichseq*/&SStichSequence::new(doublings.first_playerindex(), ekurzlang), hand.clone(), epi, ekurzlang, rules)
            .take(2*n_samples_per_stoss)
            .map(|ahand| {
                let f_rank_rules = rules.playerindex().map_or(0f64, |epi_active| {
                    if epi!=epi_active {
                        self.ai.rank_rules(
                            SFullHand::new(&ahand[epi_active], ekurzlang),
                            /*epi_first*/doublings.first_playerindex(),
                            /*epi_rank*/epi_active,
                            rules,
                            n_stock,
                        )
                    } else {
                        0f64
                    }
                });
                (ahand, f_rank_rules)
            })
            .collect::<Vec<_>>();
        vecpairahandf_suspicion.sort_unstable_by(|&(ref _ahand_l, f_rank_l), &(ref _ahand_r, f_rank_r)|
            verify!(f_rank_r.partial_cmp(&f_rank_l)).unwrap()
        );
        vecpairahandf_suspicion.truncate(n_samples_per_stoss);
        assert_eq!(n_samples_per_stoss, vecpairahandf_suspicion.len());
        verify!(txb.send(
            vecpairahandf_suspicion.into_iter()
                .map(|(mut ahand, _f_rank_rules)| {
                    explore_snapshots(
                        epi,
                        &mut ahand,
                        rules,
                        &mut SStichSequence::new(doublings.first_playerindex(), ekurzlang),
                        &|_vecstich, veccard_allowed| {
                            assert!(!veccard_allowed.is_empty());
                            random_sample_from_vec(veccard_allowed, 1);
                        },
                        &mut SMinReachablePayout(SMinReachablePayoutParams::new(
                            rules,
                            epi,
                            /*tpln_stoss_doubling*/stoss_and_doublings(vecstoss, doublings),
                            n_stock,
                        )),
                        /*ostr_file_out*/None,
                    )
                })
                .sum::<isize>().as_num::<f64>()
                / n_samples_per_stoss.as_num::<f64>()
                > 10f64
        )).unwrap()
    }
}
