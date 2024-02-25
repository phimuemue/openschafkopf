pub mod handiterators;
pub mod rulespecific;
pub mod gametree;
#[cfg(test)]
pub mod test;
pub mod stichoracle;
pub mod cardspartition;

use crate::ai::{handiterators::*, gametree::*};
pub use gametree::SPerMinMaxStrategy;
use crate::game::*;
use crate::primitives::*;
use crate::rules::*;
use crate::util::*;
use itertools::Itertools;
use rand::prelude::*;
use rayon::prelude::*;
use std::{
    self,
    fmt::Debug,
    sync::{Arc, Mutex},
    collections::BTreeMap,
};

pub fn ahand_vecstich_card_count_is_compatible(ahand: &EnumMap<EPlayerIndex, SHand>, stichseq: &SStichSequence) -> bool {
    ahand.map(|hand| hand.cards().len()) == stichseq.remaining_cards_per_hand()
}

pub enum VAIParams {
    Cheating,
    Simulating {
        n_suggest_card_samples: usize,
    },
}

pub struct SAi {
    n_rank_rules_samples: usize,
    n_suggest_card_branches: usize,
    aiparams: VAIParams,
}

impl SAi {
    pub fn new_cheating(n_rank_rules_samples: usize, n_suggest_card_branches: usize) -> Self {
        SAi {
            n_rank_rules_samples,
            n_suggest_card_branches,
            aiparams: VAIParams::Cheating,
        }
    }

    pub fn new_simulating(n_rank_rules_samples: usize, n_suggest_card_branches: usize, n_suggest_card_samples: usize) -> Self {
        SAi {
            n_rank_rules_samples,
            n_suggest_card_branches,
            aiparams: VAIParams::Simulating {
                n_suggest_card_samples,
            },
        }
    }

    pub fn rank_rules(&self, hand_fixed: SFullHand, epi_rank: EPlayerIndex, rules: &SRules, expensifiers: &SExpensifiers) -> SPerMinMaxStrategy<SPayoutStats<()>> {
        // TODO: adjust interface to get whole game in case of VAIParams::Cheating
        let ekurzlang = unwrap!(EKurzLang::from_cards_per_player(hand_fixed.get().len()));
        forever_rand_hands(
            &SStichSequence::new(ekurzlang),
            (SHand::new_from_iter(hand_fixed.get()), epi_rank),
            rules,
            &expensifiers.vecstoss,
        )
            .take(self.n_rank_rules_samples)
            .par_bridge() // TODO can we derive a true parallel iterator?
            .map(|mut ahand| {
                explore_snapshots(
                    (&mut ahand, &mut SStichSequence::new(ekurzlang)),
                    rules,
                    &SBranchingFactor::factory(1, 2),
                    &SMinReachablePayoutLowerBoundViaHint::new(
                        rules,
                        epi_rank,
                        expensifiers.clone(), // TODO? can clone be avoided
                    ),
                    &SSnapshotCacheNone::factory(), // TODO make customizable
                    &mut SNoVisualization{},
                ).map(|mapepiminmax| {
                    SPayoutStats::new_1((mapepiminmax[epi_rank], ()))
                })
            })
            .reduce(
                /*identity*/|| SPerMinMaxStrategy::new(SPayoutStats::new_identity_for_accumulate()),
                /*op*/mutate_return!(|perminmaxstrategypayoutstats_lhs, perminmaxstrategypayoutstats_rhs| {
                    perminmaxstrategypayoutstats_lhs.modify_with_other(
                        &perminmaxstrategypayoutstats_rhs,
                        SPayoutStats::accumulate,
                    );
                }),
            )
    }

