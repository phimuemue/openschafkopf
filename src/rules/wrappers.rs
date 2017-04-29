use primitives::*;

// thin wrappers ensuring invariants

pub struct SFullHand<'hand> {
    hand: &'hand SHand,
}

impl<'hand> SFullHand<'hand> {
    pub fn new(hand: &SHand) -> SFullHand {
        assert_eq!(hand.cards().len(), 8);
        SFullHand {
            hand : hand,
        }
    }
    pub fn get(&self) -> &SHand {
        self.hand
    }
}

pub struct SGameFinishedStiche<'vecstich> {
    vecstich: &'vecstich [SStich],
}

impl<'vecstich> SGameFinishedStiche<'vecstich> {
    pub fn new(vecstich: &[SStich]) -> SGameFinishedStiche {
        assert_eq!(vecstich.len(), 8);
        assert!(vecstich.iter().all(|stich| 4==stich.size()));
        SGameFinishedStiche {
            vecstich : vecstich,
        }
    }
    pub fn get(&self) -> &[SStich] {
        assert_eq!(self.vecstich.len(), 8);
        self.vecstich
    }
}

