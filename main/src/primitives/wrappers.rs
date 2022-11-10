use crate::primitives::*;

// thin wrappers ensuring invariants

#[derive(Copy, Clone)]
pub struct SFullHand<'hand>(&'hand [SCard]);

impl<'hand> SFullHand<'hand> {
    pub fn new(slccard: &[SCard], ekurzlang: EKurzLang) -> SFullHand {
        assert_eq!(slccard.len(), ekurzlang.cards_per_player());
        SFullHand(slccard)
    }
    pub fn get(self) -> &'hand [SCard] {
        self.0
    }
}

#[derive(Copy, Clone)]
pub struct SStichSequenceGameFinished<'stichseq>(&'stichseq SStichSequence);

impl SStichSequenceGameFinished<'_> {
    pub fn new(stichseq: &SStichSequence) -> SStichSequenceGameFinished {
        assert!(stichseq.game_finished());
        SStichSequenceGameFinished(stichseq)
    }
    pub fn get(&self) -> &SStichSequence {
        self.0
    }
}
