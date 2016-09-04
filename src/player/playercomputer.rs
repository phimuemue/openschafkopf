use primitives::*;
use player::*;
use rules::*;
use rules::ruleset::*;
use game::*;
use ai::TAi;

use std::sync::mpsc;

pub struct SPlayerComputer<'ai> {
    pub m_ai : &'ai TAi,
}

impl<'ai> TPlayer for SPlayerComputer<'ai> {
    fn take_control(&mut self, game: &SGame, txcard: mpsc::Sender<SCard>) {
        txcard.send(self.m_ai.suggest_card(game)).ok();
    }

    fn ask_for_game<'rules>(&self, hand: &SHand, _ : &Vec<SGameAnnouncement>, vecrulegroup: &'rules Vec<SRuleGroup>) -> Option<&'rules TRules> {
        // TODO: implement a more intelligent decision strategy
        let n_tests_per_rules = 50;
        allowed_rules(vecrulegroup).iter()
            .filter(|rules| rules.can_be_played(hand))
            .filter(|rules| {
                4 <= hand.cards().iter()
                    .filter(|&card| rules.is_trumpf(*card))
                    .count()
            })
            .map(|&rules| {
                let eplayerindex_fixed = rules.playerindex().unwrap(); 
                (
                    rules,
                    self.m_ai.rank_rules(hand, eplayerindex_fixed, rules, n_tests_per_rules)
                )
            })
            .filter(|&(_rules, f_payout_avg)| f_payout_avg > 10.) // TODO determine sensible threshold
            .max_by_key(|&(_rules, f_payout_avg)| f_payout_avg as isize) // TODO rust: Use max_by
            .map(|(rules, _f_payout_avg)| rules)
    }

    fn ask_for_stoss(
        &self,
        eplayerindex: EPlayerIndex,
        rules: &TRules,
        hand: &SHand,
        _vecstoss: &Vec<SStoss>,
    ) -> bool {
        self.m_ai.rank_rules(hand, eplayerindex, rules, /*n_tests_per_rules*/ 100) > 10f64 // TODO determine sensible threshold
    }
}
