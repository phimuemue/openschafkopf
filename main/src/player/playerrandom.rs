use crate::game::*;
use crate::player::*;
use crate::primitives::*;
use crate::rules::{ruleset::*, *};
use crate::util::*;
use rand::prelude::*;
use std::sync::mpsc;

#[derive(new)]
pub struct SPlayerRandom<FnCheckAskForGame, FnCheckAskForCard> {
    fn_check_ask_for_game: FnCheckAskForGame,
    fn_check_ask_for_card: FnCheckAskForCard,
}

impl<FnCheckAskForGame: Fn(SFullHand, &[SRuleGroup], (usize, usize)/*tpln_stoss_doubling*/, isize/*n_stock*/), FnCheckAskForCard: Fn(&SGame)> TPlayer for SPlayerRandom<FnCheckAskForGame, FnCheckAskForCard> {
    fn ask_for_doubling(
        &self,
        _veccard: &[SCard],
        txb_doubling: mpsc::Sender<bool>,
    ) {
        unwrap!(txb_doubling.send(rand::random()));
    }

    fn ask_for_card(&self, game: &SGame, txcard: mpsc::Sender<SCard>) {
        (self.fn_check_ask_for_card)(game);
        unwrap!(txcard.send(
            unwrap!(
                game.rules.all_allowed_cards(
                    &game.stichseq,
                    &game.ahand[unwrap!(game.which_player_can_do_something()).0],
                ).choose(&mut rand::thread_rng()).copied()
            )
        ));
    }

    fn ask_for_game<'rules>(
        &self,
        _epi: EPlayerIndex,
        hand: SFullHand,
        _gameannouncements: &SGameAnnouncements,
        vecrulegroup: &'rules [SRuleGroup],
        tpln_stoss_doubling: (usize, usize),
        n_stock: isize,
        _otplepiprio: Option<(EPlayerIndex, VGameAnnouncementPriority)>,
        txorules: mpsc::Sender<Option<&'rules dyn TActivelyPlayableRules>>
    ) {
        (self.fn_check_ask_for_game)(hand, vecrulegroup, tpln_stoss_doubling, n_stock);
        unwrap!(txorules.send(
            unwrap!(allowed_rules(vecrulegroup, hand).choose(&mut rand::thread_rng()))
        ));
    }

    fn ask_for_stoss(
        &self,
        _epi: EPlayerIndex,
        _doublings: &SDoublings,
        _rules: &dyn TRules,
        _hand: &SHand,
        _stichseq: &SStichSequence,
        _vecstoss: &[SStoss],
        _n_stock: isize,
        txb: mpsc::Sender<bool>,
    ) {
        unwrap!(txb.send(rand::random()));
    }

    fn name(&self) -> &str {
        "SPlayerRandom" // TODO
    }
}
