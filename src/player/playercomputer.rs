use primitives::*;
use player::*;
use rules::*;
use rules::ruleset::*;
use game::*;
use ai::*;
use ai::handiterators::forever_rand_hands;
use ai::suspicion::SSuspicion;

use std::sync::mpsc;

pub struct SPlayerComputer<'ai> {
    pub m_ai : &'ai TAi,
}

impl<'ai> TPlayer for SPlayerComputer<'ai> {
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
        txcard.send(self.m_ai.suggest_card(game)).ok();
    }

    fn ask_for_game<'rules>(&self, hand: &SFullHand, gameannouncements : &SGameAnnouncements, vecrulegroup: &'rules Vec<SRuleGroup>, n_stock: isize, txorules: mpsc::Sender<Option<&'rules TActivelyPlayableRules>>) {
        // TODO: implement a more intelligent decision strategy
        let n_tests_per_rules = 50;
        txorules.send(allowed_rules(vecrulegroup).iter()
            .filter(|rules| rules.can_be_played(hand))
            .filter(|rules| {
                4 <= hand.get().cards().iter()
                    .filter(|&card| rules.trumpforfarbe(*card).is_trumpf())
                    .count()
            })
            .map(|rules| {
                let eplayerindex_rank = rules.playerindex().unwrap(); 
                (
                    rules,
                    self.m_ai.rank_rules(hand, /*eplayerindex_first*/gameannouncements.first_playerindex(), eplayerindex_rank, rules.as_rules(), n_stock, n_tests_per_rules)
                )
            })
            .filter(|&(_rules, f_payout_avg)| f_payout_avg > 10.) // TODO determine sensible threshold
            .max_by_key(|&(_rules, f_payout_avg)| f_payout_avg as isize) // TODO rust: Use max_by
            .map(|(rules, _f_payout_avg)| *rules)).unwrap();
    }

    fn ask_for_stoss(
        &self,
        eplayerindex: EPlayerIndex,
        doublings: &SDoublings,
        rules: &TRules,
        hand: &SHand,
        vecstoss: &Vec<SStoss>,
        n_stock: isize,
        txb: mpsc::Sender<bool>,
    ) {
        let n_tests_per_rules = 50;
        let mut vecpairahandf_suspicion = forever_rand_hands(/*vecstich*/&Vec::new(), hand.clone(), eplayerindex)
            .filter(|ahand| is_compatible_with_game_so_far(ahand, rules, /*vecstich*/&vec![SStich::new(doublings.m_eplayerindex_first)])) // stoss currently only in SPreGame
            .take(2*n_tests_per_rules)
            .map(|ahand| {
                let f_rank_rules = rules.playerindex().map_or(0f64, |eplayerindex_active| {
                    if eplayerindex!=eplayerindex_active {
                        self.m_ai.rank_rules(
                            &SFullHand::new(&ahand[eplayerindex_active]),
                            /*eplayerindex_first*/doublings.first_playerindex(),
                            /*eplayerindex_rank*/eplayerindex_active,
                            rules,
                            n_stock,
                            /*n_tests*/10
                        )
                    } else {
                        0f64
                    }
                });
                (ahand, f_rank_rules)
            })
            .collect::<Vec<_>>();
        vecpairahandf_suspicion.sort_by(|&(ref _ahand_l, f_rank_l), &(ref _ahand_r, f_rank_r)|
            f_rank_r.partial_cmp(&f_rank_l).unwrap()
        );
        vecpairahandf_suspicion.truncate(n_tests_per_rules);
        assert_eq!(n_tests_per_rules, vecpairahandf_suspicion.len());
        txb.send(
            vecpairahandf_suspicion.into_iter()
                .map(|(ahand, _f_rank_rules)| {
                    SSuspicion::new(
                        doublings.first_playerindex(),
                        ahand,
                        rules,
                        &mut Vec::new(),
                        |_vecstich_complete, vecstich_successor| {
                            assert!(!vecstich_successor.is_empty());
                            random_sample_from_vec(vecstich_successor, 1);
                        }
                    ).min_reachable_payout(
                        rules,
                        &mut Vec::new(),
                        None,
                        eplayerindex,
                        /*n_stoss*/ vecstoss.len(),
                        /*n_doubling*/doublings.iter().filter(|&(_eplayerindex, &b_doubling)| b_doubling).count(),
                        n_stock,
                    )
                })
                .sum::<isize>() as f64
                / (n_tests_per_rules) as f64
                > 10f64
        ).unwrap()
    }
}
