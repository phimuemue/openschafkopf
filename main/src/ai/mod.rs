pub mod handiterators;
pub mod rulespecific;
pub mod suspicion;
#[cfg(test)]
pub mod test;

use crate::ai::{handiterators::*, suspicion::*};
use crate::game::*;
use crate::primitives::*;
use crate::rules::*;
use crate::util::*;
use itertools::Itertools;
use rand::prelude::*;
use rayon::prelude::*;
use std::{
    self,
    sync::{Arc, Mutex},
};

plain_enum_mod!(moderemainingcards, ERemainingCards {_1, _2, _3, _4, _5, _6, _7, _8,});

pub fn remaining_cards_per_hand(stichseq: &SStichSequence) -> EnumMap<EPlayerIndex, usize> {
    EPlayerIndex::map_from_fn(|epi| {
        stichseq.kurzlang().cards_per_player()
            - stichseq.completed_stichs().len()
            - match stichseq.current_stich().get(epi) {
                None => 0,
                Some(_card) => 1,
            }
    })
}

pub fn ahand_vecstich_card_count_is_compatible(stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>) -> bool {
    ahand.map(|hand| hand.cards().len()) == remaining_cards_per_hand(stichseq)
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

pub struct SDetermineBestCard<'game> {
    pub rules: &'game dyn TRules,
    pub stichseq: &'game SStichSequence,
    pub epi_fixed: EPlayerIndex,
    pub hand_fixed: &'game SHand,
    pub veccard_allowed: SHandVector,
}
impl<'game> SDetermineBestCard<'game> {
    pub fn new(rules: &'game dyn TRules, stichseq: &'game SStichSequence, hand_fixed: &'game SHand) -> Self {
        let veccard_allowed = rules.all_allowed_cards(stichseq, hand_fixed);
        assert!(!veccard_allowed.is_empty());
        Self{
            rules,
            stichseq,
            epi_fixed: unwrap!(stichseq.current_stich().current_playerindex()),
            hand_fixed,
            veccard_allowed
        }
    }

    pub fn new_from_game(game: &'game SGame) -> Self {
        Self::new(
            game.rules.as_ref(),
            &game.stichseq,
            /*hand_fixed*/&game.ahand[unwrap!(game.which_player_can_do_something()).0],
        )
    }

    pub fn single_allowed_card(&self) -> Option<SCard> {
        self.veccard_allowed.iter().exactly_one().ok().copied()
    }
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

    pub fn rank_rules(&self, hand_fixed: SFullHand, epi_rank: EPlayerIndex, rules: &dyn TRules, tpln_stoss_doubling: (usize, usize), n_stock: isize) -> f64 {
        // TODO: adjust interface to get whole game in case of VAIParams::Cheating
        let ekurzlang = EKurzLang::from_cards_per_player(hand_fixed.get().len());
        forever_rand_hands(&SStichSequence::new(ekurzlang), SHand::new_from_iter(hand_fixed.get().iter().copied()), epi_rank, rules)
            .take(self.n_rank_rules_samples)
            .par_bridge() // TODO can we derive a true parallel iterator?
            .map(|mut ahand| {
                explore_snapshots(
                    &mut ahand,
                    rules,
                    &mut SStichSequence::new(ekurzlang),
                    &branching_factor(|_stichseq| (1, 2)),
                    &SMinReachablePayoutLowerBoundViaHint::new(
                        rules,
                        epi_rank,
                        tpln_stoss_doubling,
                        n_stock,
                    ),
                    &mut SNoVisualization{},
                ).t_min[epi_rank]
            })
            .sum::<isize>().as_num::<f64>() / (self.n_rank_rules_samples.as_num::<f64>())
    }

