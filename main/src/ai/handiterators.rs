use crate::ai::*;
use crate::primitives::*;
use crate::util::*;
use permutohedron::LexicalPermutation;
use rand::prelude::*;

pub trait TNextVecEPI {
    fn init(slcepi: &mut [EPlayerIndex]);
    fn next(slcepi: &mut [EPlayerIndex]) -> bool;
}

pub struct SNextVecEPIShuffle;
impl TNextVecEPI for SNextVecEPIShuffle {
    fn init(slcepi: &mut [EPlayerIndex]) {
        slcepi.shuffle(&mut rand::thread_rng());
    }
    fn next(slcepi: &mut [EPlayerIndex]) -> bool {
        Self::init(slcepi);
        true
    }
}

pub struct SNextVecEPIPermutation;
impl TNextVecEPI for SNextVecEPIPermutation {
    fn init(_slcepi: &mut [EPlayerIndex]) {
        // noop
    }
    fn next(slcepi: &mut [EPlayerIndex]) -> bool {
        slcepi.next_permutation()
    }
}

pub struct SHandIterator<NextVecEPI> {
    veccard_unknown: Vec<ECard>,
    vecepi: Vec<EPlayerIndex>,
    ahand_known: EnumMap<EPlayerIndex, SHand>,
    b_valid: bool,
    phantom: std::marker::PhantomData<NextVecEPI>,
}

impl<NextVecEPI: TNextVecEPI> Iterator for SHandIterator<NextVecEPI> {
    type Item = EnumMap<EPlayerIndex, SHand>;
    fn next(&mut self) -> Option<Self::Item> {
        if_then_some!(self.b_valid, {
            let mut ahand = self.ahand_known.clone();
            for (i, epi) in self.vecepi.iter().copied().enumerate() {
                ahand[epi].add_card(self.veccard_unknown[i]);
            }
            self.b_valid = NextVecEPI::next(self.vecepi.as_mut_slice());
            ahand
        })
    }
}

pub fn unplayed_cards<'lifetime>(
    stichseq: &'lifetime SStichSequence,
    ahand_fixed: &'lifetime EnumMap<EPlayerIndex, SHand>,
) -> impl Iterator<Item = ECard> + 'lifetime {
    ECard::values(stichseq.kurzlang()).filter(move |card| {
        !ahand_fixed.iter().any(|hand| hand.contains(*card))
            && !stichseq
                .visible_cards()
                .any(|(_epi, card_in_stich)| card_in_stich == card)
    })
}

#[test]
fn test_unplayed_cards() {
    use crate::card::ECard::*;
    let mut stichseq = SStichSequence::new(EKurzLang::Lang);
    for acard_stich in [
        [G7, G8, GA, G9],
        [S8, HO, S7, S9],
        [H7, HK, HU, SU],
        [EO, GO, HZ, H8],
        [E9, EK, E8, EA],
        [SA, EU, SO, HA],
    ] {
        for card in acard_stich {
            stichseq.zugeben(card, &SWinnerIndexIrrelevant);
        }
    }
    let veccard_unplayed = unplayed_cards(
        &stichseq,
        &SHand::new_from_iter([GK, SK]).to_ahand(EPlayerIndex::EPI0),
    )
    .collect::<Vec<_>>();
    let veccard_unplayed_check = [GZ, E7, SZ, H9, EZ, GU];
    assert_eq!(veccard_unplayed.len(), veccard_unplayed_check.len());
    assert!(veccard_unplayed
        .iter()
        .all(|card| veccard_unplayed_check.contains(card)));
    assert!(veccard_unplayed_check
        .iter()
        .all(|card| veccard_unplayed.contains(card)));
}

fn make_handiterator<NextVecEPI: TNextVecEPI>(
    stichseq: &SStichSequence,
    ahand_known: EnumMap<EPlayerIndex, SHand>,
) -> SHandIterator<NextVecEPI> {
    let veccard_unknown = unplayed_cards(stichseq, &ahand_known).collect::<Vec<_>>();
    let mapepin_cards_per_hand = stichseq.remaining_cards_per_hand();
    let mut vecepi = Vec::new();
    for epi in EPlayerIndex::values() {
        assert!(ahand_known[epi].cards().len() <= mapepin_cards_per_hand[epi]);
        let n_unknown_per_hand = mapepin_cards_per_hand[epi] - ahand_known[epi].cards().len();
        vecepi.extend(std::iter::repeat(epi).take(n_unknown_per_hand));
    }
    assert_eq!(veccard_unknown.len(), vecepi.len());
    assert!(vecepi.iter().is_sorted_unstable_name_collision());
    NextVecEPI::init(&mut vecepi);
    SHandIterator {
        veccard_unknown,
        vecepi,
        ahand_known,
        b_valid: true, // in the beginning, there should be a valid assignment of cards to players
        phantom: std::marker::PhantomData,
    }
}

