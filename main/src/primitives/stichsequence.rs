use crate::util::*;
use arrayvec::ArrayVec;
use std::fmt;
use super::*;
use itertools::Itertools;

pub trait TWinnerIndex {
    fn winner_index(&self, stich: &SStich) -> EPlayerIndex {
        assert!(stich.is_full());
        self.winner_index_no_invariant(stich)
    }
    fn winner_index_no_invariant(&self, stich: &SStich) -> EPlayerIndex;
}

#[derive(Debug, Clone, PartialEq, Eq)] // TODO? custom impl Debug
pub struct SStichSequence {
    vecstich: ArrayVec<SStich, /*TODO: can this be bound to EKurzLang somehow?*/9>,
    ekurzlang: EKurzLang,
}

impl std::fmt::Display for SStichSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for stich in self.completed_stichs() {
            write!(f, "{} | ", stich)?;
        }
        write!(f, "{}", self.current_stich())?;
        Ok(())
    }
}

impl SStichSequence { // TODO implement wrappers for SStichSequence that allow only zugeben_and_restore and no other manipulations and use them where applicable. My first try ran into some lifetime issues.
    #[cfg(debug_assertions)]
    fn assert_invariant(&self) {
        assert!(!self.vecstich.is_empty());
        assert_eq!(self.vecstich[0].first_playerindex(), EPlayerIndex::EPI0);
        assert!(!self.current_stich_no_invariant().is_full());
        assert_eq!(self.vecstich[0..self.vecstich.len()-1].len(), self.vecstich.len()-1);
        assert!(self.vecstich[0..self.vecstich.len()-1].iter().all(SStich::is_full));
        assert!(self.completed_stichs_no_invariant().len()<=self.ekurzlang.cards_per_player());
        if self.completed_stichs_no_invariant().len()==self.ekurzlang.cards_per_player() {
            assert!(self.current_stich_no_invariant().is_empty());
        }
    }

    pub fn new(ekurzlang: EKurzLang) -> Self {
        let stichseq = SStichSequence {
            vecstich: {
                let mut vecstich = ArrayVec::new();
                vecstich.push(SStich::new(EPlayerIndex::EPI0));
                vecstich
            },
            ekurzlang,
        };
        #[cfg(debug_assertions)]stichseq.assert_invariant();
        stichseq
    }

    pub fn new_from_cards(ekurzlang: EKurzLang, itcard: impl Iterator<Item=SCard>, rules: &(impl TWinnerIndex + ?Sized)) -> Self {
        itcard.fold(Self::new(ekurzlang), mutate_return!(|stichseq, card| {
            stichseq.zugeben(card, rules);
        }))
    }

    pub fn game_finished(&self) -> bool {
        #[cfg(debug_assertions)]self.assert_invariant();
        assert!(self.completed_stichs().len()<=self.ekurzlang.cards_per_player());
        self.completed_stichs().len()==self.ekurzlang.cards_per_player()
    }

    pub fn no_card_played(&self) -> bool {
        #[cfg(debug_assertions)]self.assert_invariant();
        self.completed_stichs().is_empty() && self.current_stich().is_empty()
    }

    fn completed_stichs_no_invariant(&self) -> &[SStich] {
        &self.vecstich[0..self.vecstich.len()-1]
    }

    pub fn completed_stichs(&self) -> &[SStich] {
        #[cfg(debug_assertions)]self.assert_invariant();
        self.completed_stichs_no_invariant()
    }

    fn current_stich_no_invariant(&self) -> &SStich {
        unwrap!(self.vecstich.last())
    }

    pub fn current_stich(&self) -> &SStich {
        #[cfg(debug_assertions)]self.assert_invariant();
        self.current_stich_no_invariant()
    }

