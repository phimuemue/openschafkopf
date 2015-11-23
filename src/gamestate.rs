use hand::*;
use rules::*;
use stich::*;

pub struct SGameState {
    pub m_ahand : [CHand; 4],
    pub m_rules : Box<TRules>,
    pub m_vecstich : Vec<CStich>,
}
