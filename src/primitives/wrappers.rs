use primitives::*;

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
pub struct SGameFinishedStiche<'slcstich>(&'slcstich [SStich]);

impl<'slcstich> SGameFinishedStiche<'slcstich> {
    pub fn new(slcstich: &[SStich], ekurzlang: EKurzLang) -> SGameFinishedStiche {
        assert_eq!(slcstich.len(), ekurzlang.cards_per_player());
        assert!(slcstich.iter().all(|stich| 4==stich.size()));
        SGameFinishedStiche(slcstich)
    }
    pub fn get(&self) -> &[SStich] {
        self.0
    }
}

#[derive(Copy, Clone)]
pub struct SCompletedStichs<'slcstich>(&'slcstich [SStich]);

impl<'slcstich> SCompletedStichs<'slcstich> {
    pub fn new(slcstich: &[SStich]) -> SCompletedStichs {
        assert!(slcstich.iter().all(|stich| stich.size()==4));
        SCompletedStichs(slcstich)
    }
    pub fn get(&self) -> &[SStich] {
        self.0
    }
}
