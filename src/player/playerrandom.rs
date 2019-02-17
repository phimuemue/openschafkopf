use crate::primitives::*;
use crate::player::*;
use crate::rules::{
    *,
    ruleset::*,
};
use crate::game::*;
#[cfg(debug_assertions)]
use crate::util::*;
use std::sync::mpsc;
use rand::prelude::*;

#[derive(new)]
pub struct SPlayerRandom<FnCheckAskForCard: Fn(&SGame)> {
    fn_check_ask_for_card: FnCheckAskForCard,
}

impl<FnCheckAskForCard: Fn(&SGame)> TPlayer for SPlayerRandom<FnCheckAskForCard> {
    fn ask_for_doubling(
        &self,
        _veccard: &[SCard],
        txb_doubling: mpsc::Sender<bool>,
    ) {
        debug_verify!(txb_doubling.send(rand::random())).unwrap();
    }

    fn ask_for_card(&self, game: &SGame, txcard: mpsc::Sender<SCard>) {
        (self.fn_check_ask_for_card)(game);
        debug_verify!(txcard.send(
            debug_verify!(
                game.rules.all_allowed_cards(
                    &game.stichseq,
                    &game.ahand[debug_verify!(game.which_player_can_do_something()).unwrap().0],
                ).choose(&mut rand::thread_rng()).cloned()
            ).unwrap()
        )).unwrap();
    }

    fn ask_for_game<'rules>(
        &self,
        _epi: EPlayerIndex,
        hand: SFullHand,
        _gameannouncements: &SGameAnnouncements,
        vecrulegroup: &'rules [SRuleGroup],
        _n_stock: isize,
        _opairepiprio: Option<(EPlayerIndex, VGameAnnouncementPriority)>,
        txorules: mpsc::Sender<Option<&'rules dyn TActivelyPlayableRules>>
    ) {
        debug_verify!(txorules.send(
            debug_verify!(allowed_rules(vecrulegroup, hand).choose(&mut rand::thread_rng())).unwrap()
        )).unwrap();
    }

    fn ask_for_stoss(
        &self,
        _epi: EPlayerIndex,
        _doublings: &SDoublings,
        _rules: &dyn TRules,
        _hand: &SHand,
        _vecstoss: &[SStoss],
        _n_stock: isize,
        txb: mpsc::Sender<bool>,
    ) {
        debug_verify!(txb.send(rand::random())).unwrap();
    }
}
