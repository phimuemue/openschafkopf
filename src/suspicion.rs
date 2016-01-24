use card::*;
use stich::*;
use hand::*;
use rules::*;

struct SSuspicionTransition {
    m_stich : CStich,
    m_psusp : Box<SSuspicion>,
}

impl SSuspicionTransition {
    fn new(susp: &SSuspicion, stich: &CStich, rules: &TRules) -> SSuspicionTransition {
        SSuspicionTransition {
            m_stich : stich.clone(),
            m_psusp : Box::new(SSuspicion::new_from_susp(susp, stich, rules))
        }
    }

    fn print_suspiciontransition(&self, n_maxlevel: usize, n_level: usize) {
        if n_level<=n_maxlevel {
            for _ in 0..n_level+1 {
                print!(" ");
            }
            print!("{} : ", self.m_stich);
            if 0<self.m_psusp.hand_size() {
                self.m_psusp.print_suspicion(n_maxlevel, n_level);
            } else {
                println!("");
            }
        }
    }
}

struct SSuspicion {
    m_vecsusptrans : Vec<SSuspicionTransition>,
    m_eplayerindex_first : EPlayerIndex,
    m_ahand : [CHand; 4],
}

type SSuspicionHashType = (usize, [usize; 4]);

impl SSuspicion {

    fn new_from_raw(eplayerindex_first: EPlayerIndex, ahand: &[CHand; 4]) -> Self {
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

    fn compute_successors(&mut self, _rules: &TRules) {
        unimplemented!();
        //TODO: internal_compute_successors(self, rules, Vec::new());
    }

    fn internal_compute_successors(&mut self, _rules: &TRules, vecstich: &mut Vec<CStich>) {
        unimplemented!();
        //assert_eq!(self.m_vecsusptrans.len(), 0); // currently, we have no caching
        //vecstich.push(CStich::new(self.m_eplayerindex_first));
        //unimplemented!(); // shall player_index take i as usize?
        //let player_index = |i: usize| {(self.m_eplayerindex_first + i) % 4};
        //let traverse_valid_cards = |i_raw, func: usize| {
        //    let veccard_allowed = rules.all_allowed_cards(vecstich, &self.m_ahand[player_index(i_raw)]);
        //    unimplemented!(); // TODO assert sorted by equivalence!
        //    let mut card_current : CCard;
        //    let install_card_and_call_func = |card| {
        //        vecstich.last().unwrap().zugeben(card);
        //        card_current = card;
        //        unimplemented!();
        //        //func();
        //        vecstich.last().unwrap().undo_most_recent_card();
        //    };
        //    if 0<veccard_allowed.len() {
        //        install_card_and_call_func(veccard_allowed[0]);
        //        for &card in veccard_allowed.iter().nth(1) {
        //            if !rules.equivalent_when_on_same_hand(card_current, card, vecstich) {
        //                install_card_and_call_func(card);
        //            }
        //        }
        //    }
        //};
        //unimplemented!(); // can't rust closures be instanciated with different funcs!?
        //traverse_valid_cards(0, || { // TODO: more efficient to explicitly handle first card?
        //    traverse_valid_cards(1, || {
        //        traverse_valid_cards(2, || {
        //            traverse_valid_cards(3, || {
        //                self.m_vecsusptrans.push(
        //                    SSuspicionTransition::new (
        //                        self,
        //                        vecstich.last().unwrap(),
        //                        rules
        //                    )
        //                );
        //                let n_stich = vecstich.len();
        //                let mut b_no_premature_winner = !rules.is_prematurely_winner(vecstich).iter().any(|&b| b);
        //                if b_no_premature_winner {
        //                    self.m_vecsusptrans.last().unwrap().m_psusp.internal_compute_successors(rules, vecstich);
        //                }
        //                assert_eq!(n_stich, vecstich.len());
        //            } );
        //        } );
        //    } );
        //} );
        vecstich.pop().expect("vecstich was empty at the end of compute_successors");
    }

    fn size(&self) -> usize {
        self.m_vecsusptrans.iter().fold(0, |acc, ref susptrans| acc+susptrans.m_psusp.size())
    }

    fn leaf_count(&self) -> usize {
        if 0==self.m_vecsusptrans.len() {
            assert_eq!(self.hand_size(), 0);
            1
        } else {
            self.m_vecsusptrans.iter().fold(0, |acc, ref susptrans| acc+susptrans.m_psusp.leaf_count())
        }
    }

    fn print_suspicion(&self, _n_maxlevel: usize, _n_level: usize) {
        unimplemented!();
    }

    // apparently, quality computation is highly nontrivial.
    // This affects computation in twofold ways:
    // (1) We do not know a priori what quality means. As many points as possible? Exaclty the opposite?
    //     We do not even know if TSuspicionQuality is an adequate measure in general.
    //     Thus, TODO: make Quality a template that can not only deal with TSuspicionQuality as quality measure.
    // (2) We do not know a priori how to accumulate the different qualities of the successors into
    //     one quality for the suspicion under consideration. Thus, a functor has to be supplied that
    //     implements an accumulation strategy.
    // (3) We can not be sure that we can cache the quality, which is why we do not do it at the moment.

    fn quality<TSuspicionQuality, FuncAccu> (&self, _func_accu: FuncAccu)
        where FuncAccu: Fn(&Vec<CStich>, &Vec<(SSuspicionTransition, TSuspicionQuality)>) -> TSuspicionQuality
    {
        unimplemented!();
    }


}

