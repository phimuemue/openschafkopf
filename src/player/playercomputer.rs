use card::*;
use hand::*;
use player::*;
use rules::*;
use rules::ruleset::*;
use game::*;
use ai;

use std::sync::mpsc;

pub struct CPlayerComputer;

impl CPlayer for CPlayerComputer {
    fn take_control(&mut self, gamestate: &SGameState, txcard: mpsc::Sender<CCard>) {
        txcard.send(ai::suggest_card(gamestate)).ok();
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
                    ai::rank_rules(hand, eplayerindex_fixed, rules, n_tests_per_rules)
                )
            })
            .filter(|&(_rules, f_payout_avg)| f_payout_avg > 10.) // TODO determine sensible threshold
            .max_by_key(|&(_rules, f_payout_avg)| f_payout_avg as isize) // TODO f64 no Ord => what to do?
            .map(|(rules, _f_payout_avg)| rules)
    }
}
