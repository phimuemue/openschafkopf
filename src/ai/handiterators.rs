use primitives::*;
use game::*;

use permutohedron::LexicalPermutation;
use ai::*;

use ai::suspicion::*;

use rand::{self, Rng};


pub struct SForeverRandHands {
    m_eplayerindex_fixed : EPlayerIndex,
    m_ahand: [SHand; 4],
}

impl Iterator for SForeverRandHands {
    type Item = [SHand; 4];
    fn next(&mut self) -> Option<[SHand; 4]> {
        assert_ahand_same_size(&self.m_ahand);
        let n_len_hand = self.m_ahand[0].cards().len();
        let mut rng = rand::thread_rng();
        for i_card in 0..3*n_len_hand {
            let i_rand = rng.gen_range(0, 3*n_len_hand - i_card);
            let ((eplayerindex_swap, i_hand_swap), (eplayerindex_rand, i_hand_rand)) = {
                let convert_to_idxs = |i_rand| {
                    // eplayerindex_fixed==0 => 8..31 valid
                    // eplayerindex_fixed==1 => 0..7, 16..31 valid
                    // eplayerindex_fixed==2 => 0..15, 24..31 valid
                    // eplayerindex_fixed==3 => 0..23  valid
                    let i_valid = {
                        if i_rand < self.m_eplayerindex_fixed*n_len_hand {
                            i_rand
                        } else {
                            i_rand + n_len_hand
                        }
                    };
                    (i_valid/n_len_hand, i_valid%n_len_hand)
                };
                (convert_to_idxs(i_card), convert_to_idxs(i_rand))
            };
            {
                let assert_valid = |eplayerindex, i_hand| {
                    assert!(eplayerindex<4);
                    assert!(i_hand<n_len_hand);
                    assert!(eplayerindex!=self.m_eplayerindex_fixed);
                };
                assert_valid(eplayerindex_swap, i_hand_swap);
                assert_valid(eplayerindex_rand, i_hand_rand);
            }
            let card_swap = self.m_ahand[eplayerindex_swap].cards()[i_hand_swap];
            let card_rand = self.m_ahand[eplayerindex_rand].cards()[i_hand_rand];
            *self.m_ahand[eplayerindex_swap].cards_mut().get_mut(i_hand_swap).unwrap() = card_rand;
            *self.m_ahand[eplayerindex_rand].cards_mut().get_mut(i_hand_rand).unwrap() = card_swap;
        }
        Some(create_playerindexmap(|eplayerindex| self.m_ahand[eplayerindex].clone()))
    }
}

pub fn forever_rand_hands(vecstich: &[SStich], hand_fixed: SHand, eplayerindex_fixed: EPlayerIndex) -> SForeverRandHands {
    SForeverRandHands {
        m_eplayerindex_fixed : eplayerindex_fixed,
        m_ahand : {
            let mut vecocard = unplayed_cards(vecstich, &hand_fixed);
            assert!(vecocard.iter().filter(|ocard| ocard.is_some()).count()>=3*hand_fixed.cards().len());
            let n_size = hand_fixed.cards().len();
            create_playerindexmap(|eplayerindex| {
                if eplayerindex==eplayerindex_fixed {
                    hand_fixed.clone()
                } else {
                    random_hand(n_size, &mut vecocard)
                }
            })
        }
    }
}


pub fn for_each_possible_hand<FuncFilter, Func>(
    vecstich: &Vec<SStich>,
    hand_known: &SHand,
    eplayerindex_fixed: EPlayerIndex,
    mut func_filter: FuncFilter,
    mut func: Func
)
    where Func: FnMut([SHand; 4]),
          FuncFilter: FnMut(&[SHand; 4]) -> bool
{
    let veccard_unknown = unplayed_cards(vecstich, hand_known).into_iter()
        .filter_map(|ocard| ocard)
        .collect::<Vec<_>>();
    let n_cards_total = veccard_unknown.len();
    assert_eq!(n_cards_total%3, 0);
    let n_cards_per_player = n_cards_total / 3;
    let mut veceplayerindex : Vec<EPlayerIndex> = (0..n_cards_total)
        .map(|i| {
            let eplayerindex = i/n_cards_per_player;
            assert!(eplayerindex<=2);
            if eplayerindex<eplayerindex_fixed {
                eplayerindex
            } else {
                eplayerindex + 1
            }
            
        })
        .collect();
    let mut callback = |veceplayerindex : &Vec<EPlayerIndex>| {
        let ahand = create_playerindexmap(|eplayerindex| {
            if eplayerindex_fixed==eplayerindex {
                hand_known.clone()
            } else {
                SHand::new_from_vec(veceplayerindex.iter().enumerate()
                    .filter(|&(_i, eplayerindex_susp)| *eplayerindex_susp == eplayerindex)
                    .map(|(i, _eplayerindex_susp)| veccard_unknown[i.clone()]).collect())
            }
        });
        if func_filter(&ahand) {
            func(ahand);
        }
    };
    callback(&veceplayerindex);
    while veceplayerindex[..].next_permutation() {
        callback(&veceplayerindex);
    }
}
