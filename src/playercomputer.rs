use stich::*;
use card::*;
use hand::*;
use player::*;
use gamestate::*;
use rules::*;
use ruleset::*;

use std::sync::mpsc;

pub struct CPlayerComputer;

impl CPlayer for CPlayerComputer {
    fn take_control(&mut self, gamestate: &SGameState, txcard: mpsc::Sender<CCard>) {
        // TODO: implement some kind of strategy
        // computer player immediately plays card
        txcard.send(
            gamestate.m_rules.all_allowed_cards(
                &gamestate.m_vecstich,
                &gamestate.m_ahand[
                    // infer player index by stich
                    gamestate.m_vecstich.last().unwrap().current_player_index()
                ]
            )[0]
        ).ok();
    }

    fn ask_for_game(&self, eplayerindex: EPlayerIndex, hand: &CHand) -> Option<Box<TRules>> {
        // TODO: implement a more intelligent decision strategy
        ruleset_default(eplayerindex).allowed_rules().into_iter()
            .filter(|rules| rules.can_be_played(hand))
            .max_by_key(|rules| {
                hand.cards().iter().filter(|&card| {
                    rules.clone().is_trumpf(*card)
                })
                .count()>=4
            })
    }
}
