use card::*;
use hand::*;
use stich::*;
use rules::*;
use gamestate::*;

use std::sync::mpsc;

pub trait CPlayer {
    fn take_control(&mut self, gamestate: &SGameState, txcard: mpsc::Sender<CCard>);
    fn ask_for_game(&self, eplayerindex: EPlayerIndex, hand: &CHand) -> Option<Box<TRules>>;
}
