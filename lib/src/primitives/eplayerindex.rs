use crate::util::*;
use arrayvec::{self, ArrayVec};
use std::{fmt, slice, str::FromStr};
use serde_repr::Serialize_repr;

plain_enum_mod!(modepi, derive(Serialize_repr, Hash,), map_derive(), EPlayerIndex {
    EPI0, EPI1, EPI2, EPI3,
});
define_static_value!(pub SStaticEPI0, EPlayerIndex, EPlayerIndex::EPI0);
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

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct SPlayersInRound<T, PlayerIndex: TStaticOrDynamicValue<EPlayerIndex>> {
    pub epi_first: PlayerIndex,
    vect: ArrayVec<T, {EPlayerIndex::SIZE}>,
}

impl<T: fmt::Debug, PlayerIndex: Copy+TStaticOrDynamicValue<EPlayerIndex>> fmt::Debug for SPlayersInRound<T, PlayerIndex> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for epi in EPlayerIndex::values() {
            if epi==self.epi_first.value() {
                write!(f, ">")?;
            } else {
                write!(f, " ")?;
            }
            match self.get(epi) {
                None => {write!(f, "__")?;}
                Some(t) => {write!(f, "{t:?}")?;}
            }
        }
        write!(f, "")
    }
}

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
        debug_assert_eq!(self.position(self.first_playerindex()), 0);
        unwrap!(self.vect.first())
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
    pub fn get(&self, epi: EPlayerIndex) -> Option<&T> {
        self.vect.get(self.position(epi))
    }
}

impl<T, PlayerIndex: TStaticOrDynamicValue<EPlayerIndex>> IntoIterator for SPlayersInRound<T, PlayerIndex> {
    type Item = (EPlayerIndex, T);
    type IntoIter = SPlayersInRoundIterator<arrayvec::IntoIter<T, {EPlayerIndex::SIZE}>>;
    fn into_iter(self) -> Self::IntoIter {
        SPlayersInRoundIterator {
            iter: self.vect.into_iter(),
            n_epi: self.epi_first.value().to_usize(),
        }
    }
}
