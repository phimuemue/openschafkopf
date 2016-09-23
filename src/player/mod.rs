pub mod playercomputer;
pub mod playerhuman;

use primitives::*;
use rules::*;
use rules::ruleset::*;
use game::*;

use std::sync::mpsc;

pub trait TPlayer {
    fn ask_for_doubling(
        &self,
        veccard: &[SCard],
        txb_doubling: mpsc::Sender<bool>,
    );

    fn take_control(&mut self, game: &SGame, txcard: mpsc::Sender<SCard>);
    // TODO: players need information about who already wants to play
    fn ask_for_game<'rules>(
        &self,
        hand: &SHand,
        gameannouncements: &SGameAnnouncements,
        vecrulegroup: &'rules Vec<SRuleGroup>,
        txorules: mpsc::Sender<Option<&'rules TActivelyPlayableRules>>
    );

    fn ask_for_stoss(
        &self,
        eplayerindex: EPlayerIndex,
        doublings: &SDoublings,
        rules: &TRules,
        hand: &SHand,
        vecstoss: &Vec<SStoss>,
        txb: mpsc::Sender<bool>,
    );
}
