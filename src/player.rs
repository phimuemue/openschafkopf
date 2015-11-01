use card::*;
use hand::*;
use stich::*;
use rules::*;
use gamestate::*;

pub trait CPlayer {
    fn take_control<FnPlayCard>(&mut self, gamestate: &SGameState, fn_play_card : FnPlayCard)
        where FnPlayCard : Fn(CCard);
    fn ask_for_game(&self, hand: &CHand) -> Option<Box<TRules>>;
}
