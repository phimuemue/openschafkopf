use std::ops;
use arrayvec::ArrayVec;

use std::ops::Index;

pub type EPlayerIndex = usize; // TODO plain_enum
pub type SPlayerIndexMap<T> = [T; 4]; // TODO plain_enum

pub fn eplayerindex_values() -> ops::Range<EPlayerIndex> { // TODO plain_enum
    0..4
}
pub fn eplayerindex_wrapping_add(eplayerindex: EPlayerIndex, n_offset: usize) -> EPlayerIndex { // TODO plain_enum
    (eplayerindex + n_offset) % 4
}

pub fn create_playerindexmap<T, F>(mut func: F) -> SPlayerIndexMap<T>
    where F:FnMut(EPlayerIndex)->T
{
    [func(0), func(1), func(2), func(3)]
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

pub struct SPlayersInRoundIterator<'playersinround, T> 
    where T: 'playersinround
{
    m_i_offset : usize,
    m_playersinround: &'playersinround SPlayersInRound<T>,
}

impl<'playersinround, T> Iterator for SPlayersInRoundIterator<'playersinround, T> {
    type Item = (EPlayerIndex, &'playersinround T);
    fn next(&mut self) -> Option<(EPlayerIndex, &'playersinround T)> {
        if self.m_i_offset==self.m_playersinround.size() {
            None
        }
        else {
            let eplayerindex = eplayerindex_wrapping_add(self.m_playersinround.m_eplayerindex_first, self.m_i_offset);
            let pairicard = (eplayerindex, &self.m_playersinround[eplayerindex]);
            self.m_i_offset = self.m_i_offset + 1;
            Some(pairicard)
        }
    }
}

impl<T> Index<EPlayerIndex> for SPlayersInRound<T> {
    type Output = T;
    fn index<'a>(&'a self, eplayerindex : EPlayerIndex) -> &'a T {
        assert!(self.valid_index(eplayerindex));
        &self.m_vect[(eplayerindex+4-self.m_eplayerindex_first)%4] // TODO improve (possibly when EPlayerIndex is plain_enum)
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
            Some(eplayerindex_wrapping_add(self.first_playerindex(), self.size()))
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
    pub fn iter(&self) -> SPlayersInRoundIterator<T> {
        SPlayersInRoundIterator {
            m_i_offset: 0,
            m_playersinround: self
        }
    }
    // TODO fn into_iter(self) -> SPlayersInRoundIntoIterator
    fn valid_index(&self, eplayerindex: EPlayerIndex) -> bool {
        if eplayerindex >= self.m_eplayerindex_first {
            self.size() > eplayerindex-self.m_eplayerindex_first
        } else {
            self.size() > 4-self.m_eplayerindex_first+eplayerindex
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
