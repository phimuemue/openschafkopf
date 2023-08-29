pub mod airufspiel;
use crate::primitives::*;

pub trait TRuleSpecificAI {
    fn suggest_card(&self, hand: &SHand, stichseq: &SStichSequence) -> Option<ECard>;
}
