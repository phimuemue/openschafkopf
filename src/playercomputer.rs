use card::*;
use hand::*;
use player::*;
use gamestate::*;
use rules::*;

struct CPlayerComputer;

impl CPlayer for CPlayerComputer {
    fn take_control<FnPlayCard>(&mut self, gamestate: &SGameState, fn_play_card : FnPlayCard)
        where FnPlayCard : Fn(CCard) 
    {
        // TODO: implement some kind of strategy
        // computer player immediately plays card by calling fn_play_card
        return fn_play_card(gamestate.m_rules.all_allowed_cards(
            &gamestate.m_vecstich,
            &gamestate.m_ahand[
                // infer player index by stich
                gamestate.m_vecstich.last().unwrap().current_player_index()
            ]
        )[0]);
    }

    fn ask_for_game(&self, hand: &CHand) -> Option<Box<TRules>> {
        unimplemented!();
        return None; // TODO: implement this (probably by just counting trumpf cards or similar)
    }
}
