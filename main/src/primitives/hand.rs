use crate::primitives::{card::*, eplayerindex::*};
use arrayvec::ArrayVec;
#[cfg(debug_assertions)]
use plain_enum::PlainEnum;
use crate::util::*;
use std::fmt;
use std::borrow::{Borrow, BorrowMut};
use itertools::Itertools;

pub type SHandVector = ArrayVec<SCard, 8>;

#[derive(Clone, Debug)]
pub struct SHand {
    veccard: SHandVector,
}

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


#[cfg(debug_assertions)]
impl std::cmp::PartialEq for SHand {
    fn eq(&self, other: &SHand) -> bool {
        let to_enumset = |hand: &SHand| {
            let mut mapcardb = SCard::map_from_fn(|_| false); // TODO enumset
            for card in hand.cards() {
                verify!(assign_neq(&mut mapcardb[*card], true));
            }
            mapcardb
        };
        to_enumset(self)==to_enumset(other)
    }
}
#[cfg(debug_assertions)]
impl std::cmp::Eq for SHand {}

impl SHand {
    #[cfg(debug_assertions)]
    fn finalize_and_assert_invariant(&mut self) {
        { // SHand is actually a set-like container, users must not rely on the ordering of cards.
            use rand::prelude::SliceRandom;
            self.veccard.shuffle(&mut rand::thread_rng());
        }
        { // invariants
            let mut setcardb = SCard::map_from_fn(|_card| false); // TODO enumset
            for card in self.veccard.iter() {
                verify!(assign_neq(&mut setcardb[*card], true));
            }
        }
    }

    pub fn new_from_vec(veccard: SHandVector) -> SHand {
        #[allow(unused_mut)] // TODO is there a nicer way?
        let mut hand = SHand {veccard};
        #[cfg(debug_assertions)]hand.finalize_and_assert_invariant();
        hand
    }
    pub fn new_from_iter<Card>(itcard: impl IntoIterator<Item=Card>) -> SHand
        where
            Card: TMoveOrClone<SCard>,
    {
        Self::new_from_vec(itcard.into_iter().map(TMoveOrClone::move_or_clone).collect())
    }
    pub fn contains(&self, card_check: SCard) -> bool {
        self.contains_pred(|&card| card==card_check)
    }
    pub fn contains_pred(&self, pred: impl Fn(&SCard)->bool) -> bool {
        self.veccard
            .iter()
            .any(pred)
    }
    pub fn play_card(&mut self, card: SCard) {
        // TODO: assembly for this looks rather inefficient. Possibly replace by simpler 64-bit operations.
        self.veccard.must_find_swap_remove(&card);
        #[cfg(debug_assertions)]self.finalize_and_assert_invariant();
    }
    pub fn add_card(&mut self, card: SCard) {
        debug_assert!(!self.contains(card));
        self.veccard.push(card);
        #[cfg(debug_assertions)]self.finalize_and_assert_invariant();
    }

    pub fn cards(&self) -> &SHandVector {
        &self.veccard
    }
}

pub trait TCardSorter {
    fn sort_cards(&self, slccard: &mut [SCard]);
}

impl<F: Fn(&mut [SCard])> TCardSorter for F {
    fn sort_cards(&self, slccard: &mut [SCard]) {
        self(slccard)
    }
}

impl<CardSorter: TCardSorter> TCardSorter for Option<CardSorter> {
    fn sort_cards(&self, slccard: &mut [SCard]) {
        if let Some(cardsorter) = self {
            cardsorter.sort_cards(slccard);
        }
    }
}

pub struct SDisplayCardSlice<SlcCard>(SlcCard);
impl<SlcCard: BorrowMut<[SCard]>> SDisplayCardSlice<SlcCard> {
    pub fn new(mut slccard: SlcCard, cardsorter: &impl TCardSorter) -> Self {
        cardsorter.sort_cards(slccard.borrow_mut());
        Self(slccard)
    }
}

impl<SlcCard: Borrow<[SCard]>> fmt::Display for SDisplayCardSlice<SlcCard> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.borrow().iter().join(" "))
    }
}

pub fn display_card_slices(ahand: &EnumMap<EPlayerIndex, SHand>, cardsorter: &impl TCardSorter, str_sep: &str) -> String {
    ahand.iter()
        .map(|hand| SDisplayCardSlice::new(hand.cards().clone(), cardsorter))
        .join(str_sep)
}

#[test]
fn test_hand() {
    use super::card::SCard::*;
    let hand = SHand::new_from_iter([EU, HK, S7]);
    let hand2 = {
        let mut hand2 = hand.clone();
        hand2.play_card(SCard::new(EFarbe::Herz, ESchlag::Koenig));
        hand2
    };
    assert_eq!(hand.cards().len()-1, hand2.cards().len());
    assert!(hand2.contains(EU));
    assert!(hand2.contains(S7));
}
