// this stores the how much money each player currently has

use crate::primitives::eplayerindex::*;
use crate::util::*;

#[derive(Debug)]
pub struct SAccountBalance {
    an : EnumMap<EPlayerIndex, isize>,
    n_stock : isize,
}

impl SAccountBalance {
    pub fn new(an: EnumMap<EPlayerIndex, isize>) -> SAccountBalance {
        let n_stock = verify_eq!(0, an.iter().sum::<isize>());
        let accountbalance = SAccountBalance {
            an,
            n_stock,
        };
        accountbalance.assert_invariant();
        accountbalance
    }

    fn assert_invariant(&self) {
        assert!(0 <= self.n_stock);
        assert_eq!(self.n_stock + self.an.iter().sum::<isize>(), 0);
    }

    pub fn apply_payout(&mut self, an: &EnumMap<EPlayerIndex, isize>) {
        for epi in EPlayerIndex::values() {
            self.an[epi] += an[epi];
        }
        let n_pay_into_stock = -an.iter().sum::<isize>();
        assert!(
            n_pay_into_stock >= 0 // either pay into stock...
            || n_pay_into_stock == -self.n_stock // ... or exactly empty it (assume that this is always possible)
        );
        self.n_stock += n_pay_into_stock;
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

