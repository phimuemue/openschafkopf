use stich::*;
use card::*;
use hand::*;
use player::*;
use gamestate::*;
use rules::*;
use rulesrufspiel::*;

use std::sync::mpsc;

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

    fn ask_for_game(&self, eplayerindex: EPlayerIndex, _: &CHand) -> Option<Box<TRules>> {
        // TODO: implement a more intelligent decision strategy
        return Some(
            Box::new(
                CRulesRufspiel {
                    m_eplayerindex : eplayerindex,
                    m_efarbe : efarbeEICHEL
                }
            )
        );
    }
}
