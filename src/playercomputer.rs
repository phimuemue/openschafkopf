use stich::*;
use card::*;
use hand::*;
use player::*;
use gamestate::*;
use rules::*;
use rulesrufspiel::*;

use std::sync::mpsc;
use std::rc::Rc;

pub struct CPlayerComputer;

impl CPlayer for CPlayerComputer {
    fn take_control(&mut self, gamestate: &SGameState, txcard: mpsc::Sender<CCard>) {
        println!("CPlayerComputer {} taking control", gamestate.m_vecstich.last().unwrap().current_player_index());
        // TODO: implement some kind of strategy
        // computer player immediately plays card by calling fn_play_card
        txcard.send(
            gamestate.m_rules.all_allowed_cards(
                &gamestate.m_vecstich,
                &gamestate.m_ahand[
                    // infer player index by stich
                    gamestate.m_vecstich.last().unwrap().current_player_index()
                ]
            )[0]
        ).ok();
        println!("CPlayerComputer {} played card", gamestate.m_vecstich.last().unwrap().current_player_index());
    }

    fn ask_for_game(&self, _eplayerindex: EPlayerIndex, _: &CHand) -> Option<Rc<TRules>> {
        // TODO: implement a more intelligent decision strategy
        None
    }
}
