use primitives::*;
use game::*;
use util::*;
use permutohedron::LexicalPermutation;
use ai::{
    *,
    suspicion::*,
};
use rand::{self, Rng};


pub struct SForeverRandHands {
    epi_fixed : EPlayerIndex,
    ahand: EnumMap<EPlayerIndex, SHand>,
}

impl Iterator for SForeverRandHands {
    type Item = EnumMap<EPlayerIndex, SHand>;
    fn next(&mut self) -> Option<EnumMap<EPlayerIndex, SHand>> {
        assert_ahand_same_size(&self.ahand);
        let n_len_hand = self.ahand[EPlayerIndex::EPI0].cards().len();
        let mut rng = rand::thread_rng();
        for i_card in 0..3*n_len_hand {
            let i_rand = rng.gen_range(0, 3*n_len_hand - i_card);
            let ((epi_swap, i_hand_swap), (epi_rand, i_hand_rand)) = {
                let convert_to_idxs = |i_rand| {
                    // epi_fixed==0 => 8..31 valid
                    // epi_fixed==1 => 0..7, 16..31 valid
                    // epi_fixed==2 => 0..15, 24..31 valid
                    // epi_fixed==3 => 0..23  valid
                    let i_valid = {
                        if i_rand < self.epi_fixed.to_usize()*n_len_hand {
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
                let assert_valid = |epi, i_hand| {
                    assert!(i_hand<n_len_hand);
                    assert_ne!(epi, self.epi_fixed);
                };
                assert_valid(epi_swap, i_hand_swap);
                assert_valid(epi_rand, i_hand_rand);
            }
            let card_swap = self.ahand[epi_swap].cards()[i_hand_swap];
            let card_rand = self.ahand[epi_rand].cards()[i_hand_rand];
            self.ahand[epi_swap].cards_mut()[i_hand_swap] = card_rand;
            self.ahand[epi_rand].cards_mut()[i_hand_rand] = card_swap;
        }
        Some(self.ahand.clone())
    }
}

pub fn forever_rand_hands(vecstich: &[SStich], hand_fixed: &SHand, epi_fixed: EPlayerIndex) -> SForeverRandHands {
    SForeverRandHands {
        epi_fixed,
        ahand : {
            let mut veccard_unplayed = unplayed_cards(vecstich, hand_fixed);
            assert!(veccard_unplayed.len()>=3*hand_fixed.cards().len());
            let n_size = hand_fixed.cards().len();
            EPlayerIndex::map_from_fn(|epi| {
                if epi==epi_fixed {
                    hand_fixed.clone()
                } else {
                    random_hand(n_size, &mut veccard_unplayed)
                }
            })
        }
    }
}

pub struct SAllHands {
    epi_fixed : EPlayerIndex,
    veccard_unknown: Vec<SCard>,
    vecepi: Vec<EPlayerIndex>,
    hand_known: SHand,
    b_valid: bool,
}

impl Iterator for SAllHands {
    type Item = EnumMap<EPlayerIndex, SHand>;
    fn next(&mut self) -> Option<EnumMap<EPlayerIndex, SHand>> {
        if self.b_valid {
            let ahand = EPlayerIndex::map_from_fn(|epi| {
                if self.epi_fixed==epi {
                    self.hand_known.clone()
                } else {
                    SHand::new_from_vec(self.vecepi.iter().enumerate()
                        .filter(|&(_i, epi_susp)| *epi_susp == epi)
                        .map(|(i, _epi_susp)| self.veccard_unknown[i]).collect())
                }
            });
            self.b_valid = self.vecepi[..].next_permutation();
            Some(ahand)
        } else {
            assert!(!self.vecepi[..].next_permutation());
            None
        }
    }
}

pub fn all_possible_hands(vecstich: &[SStich], hand_fixed: SHand, epi_fixed: EPlayerIndex) -> SAllHands {
    let veccard_unknown = unplayed_cards(vecstich, &hand_fixed);
    let n_cards_total = veccard_unknown.len();
    assert_eq!(n_cards_total%3, 0);
    let n_cards_per_player = n_cards_total / 3;
    SAllHands {
        epi_fixed,
        veccard_unknown,
        vecepi: (0..n_cards_total)
            .map(|i| {
                let n_epi = i/n_cards_per_player;
                assert!(n_epi<=2);
                EPlayerIndex::from_usize(if n_epi<epi_fixed.to_usize() {
                    n_epi
                } else {
                    n_epi + 1
                })
                
            })
            .collect(),
        hand_known: hand_fixed,
        b_valid: true, // in the beginning, there should be a valid assignment of cards to players
    }
}

#[test]
fn test_all_possible_hands() {
    use primitives::cardvector::parse_cards;
    let str_to_stich = |str_stich| {
        let mut stich = SStich::new(/*epi should not be relevant*/EPlayerIndex::EPI0);
        for card in parse_cards::<Vec<_>>(str_stich).unwrap() {
            stich.push(card.clone());
        }
        stich
    };
    let mut vecstich = ["g7 g8 ga g9", "s8 ho s7 s9", "h7 hk hu su", "eo go hz h8"].iter()
        .map(|str_stich| str_to_stich(str_stich))
        .collect::<Vec<_>>();
    let mut add_stich_and_test = |str_stich, str_hand, n_count| {
        vecstich.push(str_to_stich(str_stich));
        assert_eq!(
            all_possible_hands(
                &vecstich,
                SHand::new_from_vec(parse_cards(str_hand).unwrap()),
                EPlayerIndex::EPI0, // epi_fixed
            ).count(),
            n_count
        );
    };
    // 3*n unknown cards distributed among three players: binomial(3n,3)*binomial(2n,n) possibilities
    add_stich_and_test("e9 ek e8 ea", "gk sk sa", 1680);
    add_stich_and_test("sa eu so ha", "gk sk", 90);
    add_stich_and_test("h9 gz gu gk", "sk", 6);
}

