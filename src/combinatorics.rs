use std::cmp;
use card::*;
use hand::*;
use stich::*;
use suspicion::*;

use permutohedron::LexicalPermutation;

pub fn for_each_suspicion<FuncFilter, Func>(
    hand_known: &CHand,
    veccard_unknown : &Vec<CCard>,
    eplayerindex: EPlayerIndex,
    mut func_filter: FuncFilter,
    mut func: Func
)
    where Func: FnMut(SSuspicion),
          FuncFilter: FnMut(&SSuspicion) -> bool
{
    assert_eq!(0, eplayerindex); // TODO: generalize
    let n_cards_total = veccard_unknown.len();
    assert_eq!(n_cards_total%3, 0);
    let n_cards_per_player = n_cards_total / 3;
    let mut veci : Vec<usize> = (0..n_cards_total).map(|i| i/n_cards_per_player).collect();
    let mut callback = |veci : &Vec<usize>| {
        let get_hand = |eplayerindex_hand| {
            CHand::new_from_vec(veci.iter().enumerate()
                .filter(|&(_i, eplayerindex_susp)| *eplayerindex_susp == eplayerindex_hand)
                .map(|(i, _eplayerindex_susp)| veccard_unknown[i.clone()]).collect())
        };
        let susp = SSuspicion::new_from_raw(
            eplayerindex,
            &[
                hand_known.clone(),
                get_hand(0),
                get_hand(1),
                get_hand(2),
            ]

        );
        if func_filter(&susp) {
            func(susp);
        }
    };
    callback(&veci);
    while veci[..].next_permutation() {
        callback(&veci);
    }
}
