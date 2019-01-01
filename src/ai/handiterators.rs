use crate::primitives::*;
use crate::util::*;
use permutohedron::LexicalPermutation;
use crate::ai::*;
use rand::prelude::*;

pub trait TNextVecEPI {
    fn next(slcepi: &mut[EPlayerIndex]) -> bool;
}

pub struct SNextVecEPIShuffle;
impl TNextVecEPI for SNextVecEPIShuffle {
    fn next(slcepi: &mut[EPlayerIndex]) -> bool {
        slcepi.shuffle(&mut rand::thread_rng());
        true
    }
}

pub struct SNextVecEPIPermutation;
impl TNextVecEPI for SNextVecEPIPermutation {
    fn next(slcepi: &mut[EPlayerIndex]) -> bool {
        slcepi.next_permutation()
    }
}

pub struct SHandIterator<NextVecEPI: TNextVecEPI> {
    epi_fixed : EPlayerIndex,
    veccard_unknown: Vec<SCard>,
    vecepi: Vec<EPlayerIndex>,
    hand_known: SHand,
    b_valid: bool,
    phantom: std::marker::PhantomData<NextVecEPI>,
}

impl<NextVecEPI: TNextVecEPI> Iterator for SHandIterator<NextVecEPI> {
    type Item = EnumMap<EPlayerIndex, SHand>;
    fn next(&mut self) -> Option<Self::Item> {
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
            self.b_valid = NextVecEPI::next(self.vecepi.as_mut_slice());
            Some(ahand)
        } else {
            assert!(!self.vecepi[..].next_permutation());
            None
        }
    }
}

fn make_handiterator<NextVecEPI: TNextVecEPI>(slcstich: &[SStich], hand_fixed: SHand, epi_fixed: EPlayerIndex, ekurzlang: EKurzLang) -> SHandIterator<NextVecEPI> {
    completed_stichs(slcstich); // asserts invariants. TODO introduce SStichSequence
    let veccard_unknown = unplayed_cards(slcstich, &hand_fixed, ekurzlang).collect::<Vec<_>>();
    let mapepin_cards_per_hand = remaining_cards_per_hand(slcstich, ekurzlang);
    let mut vecepi = Vec::new();
    for epi in EPlayerIndex::values() {
        if epi==epi_fixed {
            assert_eq!(mapepin_cards_per_hand[epi], hand_fixed.cards().len());
        } else {
            vecepi.extend(std::iter::repeat(epi).take(mapepin_cards_per_hand[epi]));
        }
    }
    assert_eq!(veccard_unknown.len(), vecepi.len());
    SHandIterator {
        epi_fixed,
        veccard_unknown,
        vecepi,
        hand_known: hand_fixed,
        b_valid: true, // in the beginning, there should be a valid assignment of cards to players
        phantom: std::marker::PhantomData,
    }
}

pub fn all_possible_hands(slcstich: &[SStich], hand_fixed: SHand, epi_fixed: EPlayerIndex, ekurzlang: EKurzLang) -> SHandIterator<SNextVecEPIPermutation> {
    let ithand = make_handiterator(slcstich, hand_fixed, epi_fixed, ekurzlang);
    assert!(ithand.vecepi.iter().is_sorted());
    ithand
}

pub fn forever_rand_hands(slcstich: &[SStich], hand_fixed: SHand, epi_fixed: EPlayerIndex, ekurzlang: EKurzLang) -> SHandIterator<SNextVecEPIShuffle> {
    let mut ithand = make_handiterator(slcstich, hand_fixed, epi_fixed, ekurzlang);
    assert!(ithand.vecepi.iter().is_sorted());
    SNextVecEPIShuffle::next(&mut ithand.vecepi); // initial shuffle
    ithand
}

#[test]
fn test_all_possible_hands() {
    use crate::card::card_values::*;
    let mut vecstich = Vec::new();
    for acard_stich in [[G7, G8, GA, G9], [S8, HO, S7, S9], [H7, HK, HU, SU], [EO, GO, HZ, H8]].into_iter() {
        vecstich.push(SStich::new(/*epi_first irrelevant*/EPlayerIndex::EPI0));
        completed_stichs(&vecstich); // asserts invariants. TODO introduce SStichSequence
        assert!(!current_stich(&vecstich).is_full());
        for card in acard_stich {
            current_stich_mut(&mut vecstich).push(*card);
        }
    }
    // see combinatorics.ods for computation of n_hand_count
    let epi_fixed = EPlayerIndex::EPI3;
    for atplcardslccardnan in [
        [
            (E9, vec![EA, GK, SK, SA], 34650, [4, 4, 4, 4]),
            (EK, vec![EA, GK, SK, SA], 11550, [3, 4, 4, 4]),
            (E8, vec![EA, GK, SK, SA], 4200,  [3, 3, 4, 4]),
            (EA, vec![EA, GK, SK, SA], 1680,  [3, 3, 3, 4]),
        ],
        [
            (SA, vec![HA, GK, SK], 1680, [3, 3, 3, 3]),
            (EU, vec![HA, GK, SK], 560,  [2, 3, 3, 3]),
            (SO, vec![HA, GK, SK], 210,  [2, 2, 3, 3]),
            (HA, vec![HA, GK, SK], 90,   [2, 2, 2, 3]),
        ],
        [
            (H9, vec![GK, SK], 90, [2, 2, 2, 2]),
            (GZ, vec![GK, SK], 30, [1, 2, 2, 2]),
            (GU, vec![GK, SK], 12, [1, 1, 2, 2]),
            (GK, vec![GK, SK], 6,  [1, 1, 1, 2]),
        ],
        [
            (EZ, vec![SK], 6, [1, 1, 1, 1]),
            (SZ, vec![SK], 2, [0, 1, 1, 1]),
            (E7, vec![SK], 1, [0, 0, 1, 1]),
            (SK, vec![SK], 1, [0, 0, 0, 1]),
        ],
    ].into_iter() {
        vecstich.push(SStich::new(/*epi_first irrelevant*/EPlayerIndex::EPI0));
        for (card, veccard_hand, n_hand_count, an_size_hand) in atplcardslccardnan {
            completed_stichs(&vecstich); // asserts invariants. TODO introduce SStichSequence
            assert!(!current_stich(&vecstich).is_full());
            let mut i_hand = 0;
            for ahand in all_possible_hands(
                &vecstich,
                SHand::new_from_vec(veccard_hand.iter().cloned().collect()),
                epi_fixed,
                EKurzLang::Lang,
            ) {
                i_hand+=1;
                assert_eq!(EnumMap::from_raw(an_size_hand.clone()), ahand.map(|hand| hand.cards().len()));
            }
            assert_eq!(i_hand, *n_hand_count);
            current_stich_mut(&mut vecstich).push(*card);
        }
    }
}