    pub fn suggest_card<SnapshotVisualizer: TSnapshotVisualizer<SMinMax>>(
        &self,
        game: &SGame,
        fn_visualizer: impl Fn(usize, &EnumMap<EPlayerIndex, SHand>, SCard) -> SnapshotVisualizer + std::marker::Sync,
    ) -> SCard {
        let determinebestcard = SDetermineBestCard::new_from_game(game);
        let epi_fixed = determinebestcard.epi_fixed;
        if let Some(card)=determinebestcard.single_allowed_card() {
            card
        } else if let Some(card) = game.rules.rulespecific_ai()
            .and_then(|airulespecific| airulespecific.suggest_card(game))
        {
            card
        } else {
            macro_rules! forward_to_determine_best_card{(
                ($func_filter_allowed_cards: expr, $foreachsnapshot: ident,),
                $itahand: expr,
            ) => {{ // TODORUST generic closures
                determine_best_card(
                    &determinebestcard,
                    $itahand,
                    $func_filter_allowed_cards,
                    &$foreachsnapshot::new(
                        determinebestcard.rules,
                        epi_fixed,
                        /*tpln_stoss_doubling*/stoss_and_doublings(&game.vecstoss, &game.doublings),
                        game.n_stock,
                    ),
                    fn_visualizer,
                )
            }}}
            let eremainingcards = unwrap!(ERemainingCards::checked_from_usize(
                remaining_cards_per_hand(determinebestcard.stichseq)[epi_fixed] - 1 // ERemainingCards starts with 1
            ));
            use ERemainingCards::*;
            *unwrap!(cartesian_match!(
                forward_to_determine_best_card,
                match (eremainingcards) {
                    _1|_2|_3 => (
                        &|_: &SStichSequence,_: &mut SHandVector| (/*no filtering*/),
                        SMinReachablePayout,
                    ),
                    _4 => (
                        &|_: &SStichSequence,_: &mut SHandVector| (/*no filtering*/),
                        SMinReachablePayoutLowerBoundViaHint,
                    ),
                    _5|_6|_7|_8 => (
                        &branching_factor(|_stichseq| {
                            (1, self.n_suggest_card_branches+1)
                        }),
                        SMinReachablePayoutLowerBoundViaHint,
                    ),
                },
                match ((&self.aiparams, eremainingcards)) {
                    (&VAIParams::Cheating, _) => {
                        std::iter::once(game.ahand.clone())
                    },
                    (&VAIParams::Simulating{n_suggest_card_samples:_}, _1|_2|_3|_4) => {
                        all_possible_hands(determinebestcard.stichseq, determinebestcard.hand_fixed.clone(), epi_fixed, determinebestcard.rules)
                    },
                    (&VAIParams::Simulating{n_suggest_card_samples}, _5|_6|_7|_8) =>{ 
                        forever_rand_hands(determinebestcard.stichseq, determinebestcard.hand_fixed.clone(), epi_fixed, determinebestcard.rules)
                            .take(n_suggest_card_samples)
                    },
                },
            ).cards_with_maximum_value().0.first())
        }
    }
}

pub fn unplayed_cards<'lifetime>(stichseq: &'lifetime SStichSequence, hand_fixed: &'lifetime SHand) -> impl Iterator<Item=SCard> + 'lifetime {
    SCard::values(stichseq.kurzlang())
        .filter(move |card| 
             !hand_fixed.contains(*card)
             && !stichseq.visible_cards().any(|(_epi, card_in_stich)|
                card_in_stich==card
            )
        )
}

#[test]
fn test_unplayed_cards() {
    use crate::card::card_values::*;
    let mut stichseq = SStichSequence::new(EKurzLang::Lang);
    for acard_stich in [[G7, G8, GA, G9], [S8, HO, S7, S9], [H7, HK, HU, SU], [EO, GO, HZ, H8], [E9, EK, E8, EA], [SA, EU, SO, HA]] {
        for card in acard_stich {
            stichseq.zugeben_custom_winner_index(card, |_stich| EPlayerIndex::EPI0 /*irrelevant*/);
        }
    }
    let hand = &SHand::new_from_iter([GK, SK]);
    let veccard_unplayed = unplayed_cards(&stichseq, hand).collect::<Vec<_>>();
    let veccard_unplayed_check = [GZ, E7, SZ, H9, EZ, GU];
    assert_eq!(veccard_unplayed.len(), veccard_unplayed_check.len());
    assert!(veccard_unplayed.iter().all(|card| veccard_unplayed_check.contains(card)));
    assert!(veccard_unplayed_check.iter().all(|card| veccard_unplayed.contains(card)));
}

pub struct SDetermineBestCardResult<T> {
    veccard_allowed: SHandVector,
    mapcardt: EnumMap<SCard, Option<T>>,
}