fn make_handiterator_compatible_with_game_so_far<'lifetime, NextVecEPI: TNextVecEPI + 'lifetime, FnInspect: FnMut(bool/*b_valid*/, &EnumMap<EPlayerIndex, SHand>)->bool + 'lifetime>(
    stichseq: &'lifetime SStichSequence,
    ahand_known: EnumMap<EPlayerIndex, SHand>,
    rules: &'lifetime dyn TRules,
    slcstoss: &'lifetime [SStoss],
    mut fn_inspect: FnInspect,
) -> impl Iterator<Item = EnumMap<EPlayerIndex, SHand>> + 'lifetime {
    make_handiterator::<NextVecEPI>(stichseq, ahand_known).filter(move |ahand| {
        let b_valid = {
            let stich_current = stichseq.current_stich();
            assert!(!stich_current.is_full());
            assert!(ahand_vecstich_card_count_is_compatible(ahand, stichseq));
            // hands must not contain other cards preventing farbe/trumpf frei
            let aveccard = EPlayerIndex::map_from_fn(|epi| {
                let veccard : SHandVector = stichseq.cards_from_player(&ahand[epi], epi).copied().collect();
                assert_eq!(veccard.len(), stichseq.kurzlang().cards_per_player());
                veccard
            });
            rules.playerindex().map_or(true, |epi_active| {
                rules.can_be_played(SFullHand::new(
                    &aveccard[epi_active],
                    stichseq.kurzlang(),
                ))
            }) 
                && SGame::new(
                    /*aveccard*/aveccard,
                    SExpensifiersNoStoss::new(/*n_stock*/0),
                    rules.box_clone(),
                ).play_cards_and_stoss(
                    slcstoss,
                    stichseq.visible_cards(),
                    /*fn_before_zugeben*/|_game, _i_stich, _epi, _card| {},
                ).is_ok()
        };
        fn_inspect(b_valid, ahand) && b_valid
    })
}

pub trait TToAHand {
    fn to_ahand(self, epi_pri: EPlayerIndex) -> EnumMap<EPlayerIndex, SHand>;
}

impl TToAHand for EnumMap<EPlayerIndex, SHand> {
    fn to_ahand(self, _epi_pri: EPlayerIndex) -> EnumMap<EPlayerIndex, SHand> {
        self
    }
}
impl TToAHand for SHand {
    fn to_ahand(self, epi_pri: EPlayerIndex) -> EnumMap<EPlayerIndex, SHand> {
        EPlayerIndex::map_from_fn(|epi| {
            if epi == epi_pri {
                self.clone()
            } else {
                SHand::new_from_iter(None::<ECard>)
            }
        })
    }
}

pub fn internal_all_possible_hands<'lifetime>(
    stichseq: &'lifetime SStichSequence,
    tohand: impl TToAHand + 'lifetime,
    epi_fixed: EPlayerIndex,
    rules: &'lifetime dyn TRules,
    slcstoss: &'lifetime [SStoss],
    fn_inspect: impl FnMut(bool/*b_valid*/, &EnumMap<EPlayerIndex, SHand>)->bool + 'lifetime,
) -> impl Iterator<Item = EnumMap<EPlayerIndex, SHand>> + 'lifetime {
    make_handiterator_compatible_with_game_so_far::<SNextVecEPIPermutation, _>(
        stichseq,
        tohand.to_ahand(epi_fixed),
        rules,
        slcstoss,
        fn_inspect,
    )
}

