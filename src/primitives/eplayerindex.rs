use arrayvec;
use arrayvec::ArrayVec;

use std::ops::Index;
use std::fmt;
use util::*;
use std::str::FromStr;
use std::slice;

plain_enum_mod!(modepi, EPlayerIndex {
    EPI0, EPI1, EPI2, EPI3,
});
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

#[derive(Clone)]
pub struct SPlayersInRound<T> {
    pub m_epi_first: EPlayerIndex,
    m_vect: ArrayVec<[T; 4]>,
}

impl<T> PartialEq for SPlayersInRound<T>
    where T: PartialEq,
{
    fn eq(&self, playersinround_other: &SPlayersInRound<T>) -> bool {
        self.size()==playersinround_other.size()
        && self.equal_up_to_size(playersinround_other, self.size())
    }
}
impl<T> Eq for SPlayersInRound<T> 
    where T: Eq,
{}

pub struct SPlayersInRoundIterator<InternalIter>
    where InternalIter: Iterator,
{
    m_iter: InternalIter,
    m_n_epi: usize,
}

impl<InternalIter> Iterator for SPlayersInRoundIterator<InternalIter>
    where InternalIter: Iterator,
{
    type Item = (EPlayerIndex, InternalIter::Item);
    fn next(&mut self) -> Option<(EPlayerIndex, InternalIter::Item)> {
        let item_next = self.m_iter.next()
            .map(|t| (EPlayerIndex::wrapped_from_usize(self.m_n_epi), t));
        self.m_n_epi += 1;
        item_next
    }
}

impl<T> Index<EPlayerIndex> for SPlayersInRound<T> {
    type Output = T;
    fn index(&self, epi : EPlayerIndex) -> &T {
        assert!(self.valid_index(epi));
        &self.m_vect[epi.wrapped_difference(self.m_epi_first)]
    }
}

impl<T> SPlayersInRound<T>
    where T: PartialEq,
{
    pub fn equal_up_to_size(&self, playersinround_other: &SPlayersInRound<T>, n_size: usize) -> bool {
        self.iter()
            .zip(playersinround_other.iter())
            .take(n_size)
            .all(|((i1, c1), (i2, c2))| i1==i2 && c1==c2)
    }
}

impl<T> SPlayersInRound<T> {
    pub fn new(epi_first: EPlayerIndex) -> SPlayersInRound<T> {
        SPlayersInRound {
            m_epi_first : epi_first,
            m_vect: ArrayVec::new(),
        }
    }
    pub fn first_playerindex(&self) -> EPlayerIndex {
        self.m_epi_first
    }
    pub fn current_playerindex(&self) -> Option<EPlayerIndex> {
        if self.size()==4 {
            None
        } else {
            Some(self.first_playerindex().wrapping_add(self.size()))
        }
    }
    pub fn size(&self) -> usize {
        self.m_vect.len()
    }
    pub fn push(&mut self, t: T) {
        assert!(self.size()<4);
        self.m_vect.push(t);
    }
    pub fn undo_most_recent(&mut self) {
        assert!(0 < self.size());
        self.m_vect.pop();
    }

    pub fn first(&self) -> &T {
        assert!(0 < self.size());
        &self[self.m_epi_first]
    }
    pub fn iter(&self) -> SPlayersInRoundIterator<slice::Iter<T>> {
        SPlayersInRoundIterator {
            m_iter: self.m_vect.iter(),
            m_n_epi: self.m_epi_first.to_usize(),
        }
    }
    pub fn into_iter(self) -> SPlayersInRoundIterator<arrayvec::IntoIter<[T; 4]>> {
        SPlayersInRoundIterator {
            m_iter: self.m_vect.into_iter(),
            m_n_epi: self.m_epi_first.to_usize(),
        }
    }
    fn valid_index(&self, epi: EPlayerIndex) -> bool {
        if epi >= self.m_epi_first {
            self.size() > epi.to_usize()-self.m_epi_first.to_usize()
        } else {
            self.size() > 4-self.m_epi_first.to_usize()+epi.to_usize()
        }
    }
    pub fn get(&self, epi: EPlayerIndex) -> Option<&T> {
        if self.valid_index(epi) {
            Some(&self[epi])
        } else {
            None
        }
    }
}