    pub fn zugeben_custom_winner_index(&mut self, card: SCard, fn_winner_index: impl FnOnce(&SStich)->EPlayerIndex) {
        #[cfg(debug_assertions)]self.assert_invariant();
        unwrap!(self.vecstich.last_mut()).push(card);
        if self.current_stich_no_invariant().is_full() {
            self.vecstich.push(SStich::new(fn_winner_index(self.current_stich_no_invariant())));
        }
        #[cfg(debug_assertions)]self.assert_invariant();
    }

    pub fn completed_stichs_custom_winner_index(&self, if_dbg_else!({fn_winner_index}{_fn_winner_index}): impl Fn(&SStich)->EPlayerIndex) -> impl Iterator<Item=(&SStich, EPlayerIndex)> {
        #[cfg(debug_assertions)]self.assert_invariant();
        self.vecstich[0..self.vecstich.len()]
            .iter()
            .tuple_windows()
            .map(move |(stich_0, stich_1)| {
                (stich_0, debug_verify_eq!(stich_1.first_playerindex(), fn_winner_index(stich_0)))
            })
    }

    pub fn completed_stichs_winner_index<'lifetime>(&'lifetime self, rules: &'lifetime(impl TWinnerIndex + ?Sized)) -> impl Iterator<Item=(&'lifetime SStich, EPlayerIndex)> + 'lifetime {
        self.completed_stichs_custom_winner_index(move |stich| rules.winner_index(stich))
    }

    pub fn zugeben(&mut self, card: SCard, rules: &(impl TWinnerIndex + ?Sized)) {
        self.zugeben_custom_winner_index(card, |stich| rules.winner_index(stich));
    }

    pub fn zugeben_and_restore<R>(&mut self, card: SCard, rules: &(impl TWinnerIndex + ?Sized), func: impl FnOnce(&mut Self)->R) -> R {
        #[cfg(debug_assertions)]self.assert_invariant();
        let n_len = self.vecstich.len();
        assert!(!self.current_stich().is_full());
        self.zugeben(card, rules);
        let r = func(self);
        #[cfg(debug_assertions)]self.assert_invariant();
        if self.current_stich().is_empty() {
            unwrap!(self.vecstich.pop());
            assert!(self.current_stich_no_invariant().is_full());
        }
        unwrap!(self.vecstich.last_mut()).undo_most_recent();
        debug_assert_eq!(n_len, self.vecstich.len());
        #[cfg(debug_assertions)]self.assert_invariant();
        r
    }

    pub fn visible_stichs(&self) -> &[SStich] {
        &self.vecstich[0..self.vecstich.len().min(self.ekurzlang.cards_per_player())]
    }
    
    pub fn visible_cards(&self) -> impl Iterator<Item=(EPlayerIndex, &SCard)> {
        self.visible_stichs().iter().flat_map(SStich::iter)
    }

    pub fn completed_cards(&self) -> impl Iterator<Item=(EPlayerIndex, &SCard)> {
        self.completed_stichs().iter().flat_map(SStich::iter)
    }

    pub fn kurzlang(&self) -> EKurzLang {
        #[cfg(debug_assertions)]self.assert_invariant();
        self.ekurzlang
    }

    pub fn count_played_cards(&self) -> usize {
        #[cfg(debug_assertions)]self.assert_invariant();
        self.completed_stichs().len() * EPlayerIndex::SIZE
            + self.current_stich().size()
    }


    pub fn remaining_cards_per_hand(&self) -> EnumMap<EPlayerIndex, usize> {
        EPlayerIndex::map_from_fn(|epi| {
            self.kurzlang().cards_per_player()
                - self.completed_stichs().len()
                - match self.current_stich().get(epi) {
                    None => 0,
                    Some(_card) => 1,
                }
        })
    }

    pub fn cards_from_player<'slf>(&'slf self, hand: &'slf SHand, epi: EPlayerIndex) -> impl Iterator<Item=&'slf SCard> {
        self.visible_cards()
            .filter_map(move |(epi_card, card)| if_then_some!(epi==epi_card, card))
            .chain(hand.cards().iter())
    }
}