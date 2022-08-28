use plain_enum::*;
use itertools::Itertools;
use crate::primitives::card::*;

#[derive(Debug, Clone)]
pub struct SCardsPartition {
    mapcardcard_next: EnumMap<SCard, SCard>,
    mapcardcard_prev: EnumMap<SCard, SCard>,
}

// TODO plain_enum: support derive(PartialEq)
impl PartialEq for SCardsPartition {
    fn eq(&self, other: &Self) -> bool {
        self.mapcardcard_next.as_raw() == other.mapcardcard_next.as_raw()
            && self.mapcardcard_prev.as_raw() == other.mapcardcard_prev.as_raw()
    }
}

#[derive(Debug, Clone)]
pub struct SRemoved {
    card: SCard,
    card_next_old: SCard,
    card_prev_old: SCard,
}

impl SCardsPartition {
    pub fn new() -> Self {
        let cardspartition = Self {
            mapcardcard_next: SCard::map_from_fn(|card| card),
            mapcardcard_prev: SCard::map_from_fn(|card| card),
        };
        cardspartition.assert_invariant();
        cardspartition
    }

    pub fn new_from_slices(slcslccard: &[&[SCard]]) -> Self {
        let mut cardspartition = Self::new();
        for slccard in slcslccard {
            cardspartition.chain(slccard);
        }
        cardspartition.assert_invariant();
        cardspartition
    }

    fn assert_invariant(&self) { #[cfg(debug_assertions)] {
        for card in <SCard as TPlainEnum>::values() {
            if let Some(card_next) = self.next_no_invariant(card) {
                assert!(self.mapcardcard_prev[card_next]==card, "{:?} -> {:?}", self, card);
            }
            if let Some(card_prev) = self.prev_no_invariant(card) {
                assert!(self.mapcardcard_next[card_prev]==card, "{:?} -> {:?}", self, card);
            }
        }
        // TODO
    }}

    pub fn chain(&mut self, slccard: &[SCard]) {
        for card in slccard.iter() {
            assert_eq!(self.mapcardcard_next[*card], *card);
        }
        for (card_lo, card_hi) in slccard.iter().tuple_windows() {
            self.mapcardcard_next[*card_lo] = *card_hi;
            self.mapcardcard_prev[*card_hi] = *card_lo;
        }
        self.assert_invariant();
    }

    pub fn remove_from_chain(&mut self, card: SCard) -> SRemoved {
        #[cfg(debug_assertions)] let cardspartition_clone = self.clone();
        // TODO can the following use fewer branches?
        let removed = match (self.prev(card), self.next(card)) {
            (None, None) => {
                SRemoved{card, card_prev_old:card, card_next_old:card}
            },
            (Some(card_prev_old), None) => {
                self.mapcardcard_next[card_prev_old] = card_prev_old;
                SRemoved{card, card_prev_old, card_next_old:card}
            },
            (None, Some(card_next_old)) => {
                self.mapcardcard_prev[card_next_old] = card_next_old;
                SRemoved{card, card_prev_old:card, card_next_old}
            },
            (Some(card_prev_old), Some(card_next_old)) => {
                assert_ne!(card_prev_old, card_next_old);
                self.mapcardcard_next[card_prev_old] = card_next_old;
                self.mapcardcard_prev[card_next_old] = card_prev_old;
                SRemoved{card, card_prev_old, card_next_old}
            },
        };
        self.mapcardcard_next[card] = card;
        self.mapcardcard_prev[card] = card;
        self.assert_invariant();
        #[cfg(debug_assertions)] // TODO why is this needed?
        debug_assert_eq!(
            cardspartition_clone,
            {
                let mut cardspartition_readd = self.clone();
                cardspartition_readd.readd(removed.clone());
                cardspartition_readd
            },
            "{:?}\n{:?}", card, removed,
        );
        removed
    }

    pub fn readd(&mut self, removed: SRemoved) {
        let card = removed.card;
        assert_eq!(self.mapcardcard_prev[card], card);
        assert_eq!(self.mapcardcard_next[card], card);
        self.mapcardcard_prev[card] = removed.card_prev_old;
        self.mapcardcard_next[card] = removed.card_next_old;
        if card!=removed.card_next_old {
            self.mapcardcard_prev[removed.card_next_old] = card;
        }
        if card!=removed.card_prev_old {
            self.mapcardcard_next[removed.card_prev_old] = card;
        }
        self.assert_invariant();
    }

    pub fn next(&self, card: SCard) -> Option<SCard> {
        self.assert_invariant();
        self.next_no_invariant(card)
    }

    pub fn next_no_invariant(&self, card: SCard) -> Option<SCard> {
        let card_raw_next = self.mapcardcard_next[card];
        if_then_some!(card_raw_next!=card, card_raw_next)
    }

    pub fn prev(&self, card: SCard) -> Option<SCard> {
        self.assert_invariant();
        self.prev_no_invariant(card)
    }

    pub fn prev_no_invariant(&self, card: SCard) -> Option<SCard> {
        let card_raw_prev = self.mapcardcard_prev[card];
        if_then_some!(card_raw_prev!=card, card_raw_prev)
    }

    pub fn prev_while(&self, card: SCard, mut fn_pred: impl FnMut(SCard)->bool) -> SCard {
        self.assert_invariant();
        assert!(fn_pred(card));
        let mut card_out = card;
        while let Some(card_prev) = self.prev(card_out) {
            if fn_pred(card_prev) {
                card_out = card_prev;
            } else {
                break;
            }
        }
        card_out
    }
}

#[test]
fn test_cardspartition() {
    use crate::primitives::card::card_values::*;
    let mut cardspartition = SCardsPartition::new();
    cardspartition.chain(&[E7, E8, E9]);
    cardspartition.chain(&[SA, SZ, SK]);
    cardspartition.chain(&[EO, GO, HO, SO]);
    for (card, card_prev) in [
        (E8, E7), (E9, E8),
        (SZ, SA), (SK, SZ),
        (GO, EO), (HO, GO), (SO, HO),
    ].into_iter() {
        assert_eq!(cardspartition.prev(card), Some(card_prev));
    }
    for (card, card_prev_while) in [
        (E8, E7), (E9, E7),
        (SZ, SA), (SK, SA),
        (GO, EO), (HO, EO), (SO, EO),
    ].into_iter() {
        assert_eq!(cardspartition.prev_while(card, |_| true), card_prev_while);
    }
    for card in [GA, GZ, GK, G9].into_iter() {
        assert_eq!(cardspartition.prev(card), None);
        assert_eq!(cardspartition.prev_while(card, |_| true), card);
    }
    let mut cardspartition_2 = cardspartition.clone();
    let removed_1 = cardspartition_2.remove_from_chain(GZ);
    let removed_2 = cardspartition_2.remove_from_chain(EO);
    let removed_3 = cardspartition_2.remove_from_chain(HO);
    let removed_4 = cardspartition_2.remove_from_chain(SK);
    cardspartition_2.readd(removed_4);
    cardspartition_2.readd(removed_3);
    cardspartition_2.readd(removed_2);
    cardspartition_2.readd(removed_1);
    assert_eq!(&cardspartition, &cardspartition_2);
}