    fn suggest_card_internal<SnapshotVisualizer: TSnapshotVisualizer<SMaxMinMaxSelfishMin<EnumMap<EPlayerIndex, isize>>>>(
        &self,
        rules: &SRules,
        stichseq: &SStichSequence,
        ahand: &EnumMap<EPlayerIndex, SHand>,
        expensifiers: &SExpensifiers,
        fn_visualizer: impl Fn(usize, &EnumMap<EPlayerIndex, SHand>, Option<ECard>) -> SnapshotVisualizer + std::marker::Sync,
    ) -> ECard {
        let epi_current = unwrap!(stichseq.current_stich().current_playerindex());
        let hand_fixed = &ahand[epi_current];
        if let Ok(card)=rules.all_allowed_cards(
            stichseq,
            hand_fixed
        ).iter().exactly_one() {
            *card
        } else if let Some(card) = rules.rulespecific_ai()
            .and_then(|airulespecific| airulespecific.suggest_card(hand_fixed, stichseq))
        {
            card
        } else {
            macro_rules! forward_to_determine_best_card{(
                ($func_filter_allowed_cards: expr, $foreachsnapshot: ty,),
                $itahand: expr,
            ) => {{ // TODORUST generic closures
                determine_best_card(
                    &stichseq,
                    Box::new($itahand) as Box<_>,
                    $func_filter_allowed_cards,
                    &|_stichseq, _ahand| <$foreachsnapshot>::new(
                        rules,
                        epi_current,
                        expensifiers.clone(),
                    ),
                    SSnapshotCacheNone::factory(), // TODO possibly use cache
                    fn_visualizer,
                    /*fn_inspect*/&|_b_before, _i_ahand, _ahand, _card| {},
                    /*fn_payout*/&|_stichseq, _ahand, n_payout| (n_payout, ()),
                )
            }}}
            let n_remaining_cards = stichseq.remaining_cards_per_hand()[epi_current];
            assert!(0<n_remaining_cards);
            let vecstoss = &expensifiers.vecstoss;
            *unwrap!(unwrap!(cartesian_match!(
                forward_to_determine_best_card,
                match (n_remaining_cards) {
                    1..=3 => (
                        SNoFilter::factory(),
                        SMinReachablePayoutBase::<SPrunerNothing, SMaxMinMaxSelfishMinHigherKinded, /*TODO*/SAlphaBetaPrunerNone>,
                    ),
                    4 => (
                        SNoFilter::factory(),
                        SMinReachablePayoutBase::<SPrunerViaHint, SMaxMinMaxSelfishMinHigherKinded, /*TODO*/SAlphaBetaPrunerNone>,
                    ),
                    _ => (
                        SBranchingFactor::factory(1, self.n_suggest_card_branches+1),
                        SMinReachablePayoutBase::<SPrunerViaHint, SMaxMinMaxSelfishMinHigherKinded, /*TODO*/SAlphaBetaPrunerNone>,
                    ),
                },
                match ((&self.aiparams, n_remaining_cards)) {
                    (&VAIParams::Cheating, _) => {
                        std::iter::once(ahand.clone())
                    },
                    (&VAIParams::Simulating{n_suggest_card_samples:_}, 1..=4) => {
                        all_possible_hands(stichseq, (hand_fixed.clone(), epi_current), rules, vecstoss)
                    },
                    (&VAIParams::Simulating{n_suggest_card_samples}, _) =>{ 
                        forever_rand_hands(stichseq, (hand_fixed.clone(), epi_current), rules, vecstoss)
                            .take(n_suggest_card_samples)
                    },
                },
            )).cards_with_maximum_value(|lhs, rhs| {
                SMaxMinMaxSelfishMin::compare_canonical( // TODO good idea?
                    lhs,
                    rhs,
                    |n_payout, ()| n_payout.cmp(&0), // TODO is this even correct?
                )
            }).0.first())
        }
    }

    pub fn suggest_card<SnapshotVisualizer: TSnapshotVisualizer<SMaxMinMaxSelfishMin<EnumMap<EPlayerIndex, isize>>>, Ruleset, GameAnnouncements, DetermineRules>(
        &self,
        game: &SGameGeneric<Ruleset, GameAnnouncements, DetermineRules>,
        fn_visualizer: impl Fn(usize, &EnumMap<EPlayerIndex, SHand>, Option<ECard>) -> SnapshotVisualizer + std::marker::Sync,
    ) -> ECard {
        self.suggest_card_internal(
            &game.rules,
            &game.stichseq,
            &game.ahand,
            &game.expensifiers,
            fn_visualizer,
        )
    }
}

