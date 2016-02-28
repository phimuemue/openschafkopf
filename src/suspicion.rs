use card::*;
use stich::*;
use hand::*;
use rules::*;
use std::cmp;

struct SSuspicionTransition {
    m_stich : CStich,
    m_susp : SSuspicion,
}

impl SSuspicionTransition {
    fn new(susp: &SSuspicion, stich: CStich, rules: &TRules) -> SSuspicionTransition {
        let susp = SSuspicion::new_from_susp(susp, &stich, rules);
        SSuspicionTransition {
            m_stich : stich,
            m_susp : susp
        }
    }

    fn print_suspiciontransition(&self, n_maxlevel: usize, n_level: usize, rules: &TRules, vecstich: &mut Vec<CStich>) {
        if n_level<=n_maxlevel {
            vecstich.push(self.m_stich.clone());
            for _ in 0..n_level+1 {
                print!(" ");
            }
            print!("{} : ", self.m_stich);
            if 1<self.m_susp.hand_size() {
                self.m_susp.print_suspicion(n_maxlevel, n_level, rules, vecstich);
            } else {
                println!("");
            }
            vecstich.pop().expect("vecstich empty");
        }
    }
}

pub struct SSuspicion {
    m_vecsusptrans : Vec<SSuspicionTransition>,
    m_eplayerindex_first : EPlayerIndex,
    m_ahand : [CHand; 4],
}

impl SSuspicion {

    pub fn new_from_raw(eplayerindex_first: EPlayerIndex, ahand: &[CHand; 4]) -> Self {
        SSuspicion {
            m_vecsusptrans: Vec::new(),
            m_eplayerindex_first : eplayerindex_first,
            m_ahand : [
                ahand[0].clone(),
                ahand[1].clone(),
                ahand[2].clone(),
                ahand[3].clone(),
            ]
        }
    }

    fn new_from_susp(&self, stich: &CStich, rules: &TRules) -> Self {
        SSuspicion {
            m_vecsusptrans: Vec::new(),
            m_eplayerindex_first : rules.winner_index(stich),
            m_ahand : [
                self.m_ahand[0].new_from_hand(stich[0]),
                self.m_ahand[1].new_from_hand(stich[1]),
                self.m_ahand[2].new_from_hand(stich[2]),
                self.m_ahand[3].new_from_hand(stich[3]),
            ]
        }
    }

    fn hand_size(&self) -> usize {
        assert_eq!(self.m_ahand[0].cards().len(), self.m_ahand[1].cards().len());
        assert_eq!(self.m_ahand[0].cards().len(), self.m_ahand[2].cards().len());
        assert_eq!(self.m_ahand[0].cards().len(), self.m_ahand[3].cards().len());
        self.m_ahand[0].cards().len()
    }

    pub fn compute_successors(&mut self, rules: &TRules) {
        self.internal_compute_successors(rules, &mut Vec::new())
    }

    fn internal_compute_successors(&mut self, rules: &TRules, vecstich: &mut Vec<CStich>) {
        assert_eq!(self.m_vecsusptrans.len(), 0); // currently, we have no caching
        vecstich.push(CStich::new(self.m_eplayerindex_first));
        let eplayerindex_first = self.m_eplayerindex_first;
        let player_index = move |i_raw: usize| {(eplayerindex_first + i_raw) % 4};
        macro_rules! traverse_valid_cards {
            ($i_raw : expr, $func: expr) => {
                // TODO use equivalent card optimization
                for card in rules.all_allowed_cards(vecstich, &self.m_ahand[player_index($i_raw)]) {
                    vecstich.last_mut().unwrap().zugeben(card);
                    $func;
                    vecstich.last_mut().unwrap().undo_most_recent_card();
                }
            };
        };
        traverse_valid_cards!(0, { // TODO: more efficient to explicitly handle first card?
            traverse_valid_cards!(1, {
                traverse_valid_cards!(2, {
                    traverse_valid_cards!(3, {
                        let susptrans = SSuspicionTransition::new(self, vecstich.last().unwrap().clone(), rules);
                        self.m_vecsusptrans.push(susptrans);
                        let n_stich = vecstich.len();
                        self.m_vecsusptrans.last_mut().unwrap().m_susp.internal_compute_successors(rules, vecstich);
                        assert_eq!(n_stich, vecstich.len());
                    } );
                } );
            } );
        } );
        vecstich.pop().expect("vecstich was empty at the end of compute_successors");
    }

    fn size(&self) -> usize {
        self.m_vecsusptrans.iter().fold(0, |acc, ref susptrans| acc+susptrans.m_susp.size())
    }

    fn leaf_count(&self) -> usize {
        if 0==self.m_vecsusptrans.len() {
            assert_eq!(self.hand_size(), 0);
            1
        } else {
            self.m_vecsusptrans.iter().fold(0, |acc, ref susptrans| acc+susptrans.m_susp.leaf_count())
        }
    }

    pub fn print_suspicion(&self, n_maxlevel: usize, n_level: usize, rules: &TRules, vecstich: &mut Vec<CStich>) {
        if n_maxlevel < n_level {
            return;
        }
        for eplayerindex in 0..4 {
            print!("{} | ", self.m_ahand[eplayerindex]);
        }
        let ann_payout = self.internal_quality(rules, vecstich);
        for eplayerindex in 0..4 {
            print!("({}, {}) ", ann_payout[eplayerindex].0, ann_payout[eplayerindex].1);
        }
        println!("");
        for susptrans in self.m_vecsusptrans.iter() {
            susptrans.print_suspiciontransition(n_maxlevel, n_level+1, rules, vecstich);
        }
    }

    fn internal_quality(&self, rules: &TRules, vecstich: &mut Vec<CStich>) -> [(isize, isize); 4] {
        // for now, quality is measured as range [min payout, max payout]
        if 0==self.hand_size() {
            assert_eq!(8, vecstich.len());
            let an_payout = rules.payout(vecstich);
            let mut ann_payout : [(isize, isize); 4] = [(0,0); 4];
            for eplayerindex in 0..4 {
                ann_payout[eplayerindex] = (an_payout[eplayerindex], an_payout[eplayerindex]);
            }
            ann_payout
        } else {
            assert!(!self.m_vecsusptrans.is_empty());
            vecstich.push(self.m_vecsusptrans.first().unwrap().m_stich.clone());
            let mut ann_payout = self.m_vecsusptrans.first().unwrap().m_susp.internal_quality(rules, vecstich);
            vecstich.pop().expect("vecstich is empty");

            for susptrans in self.m_vecsusptrans.iter().skip(1) {
                vecstich.push(susptrans.m_stich.clone());
                let ann_payout_successor = susptrans.m_susp.internal_quality(rules, vecstich);
                for eplayerindex in 0..4 {
                    ann_payout[eplayerindex].0 = cmp::min(
                        ann_payout[eplayerindex].0,
                        ann_payout_successor[eplayerindex].0
                    );
                    ann_payout[eplayerindex].1 = cmp::max(
                        ann_payout[eplayerindex].1,
                        ann_payout_successor[eplayerindex].1
                    );
                }
                vecstich.pop().expect("vecstich is empty");
            }
            ann_payout
        }
    }


}

