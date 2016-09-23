use primitives::card::*;
use std::fmt;
use std::mem;
use std::ptr;
use arrayvec::ArrayVec;

use std::ops::Index;

pub type EPlayerIndex = usize; // TODO: would a real enum be more adequate?

// TODO: introduce generic enummap
pub fn create_playerindexmap<T, F>(mut func: F) -> [T; 4]
    where F:FnMut(EPlayerIndex)->T
{
    let mut at : [T; 4];
    unsafe {
        at = mem::uninitialized();
        for eplayerindex in 0..4 {
            ptr::write(&mut at[eplayerindex], func(eplayerindex)); // ptr::write prevents dropping uninitialized memory
        }
    }
    at
}

#[derive(Clone)]
pub struct SPlayersInRound<T> {
    pub m_eplayerindex_first: EPlayerIndex,
    m_veccard: ArrayVec<[T; 4]>,
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
            return None;
        }
        else {
            let eplayerindex = (self.m_playersinround.m_eplayerindex_first + self.m_i_offset)%4;
            let pairicard = (eplayerindex, &self.m_playersinround[eplayerindex]);
            self.m_i_offset = self.m_i_offset + 1;
            return Some(pairicard);
        }
    }
}

impl<T> Index<EPlayerIndex> for SPlayersInRound<T> {
    type Output = T;
    fn index<'a>(&'a self, eplayerindex : EPlayerIndex) -> &'a T {
        assert!(self.valid_index(eplayerindex));
        &self.m_veccard[(eplayerindex+4-self.m_eplayerindex_first)%4] // TODO improve (possibly when EPlayerIndex is plain_enum)
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
            m_veccard: ArrayVec::new(),
        }
    }
    pub fn first_player_index(&self) -> EPlayerIndex {
        self.m_eplayerindex_first
    }
    pub fn current_player_index(&self) -> EPlayerIndex {
        (self.first_player_index() + self.size()) % 4
    }
    pub fn size(&self) -> usize {
        self.m_veccard.len()
    }
    pub fn push(&mut self, t: T) {
        assert!(self.size()<4);
        self.m_veccard.push(t);
    }
    pub fn undo_most_recent(&mut self) {
        assert!(0 < self.size());
        self.m_veccard.pop();
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

pub type SStich = SPlayersInRound<SCard>;

impl fmt::Display for SStich {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for eplayerindex in 0..4 {
            if eplayerindex==self.m_eplayerindex_first {
                try!(write!(f, ">"));
            } else {
                try!(write!(f, " "));
            }
            match self.get(eplayerindex) {
                None => {try!(write!(f, "__"));}
                Some(card) => {try!(write!(f, "{}", card));}
            }
        }
        write!(f, "")
    }
}

#[test]
fn test_stich() {
    // TODO? use quicktest or similar
    {
        use util::cardvectorparser;
        let veccard = cardvectorparser::parse_cards::<Vec<_>>("e7 e8 e9 ek").unwrap();
        for eplayerindex_first in 0..4 {
            for n_size in 0..5 {
                let mut stich = SStich::new(eplayerindex_first);
                for i_card in 0..n_size {
                    stich.push(veccard[i_card]);
                }
                assert_eq!(stich.size(), n_size);
                assert_eq!(stich.first_player_index(), eplayerindex_first);
                assert_eq!(stich.size(), stich.iter().count());
                for (eplayerindex, card) in stich.iter() {
                    assert_eq!(stich.get(eplayerindex), Some(card));
                    assert_eq!(stich[eplayerindex], *card);
                }
            }
        }
    }
    {
        let mut stich = SStich::new(2);
        stich.push(SCard::new(EFarbe::Eichel, ESchlag::Unter));
        stich.push(SCard::new(EFarbe::Gras, ESchlag::S7));
        assert!(stich[2]==SCard::new(EFarbe::Eichel, ESchlag::Unter));
        assert!(stich[3]==SCard::new(EFarbe::Gras, ESchlag::S7));
        assert_eq!(stich.iter().count(), 2);
    }
}