impl<T> SDetermineBestCardResult<T> {
    pub fn cards_and_ts(&self) -> impl Iterator<Item=(SCard, &T)> where T: std::fmt::Debug {
        self.veccard_allowed.iter()
            .map(move |card| (*card, unwrap!(self.mapcardt[*card].as_ref())))
    }
    pub fn cards_with_maximum_value(&self) -> (Vec<SCard>, &T) where T: std::fmt::Debug + Ord {
        let veccard = self.veccard_allowed.iter().copied()
            .max_set_by_key(|card| unwrap!(self.mapcardt[*card].as_ref()));
        assert!(!veccard.is_empty());
        let t = unwrap!(self.mapcardt[veccard[0]].as_ref());
        (veccard, t)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SPayoutStats {
    n_min: isize,
    n_sum: isize,
    n_max: isize,
    maporderingn_histogram_cmp_vs_0: EnumMap<std::cmp::Ordering, usize>,
}

impl SPayoutStats {
    fn new_1(n_payout: isize) -> Self {
        Self {
            n_min: n_payout,
            n_max: n_payout,
            n_sum: n_payout,
            maporderingn_histogram_cmp_vs_0: std::cmp::Ordering::map_from_fn(|ordering|
                if ordering==n_payout.cmp(&0) {1} else {0}
            ),
        }
    }

    fn accumulate(&mut self, paystats: &Self) {
        assign_min(&mut self.n_min, paystats.n_min);
        assign_max(&mut self.n_max, paystats.n_max);
        self.n_sum += paystats.n_sum;
        for ordering in std::cmp::Ordering::values() {
            self.maporderingn_histogram_cmp_vs_0[ordering] += paystats.maporderingn_histogram_cmp_vs_0[ordering];
        }
    }

    pub fn min(&self) -> isize {
        self.n_min
    }
    pub fn max(&self) -> isize {
        self.n_max
    }
    pub fn avg(&self) -> f32 {
        self.n_sum.as_num::<f32>() / self.maporderingn_histogram_cmp_vs_0.iter().sum::<usize>().as_num::<f32>()
    }
    pub fn counts(&self) -> &EnumMap<std::cmp::Ordering, usize> {
        &self.maporderingn_histogram_cmp_vs_0
    }
}

type SPayoutStatsPerStrategy = SPerMinMaxStrategy<SPayoutStats>;

impl SPayoutStatsPerStrategy {
    fn accumulate(&mut self, paystats: &Self) {
        self.t_min.accumulate(&paystats.t_min);
        self.t_selfish_min.accumulate(&paystats.t_selfish_min);
        self.t_selfish_max.accumulate(&paystats.t_selfish_max);
        self.t_max.accumulate(&paystats.t_max);
    }
}

impl std::cmp::PartialOrd for SPayoutStatsPerStrategy {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl std::cmp::Ord for SPayoutStatsPerStrategy {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;
        let n_min_self = self.t_min.n_min;
        let n_min_other = other.t_min.n_min;
        // TODO improve logic wrt t_selfish_min/t_selfish_max/t_max
        match (n_min_self.cmp(&0), n_min_other.cmp(&0)) {
            (Greater, Greater) => match n_min_self.cmp(&n_min_other) {
                Equal => unwrap!(self.t_selfish_min.avg().partial_cmp(&other.t_selfish_min.avg())),
                Greater => Greater,
                Less => Less,
            },
            (Greater, _) => Greater,
            (_, Greater) => Less,
            (Equal, Less) => Greater,
            (Less, Equal) => Less,
            (Less, Less)|(Equal, Equal) => {
                unwrap!(self.t_selfish_min.avg().partial_cmp(&other.t_selfish_min.avg()))
            },
        }
    }
}

pub fn determine_best_card<
    ForEachSnapshot: TForEachSnapshot<Output=SMinMax> + Sync,
    SnapshotVisualizer: TSnapshotVisualizer<ForEachSnapshot::Output>,
>(
    determinebestcard: &SDetermineBestCard,
    itahand: impl Iterator<Item=EnumMap<EPlayerIndex, SHand>> + Send,
    func_filter_allowed_cards: &(impl TFilterAllowedCards + std::marker::Sync),
    foreachsnapshot: &ForEachSnapshot,
    fn_visualizer: impl Fn(usize, &EnumMap<EPlayerIndex, SHand>, SCard) -> SnapshotVisualizer + std::marker::Sync,
) -> SDetermineBestCardResult<SPayoutStatsPerStrategy>
    where
        ForEachSnapshot::Output: std::fmt::Debug + Send,
{
    let mapcardooutput = Arc::new(Mutex::new(
        // aggregate n_payout per card in some way
        SCard::map_from_fn(|_card| None),
    ));
    itahand
        .enumerate()
        .par_bridge() // TODO can we derive a true parallel iterator?
        .flat_map(|(i_ahand, ahand)|
            determinebestcard.veccard_allowed.par_iter()
                .map(move |card| (i_ahand, ahand.clone(), *card))
        )
        .for_each(|(i_ahand, mut ahand, card)| {
            let mut visualizer = fn_visualizer(i_ahand, &ahand, card); // do before ahand is modified
            debug_assert!(ahand[determinebestcard.epi_fixed].cards().contains(&card));
            let mapcardooutput = Arc::clone(&mapcardooutput);
            let mut stichseq = determinebestcard.stichseq.clone();
            assert!(ahand_vecstich_card_count_is_compatible(&stichseq, &ahand));
            ahand[determinebestcard.epi_fixed].play_card(card);
            stichseq.zugeben(card, determinebestcard.rules);
            let output = explore_snapshots(
                &mut ahand,
                determinebestcard.rules,
                &mut stichseq,
                func_filter_allowed_cards,
                foreachsnapshot,
                &mut visualizer,
            );
            let ooutput = &mut unwrap!(mapcardooutput.lock())[card];
            let payoutstats = SPayoutStatsPerStrategy{
                t_min: SPayoutStats::new_1(output.t_min[determinebestcard.epi_fixed]),
                t_selfish_min: SPayoutStats::new_1(output.t_selfish_min[determinebestcard.epi_fixed]),
                t_selfish_max: SPayoutStats::new_1(output.t_selfish_max[determinebestcard.epi_fixed]),
                t_max: SPayoutStats::new_1(output.t_max[determinebestcard.epi_fixed]),
            };
            match ooutput {
                None => *ooutput = Some(payoutstats),
                Some(ref mut output_return) => output_return.accumulate(&payoutstats),
            }
        });
    let mapcardooutput = unwrap!(
        unwrap!(Arc::try_unwrap(mapcardooutput)) // "Returns the contained value, if the Arc has exactly one strong reference"   
            .into_inner() // "If another user of this mutex panicked while holding the mutex, then this call will return an error instead"
    );
    assert!(<SCard as TPlainEnum>::values().any(|card| {
        determinebestcard.veccard_allowed.contains(&card) && mapcardooutput[card].is_some()
    }));
    SDetermineBestCardResult{
        veccard_allowed: determinebestcard.veccard_allowed.clone(),
        mapcardt: mapcardooutput,
    }
}

pub fn branching_factor(fn_stichseq_to_intvl: impl Fn(&SStichSequence)->(usize, usize)) -> impl TFilterAllowedCards {
    move |stichseq: &SStichSequence, veccard_allowed: &mut SHandVector| {
        assert!(!veccard_allowed.is_empty());
        let (n_lo, n_hi) = fn_stichseq_to_intvl(stichseq);
        assert!(n_lo < n_hi);
        let mut rng = rand::thread_rng();
        let n = rng.gen_range(n_lo..n_hi);
        while n<veccard_allowed.len() {
            veccard_allowed.swap_remove(rng.gen_range(0..veccard_allowed.len()));
        }
    }
}

#[test]
fn test_is_compatible_with_game_so_far() {
    use crate::rules::rulesrufspiel::*;
    use crate::rules::payoutdecider::*;
    use crate::card::card_values::*;
    use crate::game;
    enum VTestAction {
        PlayStich([SCard; 4]),
        AssertFrei(EPlayerIndex, VTrumpfOrFarbe),
        AssertNotFrei(EPlayerIndex, VTrumpfOrFarbe),
    }
    let test_game = |aacard_hand: [[SCard; 8]; 4], rules: &dyn TRules, vectestaction: Vec<VTestAction>| {
        let ahand = EPlayerIndex::map_from_raw(aacard_hand)
            .map_into(|acard| acard.into());
        use crate::rules::ruleset::*;
        let mut game = game::SGame::new(
            ahand,
            SDoublings::new(SStaticEPI0{}),
            Some(SStossParams::new( // TODO implement tests for SStoss
                /*n_stoss_max*/ 4,
            )),
            rules.box_clone(),
            /*n_stock*/ 0,
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
                game.ahand[unwrap!(game.which_player_can_do_something()).0].clone(),
                unwrap!(game.which_player_can_do_something()).0,
                game.rules.as_ref(),
            )
                .take(100)
            {
                for epi in EPlayerIndex::values() {
                    println!("{}: {}", epi, ahand[epi]);
                }
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
        &SRulesRufspiel::new(EPlayerIndex::EPI3, EFarbe::Gras, SPayoutDeciderParams::new(/*n_payout_base*/ 20, /*n_payout_schneider_schwarz*/ 10, SLaufendeParams::new(10, 3))),
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
        &SRulesRufspiel::new(EPlayerIndex::EPI3, EFarbe::Schelln, SPayoutDeciderParams::new(/*n_payout_base*/ 20, /*n_payout_schneider_schwarz*/ 10, SLaufendeParams::new(10, 3))),
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
    use crate::card::card_values::*;
    use crate::game::*;
    use crate::rules::{ruleset::*, rulessolo::*, payoutdecider::*};
    use crate::game_analysis::TPayoutDeciderSoloLikeDefault;
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
        SDoublings::new(SStaticEPI0{}),
        Some(SStossParams::new(
            /*n_stoss_max*/ 4,
        )),
        TRulesBoxClone::box_clone(sololike(
            epi_active,
            EFarbe::Herz,
            ESoloLike::Solo,
            SPayoutDeciderPointBased::default_payoutdecider(n_payout_base, n_payout_schneider_schwarz, SLaufendeParams::new(10, 3)),
        ).as_ref()),
        /*n_stock*/ 0,
    );
    for acard_stich in [[EO, GO, HO, SO], [EU, GU, HU, SU], [HA, E7, E8, E9], [HZ, S7, S8, S9], [HK, G7, G8, G9]] {
        assert_eq!(EPlayerIndex::values().next(), Some(epi_active));
        for (epi, card) in EPlayerIndex::values().zip_eq(acard_stich) {
            unwrap!(game.zugeben(card, epi));
        }
    }
    for ahand in all_possible_hands(
        &game.stichseq,
        game.ahand[epi_active].clone(),
        epi_active,
        game.rules.as_ref(),
    ) {
        assert!(!game.current_playable_stich().is_full());
        let determinebestcard = SDetermineBestCard::new_from_game(&game);
        let determinebestcardresult = determine_best_card(
            &determinebestcard,
            std::iter::once(ahand),
            /*func_filter_allowed_cards*/&branching_factor(|_stichseq| (1, 2)),
            &SMinReachablePayout::new_from_game(&game),
            /*fn_visualizer*/|_,_,_| SNoVisualization,
        );
        for card in [H7, H8, H9] {
            assert!(determinebestcard.veccard_allowed.contains(&card));
            assert_eq!(
                determinebestcardresult.mapcardt[card].clone().map(|minmax| minmax.t_min.min()),
                Some(3*(n_payout_base+2*n_payout_schneider_schwarz))
            );
            assert_eq!(
                determinebestcardresult.mapcardt[card].clone().map(|minmax| minmax.t_selfish_min.min()),
                Some(3*(n_payout_base+2*n_payout_schneider_schwarz))
            );
            assert_eq!(
                determinebestcardresult.mapcardt[card].clone().map(|minmax| minmax.t_selfish_max.min()),
                Some(3*(n_payout_base+2*n_payout_schneider_schwarz))
            );
            assert_eq!(
                determinebestcardresult.mapcardt[card].clone().map(|minmax| minmax.t_max.min()),
                Some(3*(n_payout_base+2*n_payout_schneider_schwarz))
            );
        }
    }
}
