use primitives::*;
use player::*;
use rules::{
    *,
    ruleset::*,
};
use game::*;
use util::*;
use std::sync::mpsc;
use rand;

#[derive(new)]
pub struct SPlayerRandom<FnCheckAskForCard>
    where FnCheckAskForCard: Fn(&SGame),
{
    fn_check_ask_for_card: FnCheckAskForCard,
}

impl<FnCheckAskForCard> TPlayer for SPlayerRandom<FnCheckAskForCard>
    where FnCheckAskForCard: Fn(&SGame),
{
    fn ask_for_doubling(
        &self,
        _veccard: &[SCard],
        txb_doubling: mpsc::Sender<bool>,
    ) {
        verify!(txb_doubling.send(rand::random())).unwrap();
    }

    fn ask_for_card(&self, game: &SGame, txcard: mpsc::Sender<SCard>) {
        (self.fn_check_ask_for_card)(game);
        verify!(txcard.send(
            verify!(rand::seq::sample_iter(
                &mut rand::thread_rng(),
                game.rules.all_allowed_cards(
                    &game.vecstich,
                    &game.ahand[verify!(game.which_player_can_do_something()).unwrap().0],
                ),
                1,
            )).unwrap()[0]
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
        txorules: mpsc::Sender<Option<&'rules TActivelyPlayableRules>>
    ) {
        verify!(txorules.send(
            verify!(rand::seq::sample_iter(
                &mut rand::thread_rng(),
                allowed_rules(vecrulegroup, hand),
                1,
            )).unwrap()[0]
        )).unwrap();
    }

    fn ask_for_stoss(
        &self,
        _epi: EPlayerIndex,
        _doublings: &SDoublings,
        _rules: &TRules,
        _hand: &SHand,
        _vecstoss: &[SStoss],
        _n_stock: isize,
        txb: mpsc::Sender<bool>,
    ) {
        verify!(txb.send(rand::random())).unwrap();
    }
}
