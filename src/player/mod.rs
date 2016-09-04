pub mod playercomputer;
pub mod playerhuman;

use primitives::*;
use rules::*;
use rules::ruleset::*;
use game::*;

use std::sync::mpsc;

pub trait TPlayer {
    fn take_control(&mut self, game: &SGame, txcard: mpsc::Sender<SCard>);
    // TODO: players need information about who already wants to play
    fn ask_for_game<'rules>(
        &self,
        hand: &SHand,
        vecgameannouncement: &Vec<SGameAnnouncement>,
        vecrulegroup: &'rules Vec<SRuleGroup>
    ) -> Option<&'rules TRules>;

    fn ask_for_stoss(
        &self,
        eplayerindex: EPlayerIndex,
        rules: &TRules,
        hand: &SHand,
        vecstoss: &Vec<SStoss>,
    ) -> bool;
}
