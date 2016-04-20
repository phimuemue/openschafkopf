pub mod playercomputer;
pub mod playerhuman;

use card::*;
use hand::*;
use rules::*;
use rules::ruleset::*;
use game::*;

use std::sync::mpsc;

pub trait TPlayer {
    fn take_control(&mut self, gamestate: &SGameState, txcard: mpsc::Sender<SCard>);
    // TODO: players need information about who already wants to play
    fn ask_for_game<'rules>(
        &self,
        hand: &SHand,
        vecgameannouncement: &Vec<SGameAnnouncement>,
        ruleset : &'rules SRuleSet
    ) -> Option<&'rules TRules>;
}
