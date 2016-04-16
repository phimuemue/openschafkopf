use stich::*;
use hand::*;
use rules::*;
use itertools::Itertools;

pub struct SSuspicionTransition {
    m_stich : CStich,
    m_susp : SSuspicion,
}

fn push_pop_vecstich<Func, R>(vecstich: &mut Vec<CStich>, stich: CStich, func: Func) -> R
    where Func: FnOnce(&mut Vec<CStich>) -> R
{
    let n_stich = vecstich.len();
    assert!(vecstich.iter().all(|stich| stich.size()==4));
    vecstich.push(stich);
    let r = func(vecstich);
    vecstich.pop().expect("vecstich unexpectedly empty");
    assert!(vecstich.iter().all(|stich| stich.size()==4));
    assert_eq!(n_stich, vecstich.len());
    r
}

impl SSuspicionTransition {
    fn new(susp: &SSuspicion, stich: CStich, rules: &TRules) -> SSuspicionTransition {
        let susp = SSuspicion::new_from_susp(susp, &stich, rules);
        SSuspicionTransition {
            m_stich : stich,
            m_susp : susp
        }
    }

    pub fn stich(&self) -> &CStich {
        &self.m_stich
    }

    pub fn suspicion(&self) -> &SSuspicion {
        &self.m_susp
    }

    fn print_suspiciontransition(&self, n_maxlevel: usize, n_level: usize, rules: &TRules, vecstich: &mut Vec<CStich>, ostich_given: Option<CStich>) {
        if n_level<=n_maxlevel {
            push_pop_vecstich(vecstich, self.m_stich.clone(), |vecstich| {
                assert_eq!(vecstich.len()+self.m_susp.hand_size(), 8);
                for _ in 0..n_level+1 {
                    print!(" ");
                }
                print!("{} : ", self.m_stich);
                if 1<self.m_susp.hand_size() {
                    self.m_susp.print_suspicion(n_maxlevel, n_level, rules, vecstich, ostich_given);
                } else {
                    println!("");
                }
            });
        }
    }
}

pub struct SSuspicion {
    m_vecsusptrans : Vec<SSuspicionTransition>,
    m_eplayerindex_first : EPlayerIndex,
    m_ahand : [CHand; 4],
}

impl SSuspicion {

    pub fn suspicion_tranitions(&self) -> &Vec<SSuspicionTransition> {
        &self.m_vecsusptrans
    }

    pub fn new_from_raw(eplayerindex_first: EPlayerIndex, ahand: [CHand; 4]) -> Self {
        SSuspicion {
            m_vecsusptrans: Vec::new(),
            m_eplayerindex_first : eplayerindex_first,
            m_ahand : ahand
        }
    }

    pub fn hands(&self) -> &[CHand; 4] {
        &self.m_ahand
    }

    fn new_from_susp(&self, stich: &CStich, rules: &TRules) -> Self {
        //println!("new_from_susp {}", stich);
        //println!("wi: {}", rules.winner_index(stich));
        SSuspicion {
            m_vecsusptrans: Vec::new(),
            m_eplayerindex_first : rules.winner_index(stich),
            m_ahand : create_playerindexmap(|eplayerindex| {
                self.m_ahand[eplayerindex].new_from_hand(stich[eplayerindex])
            })
        }
    }

    fn hand_size(&self) -> usize {
        assert_eq!(self.m_ahand[0].cards().len(), self.m_ahand[1].cards().len());
        assert_eq!(self.m_ahand[0].cards().len(), self.m_ahand[2].cards().len());
        assert_eq!(self.m_ahand[0].cards().len(), self.m_ahand[3].cards().len());
        self.m_ahand[0].cards().len()
    }

