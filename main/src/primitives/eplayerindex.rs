use crate::util::*;
use arrayvec::{self, ArrayVec};
use std::{fmt, ops::Index, slice, str::FromStr};
use serde_repr::Serialize_repr;

plain_enum_mod!(modepi, derive(Serialize_repr,), map_derive(), EPlayerIndex {
    EPI0, EPI1, EPI2, EPI3,
});
define_static_value!(pub SStaticEPI0, EPlayerIndex, EPlayerIndex::EPI0);
define_static_value!(pub SStaticEPI1, EPlayerIndex, EPlayerIndex::EPI1);
define_static_value!(pub SStaticEPI2, EPlayerIndex, EPlayerIndex::EPI2);
define_static_value!(pub SStaticEPI3, EPlayerIndex, EPlayerIndex::EPI3);
impl fmt::Display for EPlayerIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_usize())
    }
}
impl FromStr for EPlayerIndex {
    type Err = &'static str;
    fn from_str(str_epi: &str) -> Result<Self, Self::Err> {
        usize::from_str(str_epi).ok()
            .and_then(|n_epi| {
                EPlayerIndex::checked_from_usize(n_epi)
            })
            .ok_or("Could not convert to EPlayerIndex")
    }
}

#[derive(Clone, Debug)]
pub struct SPlayersInRound<T, PlayerIndex: TStaticOrDynamicValue<EPlayerIndex>> {
    pub epi_first: PlayerIndex,
    vect: ArrayVec<[T; EPlayerIndex::SIZE]>,
}

impl<T: PartialEq, PlayerIndex: TStaticOrDynamicValue<EPlayerIndex>+Copy> PartialEq for SPlayersInRound<T, PlayerIndex> {
    fn eq(&self, playersinround_other: &Self) -> bool {
        self.size()==playersinround_other.size()
        && self.equal_up_to_size(playersinround_other, self.size())
    }
}
impl<T: Eq, PlayerIndex: TStaticOrDynamicValue<EPlayerIndex>+Copy> Eq for SPlayersInRound<T, PlayerIndex>{}

pub struct SPlayersInRoundIterator<InternalIter> {
    iter: InternalIter,
    n_epi: usize,
}

impl<InternalIter: Iterator> Iterator for SPlayersInRoundIterator<InternalIter> {
    type Item = (EPlayerIndex, InternalIter::Item);
    fn next(&mut self) -> Option<(EPlayerIndex, InternalIter::Item)> {
        let item_next = self.iter.next()
            .map(|t| (EPlayerIndex::wrapped_from_usize(self.n_epi), t));
        self.n_epi += 1;
        item_next
    }
}

impl<T, PlayerIndex: TStaticOrDynamicValue<EPlayerIndex>+Copy> Index<EPlayerIndex> for SPlayersInRound<T, PlayerIndex> {
    type Output = T;
    fn index(&self, epi : EPlayerIndex) -> &T {
        assert!(self.valid_index(epi));
        &self.vect[self.position(epi)]
    }
}

impl<T: PartialEq, PlayerIndex: TStaticOrDynamicValue<EPlayerIndex>+Copy> SPlayersInRound<T, PlayerIndex> {
    pub fn equal_up_to_size(&self, playersinround_other: &SPlayersInRound<T, PlayerIndex>, n_size: usize) -> bool {
        self.iter()
            .zip(playersinround_other.iter())
            .take(n_size)
            .all(|((i1, c1), (i2, c2))| i1==i2 && c1==c2)
    }
}

impl<T, PlayerIndex: TStaticOrDynamicValue<EPlayerIndex>+Copy> SPlayersInRound<T, PlayerIndex> {
    pub fn new(epi_first: PlayerIndex) -> Self {
        SPlayersInRound {
            epi_first,
            vect: ArrayVec::new(),
        }
    }
    pub fn new_full(epi_first: PlayerIndex, at: [T; EPlayerIndex::SIZE]) -> Self {
        SPlayersInRound {
            epi_first,
            vect: ArrayVec::from(at),
        }
    }
    pub fn first_playerindex(&self) -> EPlayerIndex {
        self.epi_first.value()
    }
    pub fn current_playerindex(&self) -> Option<EPlayerIndex> {
        if_then_some!(
            !self.is_full(),
            self.first_playerindex().wrapping_add(self.size())
        )
    }
    pub fn size(&self) -> usize {
        self.vect.len()
    }
    pub fn is_full(&self) -> bool {
        self.size()==EPlayerIndex::SIZE
    }
    pub fn is_empty(&self) -> bool {
        self.size()==0
    }
    pub fn push(&mut self, t: T) {
        assert!(!self.is_full());
        self.vect.push(t);
    }
    pub fn undo_most_recent(&mut self) {
        assert!(!self.is_empty());
        self.vect.pop();
    }

    pub fn first(&self) -> &T {
        assert!(!self.is_empty());
        &self[self.epi_first.value()]
    }
    pub fn iter(&self) -> SPlayersInRoundIterator<slice::Iter<T>> {
        SPlayersInRoundIterator {
            iter: self.vect.iter(),
            n_epi: self.epi_first.value().to_usize(),
        }
    }
    fn position(&self, epi: EPlayerIndex) -> usize {
        epi.wrapped_difference_usize(self.epi_first.value())
    }
    fn valid_index(&self, epi: EPlayerIndex) -> bool {
        self.position(epi)<self.size()
    }
    pub fn get(&self, epi: EPlayerIndex) -> Option<&T> {
        if_then_some!(self.valid_index(epi), &self[epi])
    }
}

impl<T, PlayerIndex: TStaticOrDynamicValue<EPlayerIndex>> IntoIterator for SPlayersInRound<T, PlayerIndex> {
    type Item = (EPlayerIndex, T);
    type IntoIter = SPlayersInRoundIterator<arrayvec::IntoIter<[T; EPlayerIndex::SIZE]>>;
    fn into_iter(self) -> Self::IntoIter {
        SPlayersInRoundIterator {
            iter: self.vect.into_iter(),
            n_epi: self.epi_first.value().to_usize(),
        }
    }
}
