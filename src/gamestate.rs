use hand::*;
use rules::*;
use stich::*;

use std::rc::Rc;

pub struct SGameState {
    pub m_ahand : [CHand; 4],
    pub m_rules : Rc<TRules>,
    pub m_vecstich : Vec<CStich>,
}

impl SGameState {
    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        if 8==self.m_vecstich.len() && 4==self.m_vecstich.last().unwrap().size() {
            None
        } else {
            Some(
                (self.m_vecstich.last().unwrap().first_player_index() + self.m_vecstich.last().unwrap().size() ) % 4
            )
        }
    }
}