    pub fn compute_successors<FuncFilterSuccessors>(&mut self, rules: &TRules, vecstich: &mut Vec<CStich>, func_filter_successors: &FuncFilterSuccessors)
        where FuncFilterSuccessors : Fn(&Vec<CStich> /*vecstich_complete*/, &mut Vec<CStich>/*vecstich_successor*/)
    {
        assert_eq!(self.m_vecsusptrans.len(), 0); // currently, we have no caching
        let mut vecstich_successor : Vec<CStich> = Vec::new();
        push_pop_vecstich(vecstich, CStich::new(self.m_eplayerindex_first), |vecstich| {
            let eplayerindex_first = self.m_eplayerindex_first;
            let player_index = move |i_raw: usize| {(eplayerindex_first + i_raw) % 4};
            macro_rules! traverse_valid_cards {
                ($i_raw : expr, $func: expr) => {
                    // TODO use equivalent card optimization
                    for card in rules.all_allowed_cards(vecstich, &self.m_ahand[player_index($i_raw)]) {
                        vecstich.last_mut().unwrap().zugeben(card);
                        assert!(card==vecstich.last().unwrap()[player_index($i_raw)]);
                        $func;
                        vecstich.last_mut().unwrap().undo_most_recent_card();
                    }
                };
            };
            traverse_valid_cards!(0, { // TODO: more efficient to explicitly handle first card?
                traverse_valid_cards!(1, {
                    traverse_valid_cards!(2, {
                        traverse_valid_cards!(3, {
                            vecstich_successor.push(vecstich.last().unwrap().clone());
                        } );
                    } );
                } );
            } );
        });
        if !vecstich_successor.is_empty() {
            func_filter_successors(vecstich, &mut vecstich_successor);
            assert!(!vecstich_successor.is_empty());
        }
        self.m_vecsusptrans = vecstich_successor.into_iter()
            .map(|stich| {
                let mut susptrans = SSuspicionTransition::new(self, stich.clone(), rules);
                push_pop_vecstich(vecstich, stich, |vecstich| {
                    susptrans.m_susp.compute_successors(rules, vecstich, func_filter_successors);
                });
                susptrans
            })
            .collect();
    }

    pub fn print_suspicion(
        &self,
        n_maxlevel: usize,
        n_level: usize,
        rules: &TRules,
        vecstich: &mut Vec<CStich>,
        ostich_given: Option<CStich>
    ) {
        if n_maxlevel < n_level {
            return;
        }
        for eplayerindex in 0..4 {
            print!("{} | ", self.m_ahand[eplayerindex]);
        }
        print!(", min payouts: ");
        for eplayerindex in 0..4 {
            print!("{}, ", self.min_reachable_payout(rules, vecstich, ostich_given.clone(), eplayerindex));
        }
        println!("");
        for susptrans in self.m_vecsusptrans.iter() {
            susptrans.print_suspiciontransition(n_maxlevel, n_level+1, rules, vecstich, ostich_given.clone());
        }
    }

    pub fn min_reachable_payout(
        &self,
        rules: &TRules,
        vecstich: &mut Vec<CStich>,
        ostich_given: Option<CStich>,
        eplayerindex: EPlayerIndex
    ) -> isize {
        let vecstich_backup = vecstich.clone();
        assert!(ostich_given.as_ref().map_or(true, |stich| stich.size() < 4));
        assert!(vecstich.iter().all(|stich| stich.size()==4));
        assert_eq!(vecstich.len()+self.hand_size(), 8);
        if 0==self.hand_size() {
            return rules.payout(vecstich)[eplayerindex];
        }
        let n_payout = self.m_vecsusptrans.iter()
            .filter(|susptrans| { // only consider successors compatible with current stich_given so far
                assert_eq!(susptrans.m_susp.hand_size()+1, self.hand_size());
                ostich_given.as_ref().map_or(true, |stich_given| {
                    stich_given.indices_and_cards()
                        .zip(susptrans.m_stich.indices_and_cards())
                        .all(|((i_current_stich, card_current_stich), (i_susp_stich, card_susp_stich))| {
                            assert_eq!(i_current_stich, i_susp_stich);
                            card_current_stich==card_susp_stich
                        })
                })
            })
            .map(|susptrans| {
                assert_eq!(susptrans.m_stich.size(), 4);
                push_pop_vecstich(vecstich, susptrans.m_stich.clone(), |vecstich| {
                    (susptrans, susptrans.m_susp.min_reachable_payout(rules, vecstich, None, eplayerindex))
                })
            })
            .group_by(|&(susptrans, _n_payout)| { // other players may play inconveniently for eplayerindex...
                susptrans.m_stich.indices_and_cards()
                    .take_while(|&(eplayerindex_stich, _card)| eplayerindex_stich != eplayerindex)
                    .map(|(_eplayerindex, card)| card)
                    .collect::<Vec<_>>();
            })
            .map(|(_stich_key_before_eplayerindex, grpsusptransn_before_eplayerindex)| {
                grpsusptransn_before_eplayerindex.into_iter()
                    .group_by(|&(susptrans, _n_payout)| susptrans.m_stich[eplayerindex])
                    .map(|(_stich_key_eplayerindex, grpsusptransn_eplayerindex)| {
                        // in this group, we need the worst case if other players play badly
                        grpsusptransn_eplayerindex.into_iter().min_by_key(|&(_susptrans, n_payout)| n_payout).unwrap()
                    })
                    .max_by_key(|&(_susptrans, n_payout)| n_payout)
                    .unwrap()
            })
            .min_by_key(|&(_susptrans, n_payout)| n_payout)
            .unwrap()
            .1;
        assert!(vecstich_backup.iter().zip(vecstich.iter()).all(|(s1,s2)|s1.size()==s2.size()));
        n_payout
    }

}

