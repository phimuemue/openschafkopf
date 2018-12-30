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
            self.b_valid = NextVecEPI::next(self.vecepi.as_mut_slice());
            Some(ahand)
        } else {
            assert!(!self.vecepi[..].next_permutation());
            None
        }
    }
}

fn make_handiterator<NextVecEPI: TNextVecEPI>(slcstich: SCompletedStichs, hand_fixed: SHand, epi_fixed: EPlayerIndex, ekurzlang: EKurzLang) -> SHandIterator<NextVecEPI> {
    let veccard_unknown = unplayed_cards(slcstich.get(), &hand_fixed, ekurzlang).collect::<Vec<_>>();
    let n_cards_total = veccard_unknown.len();
    assert_eq!(n_cards_total%3, 0);
    let n_cards_per_player = n_cards_total / 3;
    SHandIterator {
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
        phantom: std::marker::PhantomData,
    }
}

pub fn all_possible_hands(slcstich: SCompletedStichs, hand_fixed: SHand, epi_fixed: EPlayerIndex, ekurzlang: EKurzLang) -> SHandIterator<SNextVecEPIPermutation> {
    let ithand = make_handiterator(slcstich, hand_fixed, epi_fixed, ekurzlang);
    assert!(ithand.vecepi.iter().is_sorted());
    ithand
}

pub fn forever_rand_hands(slcstich: SCompletedStichs, hand_fixed: SHand, epi_fixed: EPlayerIndex, ekurzlang: EKurzLang) -> SHandIterator<SNextVecEPIShuffle> {
    let mut ithand = make_handiterator(slcstich, hand_fixed, epi_fixed, ekurzlang);
    assert!(ithand.vecepi.iter().is_sorted());
    SNextVecEPIShuffle::next(&mut ithand.vecepi); // initial shuffle
    ithand
}

#[test]
fn test_all_possible_hands() {
    use crate::card::card_values::*;
    let acard_to_stich = |acard: [SCard; 4]| {
        SStich::new_full(/*epi_first irrelevant*/EPlayerIndex::EPI0, acard)
    };
    let mut vecstich = [[G7, G8, GA, G9], [S8, HO, S7, S9], [H7, HK, HU, SU], [EO, GO, HZ, H8]].into_iter()
        .map(|acard| acard_to_stich(*acard))
        .collect::<Vec<_>>();
    let mut add_stich_and_test = |acard_stich, slccard_hand: &[SCard], n_count, an_size_hand| {
        vecstich.push(acard_to_stich(acard_stich));
        let mut i_hand = 0;
        for ahand in all_possible_hands(
            SCompletedStichs::new(&vecstich),
            SHand::new_from_vec(slccard_hand.iter().cloned().collect()),
            EPlayerIndex::EPI0, // epi_fixed
            EKurzLang::Lang,
        ) {
            i_hand+=1;
            assert_eq!(EnumMap::from_raw(an_size_hand), ahand.map(|hand| hand.cards().len()));
        }
        assert_eq!(i_hand, n_count);
    };
    // 3*n unknown cards distributed among three players: binomial(3n,3)*binomial(2n,n) possibilities
    add_stich_and_test([E9, EK, E8, EA], &[GK, SK, SA], 1680, [3, 3, 3, 3]);
    add_stich_and_test([SA, EU, SO, HA], &[GK, SK], 90, [2, 2, 2, 2]);
    add_stich_and_test([H9, GZ, GU, GK], &[SK], 6, [1, 1, 1, 1]);
}

