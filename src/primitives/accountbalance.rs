// this stores the how much money each player currently has

use primitives::eplayerindex::*;

pub struct SAccountBalance {
    m_an : SPlayerIndexMap<isize>,
    m_n_stock : isize,
}

impl SAccountBalance {
    pub fn new(an: SPlayerIndexMap<isize>, n_stock: isize) -> SAccountBalance {
        let accountbalance = SAccountBalance {
            m_an : an,
            m_n_stock : n_stock,
        };
        accountbalance.assert_invariant();
        accountbalance
    }

    fn assert_invariant(&self) {
        assert_eq!(self.m_n_stock + self.m_an.iter().sum::<isize>(), 0);
    }

    pub fn apply_payout(&mut self, accountbalance: &SAccountBalance) {
        accountbalance.assert_invariant();
        self.assert_invariant();
        for eplayerindex in eplayerindex_values() {
            self.m_an[eplayerindex] = self.m_an[eplayerindex] + accountbalance.get_player(eplayerindex);
        }
        self.m_n_stock = self.m_n_stock + accountbalance.get_stock();
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

