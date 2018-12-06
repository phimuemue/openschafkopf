// this stores the how much money each player currently has

use crate::primitives::eplayerindex::*;
use crate::util::*;

#[derive(Debug)]
pub struct SAccountBalance {
    an : EnumMap<EPlayerIndex, isize>,
    n_stock : isize,
}

impl SAccountBalance {
    pub fn new(an: EnumMap<EPlayerIndex, isize>, n_stock: isize) -> SAccountBalance {
        let accountbalance = SAccountBalance {
            an,
            n_stock,
        };
        accountbalance.assert_invariant();
        accountbalance
    }

    fn assert_invariant(&self) {
        assert_eq!(self.n_stock + self.an.iter().sum::<isize>(), 0);
    }

    pub fn apply_payout(&mut self, accountbalance: &SAccountBalance) {
        accountbalance.assert_invariant();
        self.assert_invariant();
        for epi in EPlayerIndex::values() {
            self.an[epi] += accountbalance.get_player(epi);
        }
        self.n_stock += accountbalance.get_stock();
        self.assert_invariant();
    }

    pub fn get_player(&self, epi : EPlayerIndex) -> isize {
        self.assert_invariant();
        self.an[epi]
    }

    pub fn get_stock(&self) -> isize {
        self.assert_invariant();
        self.n_stock
    }
}

