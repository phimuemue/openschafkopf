pub mod airufspiel;
use crate::primitives::*;
use crate::game::*;

pub trait TRuleSpecificAI {
    fn suggest_card(&self, game: &SGame) -> Option<SCard>;
}
