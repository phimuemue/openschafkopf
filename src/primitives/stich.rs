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
pub struct SStich {
    pub m_eplayerindex_first: EPlayerIndex,
    m_veccard: ArrayVec<[SCard; 4]>,
}

impl PartialEq for SStich {
    fn eq(&self, stich_other: &SStich) -> bool {
        self.size()==stich_other.size()
        && self.equal_up_to_size(stich_other, self.size())
    }
}
impl Eq for SStich {}

pub struct StichIterator<'stich> {
    m_i_offset : usize,
    m_stich: &'stich SStich,
}

impl<'stich> Iterator for StichIterator<'stich> {
    type Item = (EPlayerIndex, SCard);
    fn next(&mut self) -> Option<(EPlayerIndex, SCard)> {
        if self.m_i_offset==self.m_stich.size() {
            return None;
        }
        else {
            let eplayerindex = (self.m_stich.m_eplayerindex_first + self.m_i_offset)%4;
            let pairicard = (eplayerindex, self.m_stich[eplayerindex]);
            self.m_i_offset = self.m_i_offset + 1;
            return Some(pairicard);
        }
    }
}

impl Index<EPlayerIndex> for SStich {
    type Output = SCard;
    fn index<'a>(&'a self, eplayerindex : EPlayerIndex) -> &'a SCard {
        assert!(self.valid_index(eplayerindex));
        &self.m_veccard[(eplayerindex+4-self.m_eplayerindex_first)%4] // TODO improve (possibly when EPlayerIndex is plain_enum)
    }
}

impl SStich {
    pub fn new(eplayerindex_first: EPlayerIndex) -> SStich {
        SStich {
            m_eplayerindex_first : eplayerindex_first,
            m_veccard: ArrayVec::new(),
        }
    }
    pub fn equal_up_to_size(&self, stich_other: &SStich, n_size: usize) -> bool {
        self.indices_and_cards()
            .zip(stich_other.indices_and_cards())
            .take(n_size)
            .all(|((i1, c1), (i2, c2))| i1==i2 && c1==c2)
    }
    pub fn empty(&self) -> bool {
        self.size() == 0
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
    pub fn zugeben(&mut self, card: SCard) {
        assert!(self.size()<4);
        self.m_veccard.push(card);
    }
    pub fn undo_most_recent_card(&mut self) {
        assert!(0 < self.size());
        self.m_veccard.pop();
    }

    pub fn first_card(&self) -> SCard {
        self[self.m_eplayerindex_first]
    }
    pub fn indices_and_cards(&self) -> StichIterator {
        StichIterator {
            m_i_offset: 0,
            m_stich: self
        }
    }
    fn valid_index(&self, eplayerindex: EPlayerIndex) -> bool {
        if eplayerindex >= self.m_eplayerindex_first {
            self.size() > eplayerindex-self.m_eplayerindex_first
        } else {
            self.size() > 4-self.m_eplayerindex_first+eplayerindex
        }
    }
    pub fn get(&self, eplayerindex: EPlayerIndex) -> Option<SCard> {
        if self.valid_index(eplayerindex) {
            Some(self[eplayerindex])
        } else {
            None
        }
    }
}

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
                    stich.zugeben(veccard[i_card]);
                }
                assert_eq!(stich.size(), n_size);
                assert_eq!(stich.first_player_index(), eplayerindex_first);
                assert_eq!(stich.size(), stich.indices_and_cards().count());
                for (eplayerindex, card) in stich.indices_and_cards() {
                    assert_eq!(stich.get(eplayerindex), Some(card));
                    assert_eq!(stich[eplayerindex], card);
                }
            }
        }
    }
    {
        let mut stich = SStich::new(2);
        stich.zugeben(SCard::new(EFarbe::Eichel, ESchlag::Unter));
        stich.zugeben(SCard::new(EFarbe::Gras, ESchlag::S7));
        assert!(stich[2]==SCard::new(EFarbe::Eichel, ESchlag::Unter));
        assert!(stich[3]==SCard::new(EFarbe::Gras, ESchlag::S7));
        assert_eq!(stich.indices_and_cards().count(), 2);
    }
}
