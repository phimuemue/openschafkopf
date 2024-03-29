pub mod playercomputer;
#[cfg(test)]
pub mod playerrandom;

use crate::game::*;
use crate::primitives::*;
use crate::rules::{ruleset::*, *};

use std::sync::mpsc;

pub trait TPlayer {
    fn ask_for_doubling(
        &self,
        veccard: &[ECard],
        txb_doubling: mpsc::Sender<bool>,
    );

    fn ask_for_card(&self, game: &SGameGeneric<SRuleSet, (), ()>, txcard: mpsc::Sender<ECard>);
    // TODO: players need information about who already wants to play
    fn ask_for_game<'rules>(
        &self,
        epi: EPlayerIndex,
        hand: SFullHand,
        gameannouncements: &SGameAnnouncements,
        vecrulegroup: &'rules [SRuleGroup],
        expensifiers: &SExpensifiers,
        otplepiprio: Option<(EPlayerIndex, VGameAnnouncementPriority)>,
        txorules: mpsc::Sender<Option<&'rules SActivelyPlayableRules>>
    );

    fn ask_for_stoss(
        &self,
        epi: EPlayerIndex,
        rules: &SRules,
        hand: &SHand,
        stichseq: &SStichSequence,
        expensifiers: &SExpensifiers,
        txb: mpsc::Sender<bool>,
    );

    fn name(&self) -> &str;
}
