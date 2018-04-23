use primitives::*;

// thin wrappers ensuring invariants

#[derive(Copy, Clone)]
pub struct SFullHand<'hand> {
    hand: &'hand SHand,
}

impl<'hand> SFullHand<'hand> {
    pub fn new(hand: &SHand, ekurzlang: EKurzLang) -> SFullHand {
        assert_eq!(hand.cards().len(), ekurzlang.cards_per_player());
        SFullHand {
            hand,
        }
    }
    pub fn get(&self) -> &SHand {
        self.hand
    }
}

#[derive(Copy, Clone)]
pub struct SGameFinishedStiche<'vecstich> {
    vecstich: &'vecstich [SStich],
}

impl<'vecstich> SGameFinishedStiche<'vecstich> {
    pub fn new(vecstich: &[SStich], ekurzlang: EKurzLang) -> SGameFinishedStiche {
        assert_eq!(vecstich.len(), ekurzlang.cards_per_player());
        assert!(vecstich.iter().all(|stich| 4==stich.size()));
        SGameFinishedStiche {
            vecstich,
        }
    }
    pub fn get(&self) -> &[SStich] {
        self.vecstich
    }
}

