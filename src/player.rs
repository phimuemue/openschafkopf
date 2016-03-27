use card::*;
use hand::*;
use rules::*;
use ruleset::*;
use game::*;

use std::sync::mpsc;

pub trait CPlayer {
    fn take_control(&mut self, gamestate: &SGameState, txcard: mpsc::Sender<CCard>);
    // TODO: players need information about who already wants to play
    fn ask_for_game<'rules>(
        &self,
        hand: &CHand,
        vecgameannouncement: &Vec<SGameAnnouncement>,
        ruleset : &'rules SRuleSet
    ) -> Option<&'rules Box<TRules>>;
}
