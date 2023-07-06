use crate::ai::*;
use crate::primitives::*;
use crate::util::*;
use permutohedron::LexicalPermutation;
use rand::{
    prelude::*,
    distributions::WeightedIndex,
};

pub trait THandIteratorCore {
    type InitParam;
    fn new(stichseq: &SStichSequence, ahand_known: &EnumMap<EPlayerIndex, SHand>, mapepin_count_unplayed_unknown: EnumMap<EPlayerIndex, usize>, veccard_unplayed_unknown: Vec<ECard>, initparam: Self::InitParam) -> Self;
    fn next(&mut self, fn_add_card_to_hand: impl FnMut(EPlayerIndex, ECard)) -> bool;
}

pub struct SHandIteratorCoreShuffle {
    mapepin_count_unplayed_unknown: EnumMap<EPlayerIndex, usize>,
    veccard_unplayed_unknown: Vec<ECard>,
}
impl THandIteratorCore for SHandIteratorCoreShuffle {
    type InitParam = ();
    fn new(_stichseq: &SStichSequence, _ahand_known: &EnumMap<EPlayerIndex, SHand>, mapepin_count_unplayed_unknown: EnumMap<EPlayerIndex, usize>, veccard_unplayed_unknown: Vec<ECard>, _initparam: Self::InitParam) -> Self {
        Self {
            mapepin_count_unplayed_unknown,
            veccard_unplayed_unknown,
        }
    }
    fn next(&mut self, mut fn_add_card_to_hand: impl FnMut(EPlayerIndex, ECard)) -> bool {
        self.veccard_unplayed_unknown.shuffle(&mut rand::thread_rng());
        let mut i = 0;
        for epi in EPlayerIndex::values() {
            for _ in 0..self.mapepin_count_unplayed_unknown[epi] {
                fn_add_card_to_hand(epi, self.veccard_unplayed_unknown[i]);
                i += 1;
            }
        }
        true
    }
}

type SCardHistogram = EnumMap<ECard, EnumMap<EPlayerIndex, usize>>;
pub struct SHandIteratorCoreCardHistogram {
    mapepin_count_unplayed_unknown: EnumMap<EPlayerIndex, usize>,
    veccard_unplayed_unknown: Vec<ECard>,
    cardhist_unplayed_unknown: SCardHistogram,
}
impl THandIteratorCore for SHandIteratorCoreCardHistogram {
    type InitParam = SCardHistogram;
    fn new(stichseq: &SStichSequence, ahand_known: &EnumMap<EPlayerIndex, SHand>, mapepin_count_unplayed_unknown: EnumMap<EPlayerIndex, usize>, veccard_unplayed_unknown: Vec<ECard>, cardhist_raw: Self::InitParam) -> Self {
        let mapepin_cards_remaining = stichseq.remaining_cards_per_hand();
        let mapepib_complete_hand /*TODO enumset*/ = EPlayerIndex::map_from_fn(|epi|
            mapepin_cards_remaining[epi]==ahand_known[epi].cards().len()
        );
        let mut cardhist_unplayed_unknown = cardhist_raw;
        let mapepin_invalid_hist = EPlayerIndex::map_from_fn(|_| 0);
        #[cfg(debug_assertions)] {
            for (_epi, &card) in stichseq.visible_cards() {
                cardhist_unplayed_unknown[card] = mapepin_invalid_hist;
            }
            for hand in ahand_known.as_raw() {
                for &card in hand.cards() {
                    cardhist_unplayed_unknown[card] = mapepin_invalid_hist;
                }
            }
        }
        for &card in &veccard_unplayed_unknown {
            assert_ne!(cardhist_unplayed_unknown[card], mapepin_invalid_hist);
            for epi in EPlayerIndex::values() {
                if mapepib_complete_hand[epi] {
                    cardhist_unplayed_unknown[card][epi] = 0;
                }
            }
        }
        Self {
            mapepin_count_unplayed_unknown,
            veccard_unplayed_unknown,
            cardhist_unplayed_unknown,
        }
    }
    fn next(&mut self, mut fn_add_card_to_hand: impl FnMut(EPlayerIndex, ECard)) -> bool {
        let mut rng = rand::thread_rng();
        let mapepiveccard = 'outer: loop {
            let mut mapepiveccard = EPlayerIndex::map_from_fn(|_epi| Vec::new());
            for &card in &self.veccard_unplayed_unknown {
                let epi = unwrap!(EPlayerIndex::checked_from_usize({
                    unwrap!(WeightedIndex::new(
                        self.cardhist_unplayed_unknown[card].as_raw(),
                    )).sample(&mut rng)
                }));
                if mapepiveccard[epi].len() < self.mapepin_count_unplayed_unknown[epi] {
                    mapepiveccard[epi].push(card);
                } else {
                    continue 'outer;
                }
            }
            break mapepiveccard;
        };
        for epi in EPlayerIndex::values() {
            assert_eq!(mapepiveccard[epi].len(), self.mapepin_count_unplayed_unknown[epi]);
            for &card in &mapepiveccard[epi] {
                fn_add_card_to_hand(epi, card);
            }
        }
        true
    }
}

