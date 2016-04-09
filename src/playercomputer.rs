use card::*;
use hand::*;
use player::*;
use rules::*;
use ruleset::*;
use game::*;
use stich::*;
use suspicion::*;

use std::sync::mpsc;
use rand;
use rand::Rng;

pub struct CPlayerComputer;

impl CPlayerComputer {
    pub fn rank_rules (&self, hand_fixed: &CHand, eplayerindex_fixed: EPlayerIndex, rules: &Box<TRules>, n_tests: usize) -> f64 {
        (0..n_tests)
            .map(|_i_test| {
                let mut vecocard : Vec<Option<CCard>> = CCard::all_values().into_iter()
                    .map(|card| 
                         if hand_fixed.contains(card) {
                             None
                         } else {
                             Some(card)
                         }
                    )
                    .collect();
                let mut susp = SSuspicion::new_from_raw(
                    eplayerindex_fixed,
                    create_playerindexmap(|eplayerindex| {
                        if eplayerindex_fixed==eplayerindex {
                            hand_fixed.clone()
                        } else {
                            random_hand(&mut vecocard)
                        }
                    })
                );
                susp.compute_successors(rules.as_ref(), &mut Vec::new(), &|vecstich_successor| {
                    if !vecstich_successor.is_empty() {
                        let i_stich = rand::thread_rng().gen_range(0, vecstich_successor.len());
                        let stich = vecstich_successor[i_stich].clone();
                        vecstich_successor.clear();
                        vecstich_successor.push(stich);
                    }
                });
                susp.min_reachable_payout(rules.as_ref(), &mut Vec::new(), None, eplayerindex_fixed)
            })
            .fold(0, |n_payout_acc, n_payout| n_payout_acc+n_payout) as f64
            / n_tests as f64
    }
}

impl CPlayer for CPlayerComputer {
    fn take_control(&mut self, gamestate: &SGameState, txcard: mpsc::Sender<CCard>) {
        // TODO: implement some kind of strategy
        // computer player immediately plays card
        txcard.send(
            gamestate.m_rules.all_allowed_cards(
                &gamestate.m_vecstich,
                &gamestate.m_ahand[
                    // infer player index by stich
                    gamestate.m_vecstich.last().unwrap().current_player_index()
                ]
            )[0]
        ).ok();
    }

    fn ask_for_game<'rules>(&self, hand: &CHand, _ : &Vec<SGameAnnouncement>, ruleset: &'rules SRuleSet) -> Option<&'rules Box<TRules>> {
        // TODO: implement a more intelligent decision strategy
        let n_tests_per_rules = 50;
        ruleset.allowed_rules().iter()
            .filter(|rules| rules.can_be_played(hand))
            .filter(|rules| {
                4 <= hand.cards().iter()
                    .filter(|&card| rules.is_trumpf(*card))
                    .count()
            })
            .map(|rules| {
                let eplayerindex_fixed = rules.playerindex().unwrap(); 
                (
                    rules,
                    self.rank_rules(hand, eplayerindex_fixed, rules, n_tests_per_rules)
                )
            })
            .filter(|&(_rules, f_payout_avg)| f_payout_avg > 10.) // TODO determine sensible threshold
            .max_by_key(|&(_rules, f_payout_avg)| f_payout_avg as isize) // TODO f64 no Ord => what to do?
            .map(|(rules, _f_payout_avg)| rules)
    }
}
