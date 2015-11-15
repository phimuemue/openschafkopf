use hand::*;
use rules::*;
use stich::*;

use std::sync::Arc;

pub struct SGameState {
    pub m_ahand : [CHand; 4],
    pub m_rules : Box<TRules>,
    pub m_vecstich : Vec<CStich>,
}