pub struct SHandIteratorCorePermutation {
    vecepi: Vec<EPlayerIndex>,
    veccard_unplayed_unknown: Vec<ECard>,
}
impl THandIteratorCore for SHandIteratorCorePermutation {
    type InitParam = ();
    fn new(_stichseq: &SStichSequence, _ahand_known: &EnumMap<EPlayerIndex, SHand>, mapepin_count_unplayed_unknown: EnumMap<EPlayerIndex, usize>, veccard_unplayed_unknown: Vec<ECard>, _initparam: Self::InitParam) -> Self {
        let mut vecepi = Vec::new();
        for epi in EPlayerIndex::values() {
            vecepi.extend(std::iter::repeat(epi).take(mapepin_count_unplayed_unknown[epi]));
        }
        assert_eq!(veccard_unplayed_unknown.len(), vecepi.len());
        assert!(vecepi.iter().is_sorted_unstable_name_collision());
        Self {
            vecepi,
            veccard_unplayed_unknown,
        }
    }
    fn next(&mut self, mut fn_add_card_to_hand: impl FnMut(EPlayerIndex, ECard)) -> bool {
        for (epi, card) in itertools::zip_eq(&self.vecepi, &self.veccard_unplayed_unknown) {
            fn_add_card_to_hand(*epi, *card);
        }
        self.vecepi.next_permutation()
    }
}

pub struct SHandIterator<HandIteratorCore> {
    ahand_known: EnumMap<EPlayerIndex, SHand>,
    b_valid: bool,
    handitercore: HandIteratorCore,
}

impl<HandIteratorCore: THandIteratorCore> Iterator for SHandIterator<HandIteratorCore> {
    type Item = EnumMap<EPlayerIndex, SHand>;
    fn next(&mut self) -> Option<Self::Item> {
        if_then_some!(self.b_valid, {
            let mut ahand = self.ahand_known.clone();
            self.b_valid = self.handitercore.next(
                /*fn_add_card_to_hand*/|epi, card| ahand[epi].add_card(card),
            );
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

fn make_handiterator<HandIteratorCore: THandIteratorCore>(
    stichseq: &SStichSequence,
    ahand_known: EnumMap<EPlayerIndex, SHand>,
    initparam: HandIteratorCore::InitParam,
) -> SHandIterator<HandIteratorCore> {
    let veccard_unplayed_unknown = unplayed_cards(stichseq, &ahand_known).collect::<Vec<_>>();
    let mut mapepin_count_unplayed_unknown = stichseq.remaining_cards_per_hand();
    for epi in EPlayerIndex::values() {
        assert!(ahand_known[epi].cards().len() <= mapepin_count_unplayed_unknown[epi]);
        mapepin_count_unplayed_unknown[epi] -= ahand_known[epi].cards().len();
    }
    assert_eq!(
        veccard_unplayed_unknown.len(),
        mapepin_count_unplayed_unknown.iter().sum::<usize>(),
    );
    let handitercore = HandIteratorCore::new(
        stichseq,
        &ahand_known,
        mapepin_count_unplayed_unknown,
        veccard_unplayed_unknown,
        initparam,
    );
    SHandIterator {
        ahand_known,
        b_valid: true, // in the beginning, there should be a valid assignment of cards to players
        handitercore,
    }
}

fn make_handiterator_compatible_with_game_so_far<'lifetime, HandIteratorCore: THandIteratorCore + 'lifetime>(
    stichseq: &'lifetime SStichSequence,
    ahand_known: EnumMap<EPlayerIndex, SHand>,
    rules: &'lifetime dyn TRules,
    slcstoss: &'lifetime [SStoss],
    mut fn_inspect: impl FnMut(bool/*b_valid*/, &EnumMap<EPlayerIndex, SHand>)->bool + 'lifetime,
    initparam: HandIteratorCore::InitParam,
) -> impl Iterator<Item = EnumMap<EPlayerIndex, SHand>> + 'lifetime {
    make_handiterator::<HandIteratorCore>(stichseq, ahand_known, initparam).filter(move |ahand| {
        let b_valid = {
            assert!(ahand_vecstich_card_count_is_compatible(ahand, stichseq));
            // hands must not contain other cards preventing farbe/trumpf frei
            let aveccard = EPlayerIndex::map_from_fn(|epi| {
                let veccard : SHandVector = stichseq.cards_from_player(&ahand[epi], epi).collect();
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
    make_handiterator_compatible_with_game_so_far::<SHandIteratorCorePermutation>(
        stichseq,
        tohand.to_ahand(epi_fixed),
        rules,
        slcstoss,
        fn_inspect,
        (),
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

pub fn forever_hands_according_to_distribution<'lifetime>(
    stichseq: &'lifetime SStichSequence,
    tohand: impl TToAHand,
    epi_fixed: EPlayerIndex,
    rules: &'lifetime dyn TRules,
    slcstoss: &'lifetime [SStoss],
    fn_inspect: impl FnMut(bool/*b_valid*/, &EnumMap<EPlayerIndex, SHand>)->bool + 'lifetime,
) -> impl Iterator<Item = EnumMap<EPlayerIndex, SHand>> + 'lifetime {
    make_handiterator_compatible_with_game_so_far::<SHandIteratorCoreCardHistogram>(
        stichseq,
        tohand.to_ahand(epi_fixed),
        rules,
        slcstoss,
        fn_inspect,
        /*TODO*/ECard::map_from_fn(|card| EPlayerIndex::map_from_raw(
            match card.farbe() {
                EFarbe::Eichel => [16, 4, 2, 1],
                EFarbe::Gras => [1, 16, 4, 2],
                EFarbe::Herz => [2, 1, 16, 4],
                EFarbe::Schelln => [4, 2, 1, 16],
            }
        )),
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
    make_handiterator_compatible_with_game_so_far::<SHandIteratorCoreShuffle>(
        stichseq,
        tohand.to_ahand(epi_fixed),
        rules,
        slcstoss,
        fn_inspect,
        (),
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
                make_handiterator::<SHandIteratorCorePermutation>(
                    &stichseq,
                    SHand::new_from_iter(veccard_hand).to_ahand(epi_fixed),
                    (),
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
