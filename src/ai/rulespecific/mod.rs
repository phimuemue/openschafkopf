pub mod airufspiel;
use primitives::*;
use game::*;

pub trait TRuleSpecificAI {
    fn suggest_card(&self, game: &SGame) -> Option<SCard>;
}