pub fn all_possible_hands<'lifetime>(
    stichseq: &'lifetime SStichSequence,
    tohand: impl TToAHand + 'lifetime,
    epi_fixed: EPlayerIndex,
    rules: &'lifetime dyn TRules,
    slcstoss: &'lifetime [SStoss],
) -> impl Iterator<Item = EnumMap<EPlayerIndex, SHand>> + 'lifetime {
    internal_all_possible_hands(
        stichseq,
        tohand,
        epi_fixed,
        rules,
        slcstoss,
        /*fn_inspect*/|_b_valid, _ahand| true,
    )
}

pub fn internal_forever_rand_hands<'lifetime>(
    stichseq: &'lifetime SStichSequence,
    tohand: impl TToAHand,
    epi_fixed: EPlayerIndex,
    rules: &'lifetime dyn TRules,
    slcstoss: &'lifetime [SStoss],
    fn_inspect: impl FnMut(bool/*b_valid*/, &EnumMap<EPlayerIndex, SHand>)->bool + 'lifetime,
) -> impl Iterator<Item = EnumMap<EPlayerIndex, SHand>> + 'lifetime {
    make_handiterator_compatible_with_game_so_far::<SNextVecEPIShuffle, _>(
        stichseq,
        tohand.to_ahand(epi_fixed),
        rules,
        slcstoss,
        fn_inspect,
    )
}

pub fn forever_rand_hands<'lifetime>(
    stichseq: &'lifetime SStichSequence,
    tohand: impl TToAHand + 'lifetime,
    epi_fixed: EPlayerIndex,
    rules: &'lifetime dyn TRules,
    slcstoss: &'lifetime [SStoss],
) -> impl Iterator<Item = EnumMap<EPlayerIndex, SHand>> + 'lifetime {
    internal_forever_rand_hands(
        stichseq,
        tohand,
        epi_fixed,
        rules,
        slcstoss,
        /*fn_inspect*/|_b_valid, _ahand| true,
    )
}

#[test]
fn test_all_possible_hands() {
    use crate::card::ECard::*;
    let mut stichseq = SStichSequence::new(EKurzLang::Lang);
    for acard_stich in [
        [G7, G8, GA, G9],
        [S8, HO, S7, S9],
        [H7, HK, HU, SU],
        [EO, GO, HZ, H8],
    ] {
        for card in acard_stich {
            stichseq.zugeben(card, &SWinnerIndexIrrelevant);
        }
    }
    // see combinatorics.ods for computation of n_hand_count
    let epi_fixed = EPlayerIndex::EPI3;
    for atplcardslccardnan in [
        [
            (E9, vec![EA, GK, SK, SA], 34650, [4, 4, 4, 4]),
            (EK, vec![EA, GK, SK, SA], 11550, [3, 4, 4, 4]),
            (E8, vec![EA, GK, SK, SA], 4200, [3, 3, 4, 4]),
            (EA, vec![EA, GK, SK, SA], 1680, [3, 3, 3, 4]),
        ],
        [
            (SA, vec![HA, GK, SK], 1680, [3, 3, 3, 3]),
            (EU, vec![HA, GK, SK], 560, [2, 3, 3, 3]),
            (SO, vec![HA, GK, SK], 210, [2, 2, 3, 3]),
            (HA, vec![HA, GK, SK], 90, [2, 2, 2, 3]),
        ],
        [
            (H9, vec![GK, SK], 90, [2, 2, 2, 2]),
            (GZ, vec![GK, SK], 30, [1, 2, 2, 2]),
            (GU, vec![GK, SK], 12, [1, 1, 2, 2]),
            (GK, vec![GK, SK], 6, [1, 1, 1, 2]),
        ],
        [
            (EZ, vec![SK], 6, [1, 1, 1, 1]),
            (SZ, vec![SK], 2, [0, 1, 1, 1]),
            (E7, vec![SK], 1, [0, 0, 1, 1]),
            (SK, vec![SK], 1, [0, 0, 0, 1]),
        ],
    ] {
        for (card, veccard_hand, n_hand_count, an_size_hand) in atplcardslccardnan {
            assert_eq!(
                make_handiterator::<SNextVecEPIPermutation>(
                    &stichseq,
                    SHand::new_from_iter(veccard_hand).to_ahand(epi_fixed),
                )
                .inspect(|ahand| assert_eq!(
                    EnumMap::from_raw(an_size_hand),
                    ahand.map(|hand| hand.cards().len())
                ))
                .count(),
                n_hand_count
            );
            stichseq.zugeben(card, &SWinnerIndexIrrelevant);
        }
    }
}
