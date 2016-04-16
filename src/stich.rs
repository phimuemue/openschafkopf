use card::*;
use std::fmt;
use std::mem;
use std::ptr;

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
pub struct CStich {
    pub m_eplayerindex_first: EPlayerIndex,
    m_n_size: usize,
    pub m_acard: [CCard; 4],
}

impl PartialEq for CStich {
    fn eq(&self, stich_other: &CStich) -> bool {
        self.size()==stich_other.size()
        && self.equal_up_to_size(stich_other, self.m_n_size)
    }
}
impl Eq for CStich {}

pub struct StichIterator<'stich> {
    m_eplayerindex : EPlayerIndex,
    m_stich: &'stich CStich,
}

impl<'stich> Iterator for StichIterator<'stich> {
    type Item = (EPlayerIndex, CCard);
    fn next(&mut self) -> Option<(EPlayerIndex, CCard)> {
        if self.m_eplayerindex==self.m_stich.size() {
            return None;
        }
        else {
            let eplayerindex = (self.m_stich.m_eplayerindex_first + self.m_eplayerindex)%4;
            let pairicard = (eplayerindex, self.m_stich[eplayerindex]);
            self.m_eplayerindex = self.m_eplayerindex + 1;
            return Some(pairicard);
        }
    }
}

impl Index<EPlayerIndex> for CStich {
    type Output = CCard;
    fn index<'a>(&'a self, eplayerindex : EPlayerIndex) -> &'a CCard {
        &self.m_acard[eplayerindex]
    }
}

impl CStich {
    pub fn new(eplayerindex_first: EPlayerIndex) -> CStich {
        CStich {
            m_eplayerindex_first : eplayerindex_first,
            m_n_size: 0,
            m_acard: [CCard::new(EFarbe::Eichel, ESchlag::S7); 4]
        }
    }
    pub fn equal_up_to_size(&self, stich_other: &CStich, n_size: usize) -> bool {
        self.indices_and_cards()
            .zip(stich_other.indices_and_cards())
            .take(n_size)
            .all(|((i1, c1), (i2, c2))| i1==i2 && c1==c2)
    }
    pub fn empty(&self) -> bool {
        self.m_n_size == 0
    }
    pub fn first_player_index(&self) -> EPlayerIndex {
        self.m_eplayerindex_first
    }
    pub fn current_player_index(&self) -> EPlayerIndex {
        (self.first_player_index() + self.size()) % 4
    }
    pub fn size(&self) -> usize {
        self.m_n_size
    }
    pub fn zugeben(&mut self, card: CCard) {
        assert!(self.m_n_size<4);
        let eplayerindex = (self.m_eplayerindex_first + self.m_n_size)%4;
        self.m_acard[eplayerindex] = card; // sad: can not inline eplayerindex (borrowing)
        self.m_n_size = self.m_n_size + 1;
    }
    pub fn undo_most_recent_card(&mut self) {
        assert!(0 < self.m_n_size);
        self.m_n_size = self.m_n_size - 1;
    }

    pub fn first_card(&self) -> CCard {
        self[self.m_eplayerindex_first]
    }
    pub fn indices_and_cards(&self) -> StichIterator {
        StichIterator {
            m_eplayerindex: 0,
            m_stich: self
        }
    }
    pub fn get(&self, eplayerindex: EPlayerIndex) -> Option<CCard> {
        // TODO: more elegance!
        for (eplayerindex_in_stich, card) in self.indices_and_cards() {
            if eplayerindex==eplayerindex_in_stich {
                return Some(card);
            }
        }
        None
    }
}

impl fmt::Display for CStich {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: more elegance!
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
    // TODO: use quicktest or similar and check proper retrieval
    for eplayerindex in 0..4 {
        let mut stich = CStich::new(eplayerindex);
        for i_size in 0..4 {
            stich.zugeben(CCard::new(EFarbe::Eichel, ESchlag::S7));
            assert_eq!(stich.size(), i_size+1);
        }
        assert_eq!(stich.first_player_index(), eplayerindex);
    }

    let mut stich = CStich::new(2);
    stich.zugeben(CCard::new(EFarbe::Eichel, ESchlag::Unter));
    stich.zugeben(CCard::new(EFarbe::Gras, ESchlag::S7));
    assert!(stich[2]==CCard::new(EFarbe::Eichel, ESchlag::Unter));
    assert!(stich[3]==CCard::new(EFarbe::Gras, ESchlag::S7));
    assert_eq!(stich.indices_and_cards().count(), 2);
}
