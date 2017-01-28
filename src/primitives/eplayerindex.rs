use arrayvec::ArrayVec;

use std::ops::Index;
use std::fmt;
use util::*;
use std::str::FromStr;
use std::slice;

plain_enum_mod!(modeplayerindex, EPlayerIndex {
    EPI0, EPI1, EPI2, EPI3,
});
impl fmt::Display for EPlayerIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_usize())
    }
}
impl FromStr for EPlayerIndex {
    type Err = &'static str;
    fn from_str(str_eplayerindex: &str) -> Result<Self, Self::Err> {
        usize::from_str(str_eplayerindex).ok()
            .and_then(|n_eplayerindex| {
                EPlayerIndex::checked_from_usize(n_eplayerindex)
            })
            .ok_or("Could not convert to EPlayerIndex")
    }
}

#[derive(Clone)]
pub struct SPlayersInRound<T> {
    pub m_eplayerindex_first: EPlayerIndex,
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
    m_n_eplayerindex: usize,
}

impl<InternalIter> Iterator for SPlayersInRoundIterator<InternalIter>
    where InternalIter: Iterator,
{
    type Item = (EPlayerIndex, InternalIter::Item);
    fn next(&mut self) -> Option<(EPlayerIndex, InternalIter::Item)> {
        let item_next = self.m_iter.next()
            .map(|t| (EPlayerIndex::wrapped_from_usize(self.m_n_eplayerindex), t));
        self.m_n_eplayerindex = self.m_n_eplayerindex+1;
        item_next
    }
}

impl<T> Index<EPlayerIndex> for SPlayersInRound<T> {
    type Output = T;
    fn index(&self, eplayerindex : EPlayerIndex) -> &T {
        assert!(self.valid_index(eplayerindex));
        &self.m_vect[eplayerindex.wrapped_difference(self.m_eplayerindex_first)]
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
    pub fn new(eplayerindex_first: EPlayerIndex) -> SPlayersInRound<T> {
        SPlayersInRound {
            m_eplayerindex_first : eplayerindex_first,
            m_vect: ArrayVec::new(),
        }
    }
    pub fn first_playerindex(&self) -> EPlayerIndex {
        self.m_eplayerindex_first
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
        &self[self.m_eplayerindex_first]
    }
    pub fn iter(&self) -> SPlayersInRoundIterator<slice::Iter<T>> {
        SPlayersInRoundIterator {
            m_iter: self.m_vect.iter(),
            m_n_eplayerindex: self.m_eplayerindex_first.to_usize(),
        }
    }
    fn valid_index(&self, eplayerindex: EPlayerIndex) -> bool {
        if eplayerindex >= self.m_eplayerindex_first {
            self.size() > eplayerindex.to_usize()-self.m_eplayerindex_first.to_usize()
        } else {
            self.size() > 4-self.m_eplayerindex_first.to_usize()+eplayerindex.to_usize()
        }
    }
    pub fn get(&self, eplayerindex: EPlayerIndex) -> Option<&T> {
        if self.valid_index(eplayerindex) {
            Some(&self[eplayerindex])
        } else {
            None
        }
    }
}
