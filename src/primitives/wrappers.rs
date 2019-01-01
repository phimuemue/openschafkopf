use crate::primitives::*;
use crate::util::*;
use crate::rules::*;

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

impl SGameFinishedStiche<'_> {
    pub fn new(slcstich: &[SStich], ekurzlang: EKurzLang) -> SGameFinishedStiche {
        assert_eq!(slcstich.len(), ekurzlang.cards_per_player());
        assert!(slcstich.iter().all(SStich::is_full));
        SGameFinishedStiche(slcstich)
    }
    pub fn get(&self) -> &[SStich] {
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

#[derive(new)]
pub struct SVecStichPushPop<'vecstich>(&'vecstich mut Vec<SStich>); // TODO generalize

impl<'vecstich> SVecStichPushPop<'vecstich> {
    pub fn push_pop<F, R>(&mut self, card: SCard, rules: &TRules, func: F) -> R
        where
            for<'inner> F: FnOnce(SVecStichPushPop<'inner>)->R
    {
        let n_len = self.0.len();
        assert!(!current_stich(&self.0).is_full());
        current_stich_mut(&mut self.0).push(card);
        if current_stich(&self.0).is_full() {
            self.0.push(SStich::new(rules.winner_index(current_stich(&self.0))));
            assert!(current_stich(&self.0).is_empty());
        }
        let r = func(SVecStichPushPop::new(&mut self.0));
        if current_stich(&self.0).is_empty() {
            verify!(self.0.pop()).unwrap();
            assert!(current_stich(&self.0).is_full());
        }
        current_stich_mut(&mut self.0).undo_most_recent();
        assert!(!current_stich(&self.0).is_full());
        assert_eq!(n_len, self.0.len());
        r
    }

    pub fn get(&self) -> &[SStich] {
        self.0
    }
}
