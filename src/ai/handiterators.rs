use crate::primitives::*;
use crate::util::*;
use permutohedron::LexicalPermutation;
use crate::ai::*;
use rand::prelude::*;

pub trait TNextVecEPI {
    fn init(slcepi: &mut[EPlayerIndex]);
    fn next(slcepi: &mut[EPlayerIndex]) -> bool;
}

pub struct SNextVecEPIShuffle;
impl TNextVecEPI for SNextVecEPIShuffle {
    fn init(slcepi: &mut[EPlayerIndex]) {
        slcepi.shuffle(&mut rand::thread_rng());
    }
    fn next(slcepi: &mut[EPlayerIndex]) -> bool {
        Self::init(slcepi);
        true
    }
}

pub struct SNextVecEPIPermutation;
impl TNextVecEPI for SNextVecEPIPermutation {
    fn init(_slcepi: &mut[EPlayerIndex]) {/*noop*/}
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

fn make_handiterator<NextVecEPI: TNextVecEPI>(stichseq: &SStichSequence, hand_fixed: SHand, epi_fixed: EPlayerIndex) -> SHandIterator<NextVecEPI> {
    let veccard_unknown = unplayed_cards(stichseq, &hand_fixed).collect::<Vec<_>>();
    let mapepin_cards_per_hand = remaining_cards_per_hand(stichseq);
    let mut vecepi = Vec::new();
    for epi in EPlayerIndex::values() {
        if epi==epi_fixed {
            assert_eq!(mapepin_cards_per_hand[epi], hand_fixed.cards().len());
        } else {
            vecepi.extend(std::iter::repeat(epi).take(mapepin_cards_per_hand[epi]));
        }
    }
    assert_eq!(veccard_unknown.len(), vecepi.len());
    assert!(vecepi.iter().is_sorted());
    NextVecEPI::init(&mut vecepi);
    SHandIterator {
        epi_fixed,
        veccard_unknown,
        vecepi,
        hand_known: hand_fixed,
        b_valid: true, // in the beginning, there should be a valid assignment of cards to players
        phantom: std::marker::PhantomData,
    }
}

fn make_handiterator_compatible_with_game_so_far<'lifetime, NextVecEPI: TNextVecEPI+'lifetime>(stichseq: &'lifetime SStichSequence, hand_fixed: SHand, epi_fixed: EPlayerIndex, rules: &'lifetime TRules) -> impl Iterator<Item=EnumMap<EPlayerIndex, SHand>> + 'lifetime {
    make_handiterator::<NextVecEPI>(stichseq, hand_fixed, epi_fixed)
        .filter(move |ahand| {
            let stich_current = stichseq.current_stich();
            assert!(!stich_current.is_full());
            assert!(ahand_vecstich_card_count_is_compatible(stichseq, ahand));
            // hands must not contain other cards preventing farbe/trumpf frei
            let mut ahand_simulate = EPlayerIndex::map_from_fn(|epi| {
                let mut veccard = ahand[epi].cards().clone();
                veccard.extend(stich_current.get(epi).cloned().into_iter());
                veccard.extend(stichseq.completed_stichs().iter().rev().map(|stich| stich[epi]));
                assert_eq!(veccard.len(), stichseq.kurzlang().cards_per_player());
                SHand::new_from_vec(veccard)
            });
            rules.playerindex().map_or(true, |epi_active|
                rules.can_be_played(SFullHand::new(&ahand_simulate[epi_active], stichseq.kurzlang()))
            )
            && {
                let mut b_valid_up_to_now = true;
                let mut stichseq_simulate = SStichSequence::new(stichseq.first_playerindex(), stichseq.kurzlang());
                'loopstich: for stich in stichseq.visible_stichs() {
                    for (epi, card) in stich.iter() {
                        if rules.card_is_allowed(
                            &stichseq_simulate,
                            &ahand_simulate[epi],
                            *card
                        ) {
                            assert!(ahand_simulate[epi].contains(*card));
                            ahand_simulate[epi].play_card(*card);
                            stichseq_simulate.zugeben(*card, rules);
                        } else {
                            b_valid_up_to_now = false;
                            break 'loopstich;
                        }
                    }
                }
                b_valid_up_to_now
            }
        })
}

pub fn all_possible_hands<'lifetime>(stichseq: &'lifetime SStichSequence, hand_fixed: SHand, epi_fixed: EPlayerIndex, rules: &'lifetime TRules) -> impl Iterator<Item=EnumMap<EPlayerIndex, SHand>> + 'lifetime {
    make_handiterator_compatible_with_game_so_far::<SNextVecEPIPermutation>(stichseq, hand_fixed, epi_fixed, rules)
}

pub fn forever_rand_hands<'lifetime>(stichseq: &'lifetime SStichSequence, hand_fixed: SHand, epi_fixed: EPlayerIndex, rules: &'lifetime TRules) -> impl Iterator<Item=EnumMap<EPlayerIndex, SHand>> + 'lifetime {
    make_handiterator_compatible_with_game_so_far::<SNextVecEPIShuffle>(stichseq, hand_fixed, epi_fixed, rules)
}

#[test]
fn test_all_possible_hands() {
    use crate::card::card_values::*;
    let epi_irrelevant = EPlayerIndex::EPI0;
    let mut stichseq = SStichSequence::new(epi_irrelevant, EKurzLang::Lang);
    for acard_stich in [[G7, G8, GA, G9], [S8, HO, S7, S9], [H7, HK, HU, SU], [EO, GO, HZ, H8]].iter() {
        for card in acard_stich {
            stichseq.zugeben_custom_winner_index(*card, |_stich| epi_irrelevant);
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
    ].iter() {
        for (card, veccard_hand, n_hand_count, an_size_hand) in atplcardslccardnan.iter() {
            let mut i_hand = 0;
            for ahand in make_handiterator::<SNextVecEPIPermutation>(
                &stichseq,
                SHand::new_from_vec(veccard_hand.iter().cloned().collect()),
                epi_fixed,
            ) {
                i_hand+=1;
                assert_eq!(EnumMap::from_raw(*an_size_hand), ahand.map(|hand| hand.cards().len()));
            }
            assert_eq!(i_hand, *n_hand_count);
            stichseq.zugeben_custom_winner_index(*card, |_stich| epi_irrelevant);
        }
    }
}

