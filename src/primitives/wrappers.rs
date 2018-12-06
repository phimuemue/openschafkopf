use crate::primitives::*;
use crate::util::*;

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
        assert!(slcstich.iter().all(|stich| 4==stich.size()));
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
        assert!(slcstich.iter().all(|stich| stich.size()==4));
        SCompletedStichs(slcstich)
    }
    pub fn get(&self) -> &[SStich] {
        self.0
    }
}

pub struct SVecStichPushPop<'vecstich>(&'vecstich mut Vec<SStich>); // TODO generalize
pub struct SVecStichPushPopTmp<'vecstich>(&'vecstich mut Vec<SStich>);

impl<'vecstich> SVecStichPushPop<'vecstich> {
    pub fn new(vecstich: &'vecstich mut Vec<SStich>) -> Self {
        assert!(vecstich.iter().all(|stich| stich.size()==4));
        SVecStichPushPop(vecstich)
    }

    pub fn push_pop<Func, R>(&mut self, stich: SStich, func: Func) -> R
        where
            for<'vecstichtmp> Func: FnOnce(SVecStichPushPopTmp<'vecstichtmp>) -> R,
    {
        let n_stich = self.0.len();
        assert!(self.0.iter().all(|stich| stich.size()==4));
        self.0.push(stich);
        let r = func(SVecStichPushPopTmp(self.0));
        verify!(self.0.pop()).unwrap();
        assert!(self.0.iter().all(|stich| stich.size()==4));
        assert_eq!(n_stich, self.0.len());
        r
    }

    pub fn get(&self) -> SCompletedStichs {
        SCompletedStichs::new(self.0)
    }
}

impl<'vecstich> SVecStichPushPopTmp<'vecstich> {
    pub fn into_pushpop(self) -> SVecStichPushPop<'vecstich> {
        SVecStichPushPop::new(self.0)
    }

    pub fn get(&self) -> &[SStich] {
        self.0
    }

    pub fn get_mut(&mut self) -> &mut [SStich] {
        self.0
    }
}
