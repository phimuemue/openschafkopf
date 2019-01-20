use crate::primitives::*;
use crate::game::*;

// thin wrappers ensuring invariants

#[derive(Copy, Clone)]
pub struct SFullHand<'hand>(&'hand SHand);

impl<'hand> SFullHand<'hand> {
    pub fn new(hand: &SHand, ekurzlang: EKurzLang) -> SFullHand {
        assert_eq!(hand.cards().len(), ekurzlang.cards_per_player());
        SFullHand(hand)
    }
    pub fn get(self) -> &'hand SHand {
        self.0
    }
}

#[derive(Copy, Clone)]
pub struct SStichSequenceGameFinished<'stichseq>(&'stichseq SStichSequence);

impl SStichSequenceGameFinished<'_> {
    pub fn new(stichseq: &SStichSequence) -> SStichSequenceGameFinished {
        assert_eq!(stichseq.completed_stichs().get().len(), stichseq.kurzlang().cards_per_player());
        SStichSequenceGameFinished(stichseq)
    }
    pub fn get(&self) -> &SStichSequence {
        self.0
    }
}

#[derive(Copy, Clone)]
pub struct SCompletedStichs<'slcstich>(&'slcstich [SStich]);

impl SCompletedStichs<'_> {
    pub fn new(slcstich: &[SStich]) -> SCompletedStichs {
        assert!(slcstich.iter().all(SStich::is_full));
        SCompletedStichs(slcstich)
    }
    pub fn get(&self) -> &[SStich] {
        self.0
    }
}
