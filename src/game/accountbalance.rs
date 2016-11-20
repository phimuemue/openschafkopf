// this stores the how much money each player currently has

use primitives::eplayerindex::*;

pub struct SAccountBalance {
    m_an : SPlayerIndexMap<isize>,
    m_n_stock : isize,
}

impl SAccountBalance {
    pub fn new() -> SAccountBalance {
        SAccountBalance {
            m_an : [0, 0, 0, 0],
            m_n_stock : 0,
        }
    }

    fn assert_invariant(&self) {
        assert_eq!(self.m_n_stock + self.m_an.iter().sum::<isize>(), 0);
    }

    pub fn apply_payout(&mut self, an_payout: &SPlayerIndexMap<isize>) {
        self.assert_invariant();
        for eplayerindex in eplayerindex_values() {
            self.m_an[eplayerindex] = self.m_an[eplayerindex] + an_payout[eplayerindex];
        }
        self.assert_invariant();
    }

    pub fn get_player(&self, eplayerindex : EPlayerIndex) -> isize {
        self.assert_invariant();
        self.m_an[eplayerindex]
    }

    pub fn get_stock(&self) -> isize {
        self.assert_invariant();
        self.m_n_stock
    }
}

