use primitives::*;
use game::*;
use util::*;

use permutohedron::LexicalPermutation;
use ai::*;

use ai::suspicion::*;

use rand::{self, Rng};


pub struct SForeverRandHands {
    m_eplayerindex_fixed : EPlayerIndex,
    m_ahand: SEnumMap<EPlayerIndex, SHand>,
}

impl Iterator for SForeverRandHands {
    type Item = SEnumMap<EPlayerIndex, SHand>;
    fn next(&mut self) -> Option<SEnumMap<EPlayerIndex, SHand>> {
        assert_ahand_same_size(&self.m_ahand);
        let n_len_hand = self.m_ahand[EPlayerIndex::EPI0].cards().len();
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
                        if i_rand < self.m_eplayerindex_fixed.to_usize()*n_len_hand {
                            i_rand
                        } else {
                            i_rand + n_len_hand
                        }
                    };
                    (EPlayerIndex::from_usize(i_valid/n_len_hand), i_valid%n_len_hand)
                };
                (convert_to_idxs(i_card), convert_to_idxs(i_rand))
            };
            {
                let assert_valid = |eplayerindex, i_hand| {
                    assert!(i_hand<n_len_hand);
                    assert!(eplayerindex!=self.m_eplayerindex_fixed);
                };
                assert_valid(eplayerindex_swap, i_hand_swap);
                assert_valid(eplayerindex_rand, i_hand_rand);
            }
            let card_swap = self.m_ahand[eplayerindex_swap].cards()[i_hand_swap];
            let card_rand = self.m_ahand[eplayerindex_rand].cards()[i_hand_rand];
            self.m_ahand[eplayerindex_swap].cards_mut()[i_hand_swap] = card_rand;
            self.m_ahand[eplayerindex_rand].cards_mut()[i_hand_rand] = card_swap;
        }
        Some(EPlayerIndex::map_from_fn(|eplayerindex| self.m_ahand[eplayerindex].clone()))
    }
}

pub fn forever_rand_hands(vecstich: &[SStich], hand_fixed: SHand, eplayerindex_fixed: EPlayerIndex) -> SForeverRandHands {
    SForeverRandHands {
        m_eplayerindex_fixed : eplayerindex_fixed,
        m_ahand : {
            let mut veccard_unplayed = unplayed_cards(vecstich, &hand_fixed);
            assert!(veccard_unplayed.len()>=3*hand_fixed.cards().len());
            let n_size = hand_fixed.cards().len();
            EPlayerIndex::map_from_fn(|eplayerindex| {
                if eplayerindex==eplayerindex_fixed {
                    hand_fixed.clone()
                } else {
                    random_hand(n_size, &mut veccard_unplayed)
                }
            })
        }
    }
}

pub struct SAllHands {
    m_eplayerindex_fixed : EPlayerIndex,
    m_veccard_unknown: Vec<SCard>,
    m_veceplayerindex: Vec<EPlayerIndex>,
    m_hand_known: SHand,
    m_b_valid: bool,
}

impl Iterator for SAllHands {
    type Item = SEnumMap<EPlayerIndex, SHand>;
    fn next(&mut self) -> Option<SEnumMap<EPlayerIndex, SHand>> {
        if self.m_b_valid {
            let ahand = EPlayerIndex::map_from_fn(|eplayerindex| {
                if self.m_eplayerindex_fixed==eplayerindex {
                    self.m_hand_known.clone()
                } else {
                    SHand::new_from_vec(self.m_veceplayerindex.iter().enumerate()
                        .filter(|&(_i, eplayerindex_susp)| *eplayerindex_susp == eplayerindex)
                        .map(|(i, _eplayerindex_susp)| self.m_veccard_unknown[i]).collect())
                }
            });
            self.m_b_valid = self.m_veceplayerindex[..].next_permutation();
            Some(ahand)
        } else {
            assert!(!self.m_veceplayerindex[..].next_permutation());
            None
        }
    }
}

pub fn all_possible_hands(vecstich: &[SStich], hand_fixed: SHand, eplayerindex_fixed: EPlayerIndex) -> SAllHands {
    let veccard_unknown = unplayed_cards(vecstich, &hand_fixed);
    let n_cards_total = veccard_unknown.len();
    assert_eq!(n_cards_total%3, 0);
    let n_cards_per_player = n_cards_total / 3;
    SAllHands {
        m_eplayerindex_fixed : eplayerindex_fixed,
        m_veccard_unknown: veccard_unknown,
        m_veceplayerindex: (0..n_cards_total)
            .map(|i| {
                let n_eplayerindex = i/n_cards_per_player;
                assert!(n_eplayerindex<=2);
                EPlayerIndex::from_usize(if n_eplayerindex<eplayerindex_fixed.to_usize() {
                    n_eplayerindex
                } else {
                    n_eplayerindex + 1
                })
                
            })
            .collect(),
        m_hand_known: hand_fixed,
        m_b_valid: true, // in the beginning, there should be a valid assignment of cards to players
    }
}

#[test]
fn test_all_possible_hands() {
    use util::cardvectorparser;
    // TODO improve test
    let vecstich = ["g7 g8 ga g9", "s8 ho s7 s9", "h7 hk hu su", "eo go hz h8", "e9 ek e8 ea", "sa eu so ha"].into_iter()
        .map(|str_stich| {
            let mut stich = SStich::new(/*eplayerindex should not be relevant*/EPlayerIndex::EPI0);
            for card in cardvectorparser::parse_cards::<Vec<_>>(str_stich).unwrap() {
                stich.push(card.clone());
            }
            stich
        })
        .collect::<Vec<_>>();
    let vecahand = all_possible_hands(
        &vecstich,
        SHand::new_from_vec(cardvectorparser::parse_cards("gk sk").unwrap()),
        EPlayerIndex::EPI0, // eplayerindex_fixed
    ).collect::<Vec<_>>();
    assert_eq!(vecahand.len(), 90); // 6 cards are unknown, distributed among three other players, i.e. binomial(6,2)*binomial(4,2)=90 possibilities
}

