pub mod airufspiel;
use crate::game::*;
use crate::primitives::*;

pub trait TRuleSpecificAI {
    fn suggest_card(&self, game: &SGame) -> Option<SCard>;
}
