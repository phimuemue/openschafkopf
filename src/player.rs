use card::*;
use hand::*;
use stich::*;
use rules::*;
use gamestate::*;

use std::sync::mpsc;
use std::rc::Rc;

pub trait CPlayer {
    fn take_control(&mut self, gamestate: &SGameState, txcard: mpsc::Sender<CCard>);
    // TODO: players need information about who already wants to play
    fn ask_for_game(&self, eplayerindex: EPlayerIndex, hand: &CHand) -> Option<Box<TRules>>;
}