#[derive(Clone, Debug)]
pub struct SDetermineBestCardResult<T> {
    mapcardt: EnumMap<ECard, Option<T>>,
}

impl<T> SDetermineBestCardResult<T> {
    pub fn cards_and_ts(&self) -> impl Iterator<Item=(ECard, &T)> where T: Debug {
        <ECard as PlainEnum>::values()
            .filter_map(|card| self.mapcardt[card].as_ref().map(|t| (card, t)))
    }
    pub fn cards_with_maximum_value(&self, mut fn_cmp: impl FnMut(&T, &T)->std::cmp::Ordering) -> (Vec<ECard>, &T) where T: Debug {
        let veccard = <ECard as PlainEnum>::values()
            .filter(|card| self.mapcardt[*card].is_some())
            .max_set_by(|card_lhs, card_rhs| fn_cmp(
                unwrap!(self.mapcardt[*card_lhs].as_ref()),
                unwrap!(self.mapcardt[*card_rhs].as_ref()),
            ));
        assert!(!veccard.is_empty());
        let t = unwrap!(self.mapcardt[veccard[0]].as_ref());
        (veccard, t)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SPayoutStats<T: Ord> {
    mapnn_histogram: BTreeMap<(isize/*n_payout*/, T), usize/*n_count*/>, // TODO manually sorted Vec better?
}

impl<T: Ord + Copy> SPayoutStats<T> {
    pub fn new_1((n_payout, t): (isize, T)) -> Self {
        Self {
            mapnn_histogram: Some(((n_payout, t), 1)).into_iter().collect()
        }
    }

    pub fn new_identity_for_accumulate() -> Self {
        Self {
            mapnn_histogram: BTreeMap::new(),
        }
    }

    pub fn accumulate(&mut self, paystats: &Self) {
        for ((n_payout_other, t), n_count_other) in paystats.mapnn_histogram.iter() {
            *self.mapnn_histogram.entry((*n_payout_other, *t)).or_insert(0) += n_count_other;
        }
    }

    pub fn min(&self) -> isize {
        unwrap!(self.mapnn_histogram.keys().next()).0
    }
    pub fn max(&self) -> isize {
        unwrap!(self.mapnn_histogram.keys().last()).0
    }
    pub fn avg(&self) -> f32 {
        let (n_payout_sum, n_count_sum) = self.mapnn_histogram.iter()
            .fold((0, 0), |(n_payout_sum, n_count_sum), ((n_payout, _t), n_count)| (
                n_payout_sum + n_payout * n_count.as_num::<isize>(),
                n_count_sum + n_count,
            ));
        n_payout_sum.as_num::<f32>() / n_count_sum.as_num::<f32>()
    }
    pub fn histogram(&self) -> &BTreeMap<(isize, T), usize> {
        &self.mapnn_histogram
    }
    pub fn counts(&self, fn_loss_or_win: impl Fn(isize, T)->std::cmp::Ordering) -> EnumMap<std::cmp::Ordering, usize> {
        let mut mapordn_counts = std::cmp::Ordering::map_from_fn(|_ord| 0);
        for ((n_payout, t), n_count) in self.mapnn_histogram.iter() {
            mapordn_counts[fn_loss_or_win(*n_payout, *t)] += n_count;
        }
        mapordn_counts
    }
}

pub fn determine_best_card<
    'stichseq,
    'rules,
    FilterAllowedCards: TFilterAllowedCards,
    MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded,
    AlphaBetaPruner: TAlphaBetaPruner+Sync,
    Pruner: TPruner+Sync,
    SnapshotCache: TSnapshotCache<<SMinReachablePayoutBase<'rules, Pruner, MinMaxStrategiesHK, AlphaBetaPruner> as TForEachSnapshot>::Output>,
    OSnapshotCache: Into<Option<SnapshotCache>>,
    SnapshotVisualizer: TSnapshotVisualizer<<SMinReachablePayoutBase<'rules, Pruner, MinMaxStrategiesHK, AlphaBetaPruner> as TForEachSnapshot>::Output>,
    OFilterAllowedCards: Into<Option<FilterAllowedCards>>,
    PayoutStatsPayload: Ord + Copy + Sync + Send,
>(
    stichseq: &'stichseq SStichSequence,
    itahand: Box<dyn Iterator<Item=EnumMap<EPlayerIndex, SHand>> + Send + 'stichseq>,
    fn_make_filter: impl Fn(&SStichSequence, &EnumMap<EPlayerIndex, SHand>)->OFilterAllowedCards + std::marker::Sync,
    fn_make_foreachsnapshot: &(dyn Fn(&SStichSequence, &EnumMap<EPlayerIndex, SHand>) -> SMinReachablePayoutBase<'rules, Pruner, MinMaxStrategiesHK, AlphaBetaPruner> + std::marker::Sync),
    fn_snapshotcache: impl Fn(&SRuleStateCacheFixed) -> OSnapshotCache + std::marker::Sync,
    fn_visualizer: impl Fn(usize, &EnumMap<EPlayerIndex, SHand>, Option<ECard>) -> SnapshotVisualizer + std::marker::Sync,
    fn_inspect: &(dyn Fn(bool/*b_before*/, usize, &EnumMap<EPlayerIndex, SHand>, ECard) + std::marker::Sync),
    fn_payout: &(impl Fn(&SStichSequence, &EnumMap<EPlayerIndex, SHand>, isize)->(isize, PayoutStatsPayload) + Sync),
) -> Option<SDetermineBestCardResult<MinMaxStrategiesHK::Type<SPayoutStats<PayoutStatsPayload>>>>
    where
        <SMinReachablePayoutBase<'rules, Pruner, MinMaxStrategiesHK, AlphaBetaPruner> as TForEachSnapshot>::Output: TMinMaxStrategies<MinMaxStrategiesHK, Arg0=EnumMap<EPlayerIndex, isize>>,
        MinMaxStrategiesHK::Type<SPayoutStats<PayoutStatsPayload>>: Send,
        MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>: TMinMaxStrategiesInternal<MinMaxStrategiesHK>,
        AlphaBetaPruner: Sync,
{
    let mapcardooutput = Arc::new(Mutex::new(
        // aggregate n_payout per card in some way
        ECard::map_from_fn(|_card| None),
    ));
    let epi_current = unwrap!(stichseq.current_stich().current_playerindex());
    itahand
        .enumerate()
        .flat_map(|(i_ahand, ahand)| {
            let foreachsnapshot = fn_make_foreachsnapshot(stichseq, &ahand);
            foreachsnapshot.rules.all_allowed_cards(
                stichseq,
                &ahand[epi_current],
            )
                .into_iter()
                .map(move |card| (i_ahand, ahand.clone(), card))
        })
        .par_bridge() // TODO can we derive a true parallel iterator?
        .for_each(|(i_ahand, mut ahand, card)| {
            fn_inspect(/*b_before*/true, i_ahand, &ahand, card);
            let foreachsnapshot = fn_make_foreachsnapshot(stichseq, &ahand);
            let mut visualizer = fn_visualizer(i_ahand, &ahand, Some(card)); // do before ahand is modified
            debug_assert!(ahand[epi_current].cards().contains(&card));
            let mut stichseq = stichseq.clone();
            assert!(ahand_vecstich_card_count_is_compatible(&ahand, &stichseq));
            let rules = foreachsnapshot.rules;
            let output = stichseq.zugeben_and_restore_with_hands(&mut ahand, epi_current, card, rules, |ahand, stichseq| {
                if ahand.iter().all(|hand| hand.cards().is_empty()) {
                    let stichseq_finished = SStichSequenceGameFinished::new(stichseq);
                    foreachsnapshot.final_output(
                        stichseq_finished,
                        &SRuleStateCache::new_from_gamefinishedstiche(
                            stichseq_finished,
                            rules,
                        ),
                    )
                } else {
                    explore_snapshots(
                        (ahand, stichseq),
                        rules,
                        &fn_make_filter,
                        &foreachsnapshot,
                        &fn_snapshotcache,
                        &mut visualizer,
                    )
                }
            });
            let payoutstats : MinMaxStrategiesHK::Type<SPayoutStats<PayoutStatsPayload>> = output.map(|mapepin_payout|
                SPayoutStats::new_1(fn_payout(&stichseq, &ahand, mapepin_payout[foreachsnapshot.epi]))
            );
            let mapcardooutput = Arc::clone(&mapcardooutput);
            let ooutput = &mut unwrap!(mapcardooutput.lock())[card];
            match ooutput {
                None => *ooutput = Some(payoutstats),
                Some(ref mut output_return) => {
                    output_return.modify_with_other(
                        &payoutstats,
                        |lhs, rhs| SPayoutStats::accumulate(lhs, rhs),
                    );
                },
            }
            fn_inspect(/*b_before*/false, i_ahand, &ahand, card);
        });
    let mapcardooutput = unwrap!(
        unwrap!(Arc::into_inner(mapcardooutput)) // "Returns the inner value, if the Arc has exactly one strong reference"   
            .into_inner() // "If another user of this mutex panicked while holding the mutex, then this call will return an error instead"
    );
    if_then_some!(mapcardooutput.iter().any(Option::is_some), {
        SDetermineBestCardResult{
            mapcardt: mapcardooutput,
        }
    })
}

pub struct SBranchingFactor{
    intvln: SInterval<usize>,
}
impl TFilterAllowedCards for SBranchingFactor {
    type UnregisterStich = ();
    fn register_stich(&mut self, _ahand: &mut EnumMap<EPlayerIndex, SHand>, _stichseq: &mut SStichSequence) -> Self::UnregisterStich {}
    fn unregister_stich(&mut self, _unregisterstich: Self::UnregisterStich) {}
    fn filter_allowed_cards(&self, _stichseq: &SStichSequence, veccard: &mut SHandVector) {
        assert!(!veccard.is_empty());
        let mut rng = rand::thread_rng();
        let n = rng.gen_range(self.intvln[ELoHi::Lo]..self.intvln[ELoHi::Hi]);
        while n<veccard.len() {
            veccard.swap_remove(rng.gen_range(0..veccard.len()));
        }
    }
}
impl SBranchingFactor {
    pub fn factory(n_lo: usize, n_hi: usize) -> impl Fn(&SStichSequence, &EnumMap<EPlayerIndex, SHand>)->Self {
        assert!(n_lo < n_hi);
        move |_,_| Self {intvln: SInterval::from_raw([n_lo, n_hi])}
    }
}

#[test]
fn test_is_compatible_with_game_so_far() {
    use crate::rules::rulesrufspiel::*;
    use crate::rules::payoutdecider::*;
    use crate::primitives::card::ECard::*;
    use crate::game;
    enum VTestAction {
        PlayStich([ECard; EPlayerIndex::SIZE]),
        AssertFrei(EPlayerIndex, VTrumpfOrFarbe),
        AssertNotFrei(EPlayerIndex, VTrumpfOrFarbe),
    }
    let test_game = |aacard_hand: [[ECard; EKurzLang::Lang.cards_per_player()]; EPlayerIndex::SIZE], rules: &SRules, vectestaction: Vec<VTestAction>| {
        // TODO implement tests for SStoss
        let ahand = EPlayerIndex::map_from_raw(aacard_hand)
            .map_into(|acard| acard.into());
        let mut game = game::SGame::new(
            ahand,
            SExpensifiersNoStoss::new(/*n_stock*/ 0),
            rules.clone(),
        );
        let mut vectplepitrumpforfarbe_frei = Vec::new();
        for testaction in vectestaction {
            let mut oassertnotfrei = None;
            match testaction {
                VTestAction::PlayStich(acard) => {
                    for card in acard.iter() {
                        let epi = unwrap!(game.which_player_can_do_something()).0;
                        unwrap!(game.zugeben(*card, epi));
                    }
                },
                VTestAction::AssertFrei(epi, trumpforfarbe) => {
                    vectplepitrumpforfarbe_frei.push((epi, trumpforfarbe));
                },
                VTestAction::AssertNotFrei(epi, trumpforfarbe) => {
                    oassertnotfrei = Some((epi, trumpforfarbe));
                }
            }
            for ahand in forever_rand_hands(
                &game.stichseq,
                (
                    game.ahand[unwrap!(game.which_player_can_do_something()).0].clone(),
                    unwrap!(game.which_player_can_do_something()).0,
                ),
                &game.rules,
                &game.expensifiers.vecstoss,
            )
                .take(100)
            {
                println!("{}", display_card_slices(&ahand, &game.rules, "\n"));
                for &(epi, ref trumpforfarbe) in vectplepitrumpforfarbe_frei.iter() {
                    assert!(!ahand[epi].contains_pred(|card| *trumpforfarbe==game.rules.trumpforfarbe(*card)));
                }
                if let Some((epi_not_frei, ref trumpforfarbe))=oassertnotfrei {
                    assert!(ahand[epi_not_frei].contains_pred(|card| *trumpforfarbe==game.rules.trumpforfarbe(*card)));
                }
            }
        }
    };
    test_game(
        [[H9, E7, GA, GZ, G9, E9, EK, EA], [HU, HA, SO, S8, GO, E8, SK, EZ], [H8, SU, G7, S7, GU, EO, GK, S9], [EU, H7, G8, SA, HO, SZ, HK, HZ]],
        &SActivelyPlayableRules::from(SRulesRufspiel::new(EPlayerIndex::EPI3, EFarbe::Gras, SPayoutDeciderParams::new(/*n_payout_base*/ 20, /*n_payout_schneider_schwarz*/ 10, SLaufendeParams::new(10, 3)), SStossParams::new(/*n_stoss_max*/4))).into(),
        vec![
            VTestAction::AssertNotFrei(EPlayerIndex::EPI3, VTrumpfOrFarbe::Farbe(EFarbe::Gras)),
            VTestAction::PlayStich([H9, HU, H8, EU]),
            VTestAction::AssertNotFrei(EPlayerIndex::EPI3, VTrumpfOrFarbe::Farbe(EFarbe::Gras)),
            VTestAction::PlayStich([H7, E7, HA, SU]),
            VTestAction::AssertNotFrei(EPlayerIndex::EPI3, VTrumpfOrFarbe::Farbe(EFarbe::Gras)),
            VTestAction::AssertFrei(EPlayerIndex::EPI0, VTrumpfOrFarbe::Trumpf),
            VTestAction::PlayStich([G7, G8, GA, SO]),
            VTestAction::AssertFrei(EPlayerIndex::EPI1, VTrumpfOrFarbe::Farbe(EFarbe::Gras)),
            VTestAction::PlayStich([S8, S7, SA, GZ]),
            VTestAction::AssertFrei(EPlayerIndex::EPI0, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
            // Remaining stichs: "ho g9 go gu" "e8 eo sz e9" "gk hk ek sk" "hz ea ez s9"
        ]
    );
    test_game(
        [[S7, GZ, H7, HO, G7, SA, S8, S9], [E9, EK, GU, GO, GK, SU, SK, HU], [SO, EZ, EO, H9, HZ, H8, HA, EU], [SZ, GA, HK, G8, EA, E8, G9, E7]],
        &SActivelyPlayableRules::from(SRulesRufspiel::new(EPlayerIndex::EPI3, EFarbe::Schelln, SPayoutDeciderParams::new(/*n_payout_base*/ 20, /*n_payout_schneider_schwarz*/ 10, SLaufendeParams::new(10, 3)), SStossParams::new(/*n_stoss_max*/4))).into(),
        vec![
            VTestAction::AssertNotFrei(EPlayerIndex::EPI3, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
            VTestAction::PlayStich([S9, SK, HZ, SZ]),
            VTestAction::AssertFrei(EPlayerIndex::EPI3, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
            VTestAction::AssertFrei(EPlayerIndex::EPI1, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
            VTestAction::AssertFrei(EPlayerIndex::EPI2, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
        ]
    );
}

#[test]
fn test_very_expensive_exploration() { // this kind of abuses the test mechanism to benchmark the performance
    use crate::primitives::card::ECard::*;
    use crate::game::*;
    use crate::rules::{
        rulessolo::*,
        payoutdecider::*,
    };
    let epi_active = EPlayerIndex::EPI0;
    let n_payout_base = 50;
    let n_payout_schneider_schwarz = 10;
    let mut game = SGame::new(
        EPlayerIndex::map_from_raw([
            [EO,EU,HA,HZ,HK,H9,H8,H7],
            [GO,GU,E7,G7,S7,EA,EZ,EK],
            [HO,HU,E8,G8,S8,GA,GZ,GK],
            [SO,SU,E9,G9,S9,SA,SZ,SK],
        ]).map_into(|acard| acard.into()),
        SExpensifiersNoStoss::new(/*n_stock*/0),
        sololike(
            epi_active,
            EFarbe::Herz,
            ESoloLike::Solo,
            SPayoutDeciderPointBased::default_payoutdecider(n_payout_base, n_payout_schneider_schwarz, SLaufendeParams::new(10, 3)),
            SStossParams::new(
                /*n_stoss_max*/ 4,
            ),
        ).into(),
    );
    for acard_stich in [[EO, GO, HO, SO], [EU, GU, HU, SU], [HA, E7, E8, E9], [HZ, S7, S8, S9], [HK, G7, G8, G9]] {
        assert_eq!(EPlayerIndex::values().next(), Some(epi_active));
        for (epi, card) in EPlayerIndex::values().zip_eq(acard_stich) {
            unwrap!(game.zugeben(card, epi));
        }
    }
    for ahand in all_possible_hands(
        &game.stichseq,
        (game.ahand[epi_active].clone(), epi_active),
        &game.rules,
        &game.expensifiers.vecstoss,
    ) {
        assert!(!game.current_playable_stich().is_full());
        let stichseq = &game.stichseq;
        let determinebestcardresult = unwrap!(determine_best_card(
            stichseq,
            Box::new(std::iter::once(ahand)) as Box<_>,
            /*fn_make_filter*/SBranchingFactor::factory(1, 2),
            &|_stichseq, _ahand| SMinReachablePayout::new_from_game(&game),
            /*fn_snapshotcache*/SSnapshotCacheNone::factory(), // TODO test cache
            /*fn_visualizer*/SNoVisualization::factory(),
            /*fn_inspect*/&|_b_before, _i_ahand, _ahand, _card| {},
            /*fn_payout*/&|_stichseq, _ahand, n_payout| (n_payout, ()),
        ));
        for card in [H7, H8, H9] {
            assert_eq!(
                determinebestcardresult.mapcardt[card].clone().map(|minmax| minmax.minmin.0.min()),
                Some(3*(n_payout_base+2*n_payout_schneider_schwarz))
            );
            assert_eq!(
                determinebestcardresult.mapcardt[card].clone().map(|minmax| minmax.maxmin.0.min()),
                Some(3*(n_payout_base+2*n_payout_schneider_schwarz))
            );
            assert_eq!(
                determinebestcardresult.mapcardt[card].clone().map(|minmax| minmax.maxselfishmin.0.min()),
                Some(3*(n_payout_base+2*n_payout_schneider_schwarz))
            );
            assert_eq!(
                determinebestcardresult.mapcardt[card].clone().map(|minmax| minmax.maxselfishmax.0.min()),
                Some(3*(n_payout_base+2*n_payout_schneider_schwarz))
            );
            assert_eq!(
                determinebestcardresult.mapcardt[card].clone().map(|minmax| minmax.maxmax.0.min()),
                Some(3*(n_payout_base+2*n_payout_schneider_schwarz))
            );
        }
    }
}
