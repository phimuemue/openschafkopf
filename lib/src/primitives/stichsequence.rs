use crate::util::*;
use arrayvec::ArrayVec;
use std::fmt;
use super::*;
use itertools::Itertools;

pub trait TWinnerIndex {
    fn winner_index(&self, stich: SFullStich<&SStich>) -> EPlayerIndex;
}

#[cfg(test)]
pub struct SWinnerIndexIrrelevant;
#[cfg(test)]
impl TWinnerIndex for SWinnerIndexIrrelevant {
    fn winner_index(&self, _stich: SFullStich<&SStich>) -> EPlayerIndex {
        EPlayerIndex::EPI0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)] // TODO? custom impl Debug
pub struct SStichSequence {
    vecstich: ArrayVec<SStich, {EKurzLang::max_cards_per_player()+/*surrogate stich holds winner index for last real stich*/1}>,
    ekurzlang: EKurzLang,
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

impl std::fmt::Display for SStichSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for stich in self.completed_stichs() {
            write!(f, "{} | ", stich)?;
        }
        write!(f, "{}", self.current_stich())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SDuplicateCard(pub ECard);

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

    pub fn new_from_cards(ekurzlang: EKurzLang, mut itcard: impl Iterator<Item=ECard>, winnerindex: &(impl TWinnerIndex + ?Sized)) -> Result<Self, SDuplicateCard> {
        let mut setcard = EnumSet::new_empty();
        itcard.try_fold(Self::new(ekurzlang), |mut stichseq, card| {
            if setcard.insert(card) {
                stichseq.zugeben(card, winnerindex);
                Ok(stichseq)
            } else {
                Err(SDuplicateCard(card))
            }
        })
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

    pub fn last_completed_stich(&self) -> Option<SFullStich<&SStich>> {
        self.completed_stichs().last().map(SFullStich::new)
    }

    fn current_stich_no_invariant(&self) -> &SStich {
        unwrap!(self.vecstich.last())
    }

    pub fn current_stich(&self) -> &SStich {
        #[cfg(debug_assertions)]self.assert_invariant();
        self.current_stich_no_invariant()
    }

    pub fn current_playable_stich(&self) -> &SStich {
        #[cfg(debug_assertions)]self.assert_invariant();
        assert!(self.completed_stichs().len()<self.kurzlang().cards_per_player());
        self.current_stich()
    }

    pub fn completed_stichs_winner_index<'lifetime>(&'lifetime self, if_dbg_else!({winnerindex}{_}): &'lifetime(impl TWinnerIndex + ?Sized)) -> impl Iterator<Item=(SFullStich<&'lifetime SStich>, EPlayerIndex)> + 'lifetime {
        #[cfg(debug_assertions)]self.assert_invariant();
        self.vecstich[0..self.vecstich.len()]
            .iter()
            .tuple_windows()
            .map(move |(stich_0, stich_1)| {
                let stich_0 = SFullStich::new(stich_0);
                (stich_0, debug_verify_eq!(stich_1.first_playerindex(), winnerindex.winner_index(stich_0)))
            })
    }

    pub fn zugeben(&mut self, card: ECard, winnerindex: &(impl TWinnerIndex + ?Sized)) {
        #[cfg(debug_assertions)]self.assert_invariant();
        unwrap!(self.vecstich.last_mut()).push(card);
        if self.current_stich_no_invariant().is_full() {
            self.vecstich.push(SStich::new(winnerindex.winner_index(SFullStich::new(self.current_stich_no_invariant()))));
        }
        #[cfg(debug_assertions)]self.assert_invariant();
    }

    pub fn zugeben_and_restore<R>(&mut self, card: ECard, winnerindex: &(impl TWinnerIndex + ?Sized), func: impl FnOnce(&mut Self)->R) -> R {
        #[cfg(debug_assertions)]self.assert_invariant();
        let n_len = self.vecstich.len();
        assert!(!self.current_stich().is_full());
        self.zugeben(card, winnerindex);
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

    pub fn zugeben_and_restore_with_hands<R>(&mut self, ahand: &mut EnumMap<EPlayerIndex, SHand>, epi: EPlayerIndex, card: ECard, winnerindex: &(impl TWinnerIndex + ?Sized), func: impl FnOnce(&mut EnumMap<EPlayerIndex, SHand>, &mut Self)->R) -> R {
        ahand[epi].play_card(card);
        let r = self.zugeben_and_restore(
            card,
            winnerindex,
            |stichseq| func(ahand, stichseq),
        );
        ahand[epi].add_card(card);
        r
    }

    pub fn visible_stichs(&self) -> &[SStich] {
        &self.vecstich[0..self.vecstich.len().min(self.ekurzlang.cards_per_player())]
    }
    
    pub fn visible_cards(&self) -> impl Iterator<Item=(EPlayerIndex, &ECard)> {
        self.visible_stichs().iter().flat_map(SStich::iter)
    }

    pub fn completed_cards(&self) -> impl Iterator<Item=(EPlayerIndex, &ECard)> {
        self.completed_stichs().iter().flat_map(SStich::iter)
    }

    pub fn completed_cards_by(&self, epi: EPlayerIndex) -> impl DoubleEndedIterator<Item=ECard> + Clone + '_ {
        self.completed_stichs().iter().map(move |stich| SFullStich::new(stich)[epi])
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

    pub fn cards_from_player<'slf>(&'slf self, hand: &'slf SHand, epi: EPlayerIndex) -> impl Iterator<Item=ECard> + 'slf + Clone {
        let itcard_played = self.completed_cards_by(epi)
            .chain(self.current_stich().get(epi).copied());
        debug_assert!(itertools::equal(
            itcard_played.clone(),
            self.visible_cards()
                .filter_map(move |(epi_card, card)| if_then_some!(epi==epi_card, card))
                .copied()
        ));
        itcard_played
            .chain(hand.cards().iter().copied())
    }
}
