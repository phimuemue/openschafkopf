pub mod suspicion;
pub mod handiterators;
pub mod rulespecific;
#[cfg(test)]
pub mod test;

use crate::primitives::*;
use crate::rules::{
    *,
};
use crate::game::*;
use crate::ai::{
    suspicion::*,
    handiterators::*,
};
use rand::prelude::*;
use std::{
    self,
    sync::{
        Arc, Mutex,
    },
    cmp,
};
use rayon::prelude::*;
use crate::util::*;
use chrono::Local;

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
    rules: &'game dyn TRules,
    stichseq: &'game SStichSequence,
    pub epi_fixed: EPlayerIndex,
    pub veccard_allowed: SHandVector,
}
impl<'game> SDetermineBestCard<'game> {
    pub fn new(rules: &'game dyn TRules, stichseq: &'game SStichSequence, hand_fixed: &SHand, epi_fixed: EPlayerIndex) -> Self {
        let veccard_allowed = rules.all_allowed_cards(stichseq, hand_fixed);
        assert!(1<=veccard_allowed.len());
        Self{
            rules,
            stichseq,
            epi_fixed,
            veccard_allowed
        }
    }

    pub fn new_from_game(game: &'game SGame) -> Self {
        let epi_fixed = debug_verify!(game.which_player_can_do_something()).unwrap().0;
        Self::new(
            game.rules.as_ref(),
            &game.stichseq,
            /*hand_fixed*/&game.ahand[epi_fixed],
            epi_fixed,
        )
    }

    pub fn single_allowed_card(&self) -> Option<SCard> {
        self.veccard_allowed.iter().single().ok().copied()
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

    pub fn rank_rules(&self, hand_fixed: SFullHand, epi_first: EPlayerIndex, epi_rank: EPlayerIndex, rules: &dyn TRules, tpln_stoss_doubling: (usize, usize), n_stock: isize) -> f64 {
        // TODO: adjust interface to get whole game in case of VAIParams::Cheating
        let ekurzlang = EKurzLang::from_cards_per_player(hand_fixed.get().cards().len());
        forever_rand_hands(&SStichSequence::new(epi_first, ekurzlang), hand_fixed.get().clone(), epi_rank, rules)
            .take(self.n_rank_rules_samples)
            .collect::<Vec<_>>() // TODO necessary?
            .into_par_iter()
            .map(|mut ahand| {
                explore_snapshots(
                    &mut ahand,
                    rules,
                    &mut SStichSequence::new(epi_first, ekurzlang),
                    &branching_factor(|_stichseq| (1, 2)),
                    &SMinReachablePayoutLowerBoundViaHint(SMinReachablePayoutParams::new(
                        rules,
                        epi_rank,
                        tpln_stoss_doubling,
                        n_stock,
                    )),
                    /*opath_out_dir*/None,
                )
            })
            .sum::<isize>().as_num::<f64>() / (self.n_rank_rules_samples.as_num::<f64>())
    }

    pub fn suggest_card_internal(
        rules: &dyn TRules,
        stichseq: &SStichSequence,
        epi_fixed: EPlayerIndex,
        hand_fixed: &SHand, // TODO is this needed if we have itahand?
        itahand: impl Iterator<Item=EnumMap<EPlayerIndex, SHand>>,
        n_suggest_card_branches: usize,
        tpln_stoss_doubling: (usize, usize),
        n_stock: isize,
        opath_out_dir: Option<&std::path::Path>,
    ) -> SCard {
        assert!(!hand_fixed.cards().is_empty()); // TODO? introduce verify_that or similar
        let determinebestcard = SDetermineBestCard::new(
            rules,
            stichseq,
            hand_fixed,
            epi_fixed,
        );
        determinebestcard
            .single_allowed_card()
            .unwrap_or_else(|| {
                macro_rules! forward_to_determine_best_card{($func_filter_allowed_cards: expr, $foreachsnapshot: expr,) => { // TODORUST generic closures
                    {
                        determine_best_card(
                            &determinebestcard,
                            itahand,
                            $func_filter_allowed_cards,
                            $foreachsnapshot,
                            opath_out_dir.map(|path_out_dir| {
                                debug_verify!(std::fs::create_dir_all(path_out_dir)).unwrap();
                                crate::game_analysis::generate_html_auxiliary_files(path_out_dir).unwrap();
                                path_out_dir
                                    .join(format!("{}", Local::now().format("%Y%m%d%H%M%S")))
                            }),
                        ).best_card().0
                    }
                }}
                // TODORUST exhaustive_integer_patterns for isize/usize
                // https://github.com/rust-lang/rfcs/pull/2591/commits/46135303146c660f3c5d34484e0ede6295c8f4e7#diff-8fe9cb03c196455367c9e539ea1964e8R70
                let make_minreachablepayoutparams = || {
                    SMinReachablePayoutParams::new(
                        rules,
                        epi_fixed,
                        tpln_stoss_doubling,
                        n_stock,
                    )
                };
                match /*n_remaining_cards_on_hand*/remaining_cards_per_hand(stichseq)[determinebestcard.epi_fixed] {
                    1|2|3 => forward_to_determine_best_card!(
                        &|_,_| (/*no filtering*/),
                        &SMinReachablePayout(make_minreachablepayoutparams()),
                    ),
                    4 => forward_to_determine_best_card!(
                        &|_,_| (/*no filtering*/),
                        &SMinReachablePayoutLowerBoundViaHint(make_minreachablepayoutparams()),
                    ),
                    5|6|7|8 => forward_to_determine_best_card!(
                        &branching_factor(|_stichseq| {
                            (1, n_suggest_card_branches+1)
                        }),
                        &SMinReachablePayoutLowerBoundViaHint(make_minreachablepayoutparams()),
                    ),
                    n_remaining_cards_on_hand => panic!("internal_suggest_card called with {} cards on hand", n_remaining_cards_on_hand),
                }
        })
    }

    pub fn suggest_card_simulating(
        rules: &dyn TRules,
        stichseq: &SStichSequence,
        epi_fixed: EPlayerIndex,
        hand_fixed: &SHand,
        n_suggest_card_samples: usize,
        n_suggest_card_branches: usize,
        tpln_stoss_doubling: (usize, usize),
        n_stock: isize,
        opath_out_dir: Option<&std::path::Path>,
    ) -> SCard {
        macro_rules! forward_to_determine_best_card{($itahand: expr) => { // TODORUST generic closures
            Self::suggest_card_internal(
                rules,
                stichseq,
                epi_fixed,
                hand_fixed,
                $itahand,
                n_suggest_card_branches,
                tpln_stoss_doubling,
                n_stock,
                opath_out_dir,
            )
        }}
        match /*n_remaining_cards_on_hand*/remaining_cards_per_hand(stichseq)[epi_fixed] {
            1|2|3|4 => forward_to_determine_best_card!(
                all_possible_hands(stichseq, hand_fixed.clone(), epi_fixed, rules)
            ),
            5|6|7|8 => forward_to_determine_best_card!(
                forever_rand_hands(stichseq, hand_fixed.clone(), epi_fixed, rules)
                    .take(n_suggest_card_samples)
            ),
            n_remaining_cards_on_hand => panic!("internal_suggest_card called with {} cards on hand", n_remaining_cards_on_hand),
        }
    }

    pub fn suggest_card(&self, game: &SGame, opath_out_dir: Option<&std::path::Path>) -> SCard {
        let rules = game.rules.as_ref();
        if let Some(card) = rules.rulespecific_ai()
            .and_then(|airulespecific| airulespecific.suggest_card(game))
        {
            card
        } else {
            macro_rules! suggest_via{($fn_suggest: ident, $arg: expr) => {{ // TODORUST generic closures
                let epi_fixed = debug_verify_eq!(
                    debug_verify!(game.which_player_can_do_something()).unwrap().0,
                    debug_verify!(game.current_playable_stich().current_playerindex()).unwrap()
                );
                Self::$fn_suggest(
                    rules,
                    &game.stichseq,
                    epi_fixed,
                    /*hand_fixed*/&game.ahand[epi_fixed],
                    $arg,
                    self.n_suggest_card_branches,
                    /*tpln_stoss_doubling*/stoss_and_doublings(&game.vecstoss, &game.doublings),
                    game.n_stock,
                    opath_out_dir,
                )
            }}}
            match self.aiparams {
                VAIParams::Cheating => {
                    suggest_via!(suggest_card_internal, /*itahand*/Some(game.ahand.clone()).into_iter())
                },
                VAIParams::Simulating{n_suggest_card_samples} => {
                    suggest_via!(suggest_card_simulating, n_suggest_card_samples)
                },
            }
        }
    }
}

pub fn unplayed_cards<'lifetime>(stichseq: &'lifetime SStichSequence, hand_fixed: &'lifetime SHand) -> impl Iterator<Item=SCard> + 'lifetime {
    SCard::values(stichseq.kurzlang())
        .filter(move |card| 
             !hand_fixed.contains(*card)
             && !stichseq.visible_stichs().any(|stich|
                stich.iter().any(|(_epi, card_in_stich)|
                    card_in_stich==card
                )
             )
        )
}

#[test]
fn test_unplayed_cards() {
    use crate::card::card_values::*;
    let epi_irrelevant = EPlayerIndex::EPI0;
    let mut stichseq = SStichSequence::new(epi_irrelevant, EKurzLang::Lang);
    for acard_stich in [[G7, G8, GA, G9], [S8, HO, S7, S9], [H7, HK, HU, SU], [EO, GO, HZ, H8], [E9, EK, E8, EA], [SA, EU, SO, HA]].iter() {
        for card in acard_stich.iter() {
            stichseq.zugeben_custom_winner_index(*card, |_stich| epi_irrelevant);
        }
    }
    let hand = &SHand::new_from_vec([GK, SK].iter().copied().collect());
    let veccard_unplayed = unplayed_cards(&stichseq, &hand).collect::<Vec<_>>();
    let veccard_unplayed_check = [GZ, E7, SZ, H9, EZ, GU];
    assert_eq!(veccard_unplayed.len(), veccard_unplayed_check.len());
    assert!(veccard_unplayed.iter().all(|card| veccard_unplayed_check.contains(card)));
    assert!(veccard_unplayed_check.iter().all(|card| veccard_unplayed.contains(card)));
}

pub struct SDetermineBestCardResult<'determinebestcard, T> {
    determinebestcard: &'determinebestcard SDetermineBestCard<'determinebestcard>,
    mapcardt: EnumMap<SCard, T>,
}

impl<'determinebestcard, T> SDetermineBestCardResult<'determinebestcard, T> {
    pub fn best_card(&self) -> (SCard, &T) where T: Ord+std::fmt::Debug {
        let card = debug_verify!(
            self.determinebestcard.veccard_allowed.iter()
                .max_by_key(|&card| &self.mapcardt[*card])
        ).unwrap();
        (*card, &self.mapcardt[*card])
    }
}

pub fn determine_best_card<'determinebestcard>(
    determinebestcard: &'determinebestcard SDetermineBestCard,
    itahand: impl Iterator<Item=EnumMap<EPlayerIndex, SHand>>,
    func_filter_allowed_cards: &(impl Fn(&SStichSequence, &mut SHandVector) + std::marker::Sync),
    foreachsnapshot: &(impl TForEachSnapshot<Output=isize> + Sync),
    opath_out_dir: Option<std::path::PathBuf>
) -> SDetermineBestCardResult<'determinebestcard, isize> {
    let mapcardn_payout = Arc::new(Mutex::new(
        // aggregate n_payout per card in some way
        SCard::map_from_fn(|_card| std::isize::MAX),
    ));
    itahand.enumerate()
        .collect::<Vec<_>>() // TODO necessary?
        .into_par_iter()
        .flat_map(|(i_susp, ahand)|
            determinebestcard.veccard_allowed.par_iter()
                .map(move |card| (i_susp, ahand.clone(), *card))
        )
        .for_each(|(i_susp, ahand, card)| {
            debug_assert!(ahand[determinebestcard.epi_fixed].cards().contains(&card));
            let mut ahand = ahand.clone();
            let mapcardn_payout = Arc::clone(&mapcardn_payout);
            let mut stichseq = determinebestcard.stichseq.clone();
            assert!(ahand_vecstich_card_count_is_compatible(&stichseq, &ahand));
            ahand[determinebestcard.epi_fixed].play_card(card);
            stichseq.zugeben(card, determinebestcard.rules);
            let n_payout = explore_snapshots(
                &mut ahand,
                determinebestcard.rules,
                &mut stichseq,
                func_filter_allowed_cards,
                foreachsnapshot,
                opath_out_dir.as_ref().map(|path_out_dir| {
                    debug_verify!(std::fs::create_dir_all(path_out_dir)).unwrap();
                    debug_verify!(std::fs::File::create(
                        path_out_dir
                            .join(format!("{}_{}.html", i_susp, card))
                    )).unwrap()
                }).map(|file_output| (file_output, determinebestcard.epi_fixed)),
            );
            let mut mapcardn_payout = debug_verify!(mapcardn_payout.lock()).unwrap();
            mapcardn_payout[card] = cmp::min(mapcardn_payout[card], n_payout);
        });
    let mapcardn_payout = debug_verify!(
        debug_verify!(Arc::try_unwrap(mapcardn_payout)).unwrap() // "Returns the contained value, if the Arc has exactly one strong reference"   
            .into_inner() // "If another user of this mutex panicked while holding the mutex, then this call will return an error instead"
    ).unwrap();
    assert!(<SCard as TPlainEnum>::values().any(|card| {
        determinebestcard.veccard_allowed.contains(&card) && mapcardn_payout[card] < std::isize::MAX
    }));
    SDetermineBestCardResult{
        determinebestcard,
        mapcardt: mapcardn_payout,
    }
}

pub fn branching_factor(fn_stichseq_to_intvl: impl Fn(&SStichSequence)->(usize, usize)) -> impl Fn(&SStichSequence, &mut SHandVector) {
    move |stichseq, veccard_allowed| {
        assert!(!veccard_allowed.is_empty());
        let (n_lo, n_hi) = fn_stichseq_to_intvl(stichseq);
        assert!(n_lo < n_hi);
        let mut rng = rand::thread_rng();
        let n = rng.gen_range(n_lo, n_hi);
        while n<veccard_allowed.len() {
            veccard_allowed.swap_remove(rng.gen_range(0, veccard_allowed.len()));
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
    let test_game = |aacard_hand: [[SCard; 8]; 4], rules: &dyn TRules, epi_first, vectestaction: Vec<VTestAction>| {
        let ahand = EPlayerIndex::map_from_raw(aacard_hand)
            .map(|acard_hand|
                SHand::new_from_vec(acard_hand.iter().copied().collect())
            );
        use crate::rules::ruleset::*;
        let mut game = game::SGame::new(
            ahand,
            SDoublings::new(epi_first),
            Some(SStossParams::new( // TODO implement tests for SStoss
                /*n_stoss_max*/ 4,
            )),
            rules.box_clone(),
            /*n_stock*/ 0,
        );
        let mut vecpairepitrumpforfarbe_frei = Vec::new();
        for testaction in vectestaction {
            let mut oassertnotfrei = None;
            match testaction {
                VTestAction::PlayStich(acard) => {
                    for card in acard.iter() {
                        let epi = debug_verify!(game.which_player_can_do_something()).unwrap().0;
                        debug_verify!(game.zugeben(*card, epi)).unwrap();
                    }
                },
                VTestAction::AssertFrei(epi, trumpforfarbe) => {
                    vecpairepitrumpforfarbe_frei.push((epi, trumpforfarbe));
                },
                VTestAction::AssertNotFrei(epi, trumpforfarbe) => {
                    oassertnotfrei = Some((epi, trumpforfarbe));
                }
            }
            for ahand in forever_rand_hands(
                &game.stichseq,
                game.ahand[debug_verify!(game.which_player_can_do_something()).unwrap().0].clone(),
                debug_verify!(game.which_player_can_do_something()).unwrap().0,
                game.rules.as_ref(),
            )
                .take(100)
            {
                for epi in EPlayerIndex::values() {
                    println!("{}: {}", epi, ahand[epi]);
                }
                for &(epi, ref trumpforfarbe) in vecpairepitrumpforfarbe_frei.iter() {
                    assert!(!ahand[epi].contains_pred(|card| *trumpforfarbe==game.rules.trumpforfarbe(*card)));
                }
                if let Some((epi_not_frei, ref trumpforfarbe))=oassertnotfrei {
                    assert!(ahand[epi_not_frei].contains_pred(|card| *trumpforfarbe==game.rules.trumpforfarbe(*card)));
                }
            }
        }
    };
    test_game(
        [[H8, SU, G7, S7, GU, EO, GK, S9], [EU, H7, G8, SA, HO, SZ, HK, HZ], [H9, E7, GA, GZ, G9, E9, EK, EA], [HU, HA, SO, S8, GO, E8, SK, EZ]],
        &SRulesRufspiel::new(EPlayerIndex::EPI1, EFarbe::Gras, SPayoutDeciderParams::new(/*n_payout_base*/ 20, /*n_payout_schneider_schwarz*/ 10, SLaufendeParams::new(10, 3))),
        /*epi_first*/ EPlayerIndex::EPI2,
        vec![
            VTestAction::AssertNotFrei(EPlayerIndex::EPI1, VTrumpfOrFarbe::Farbe(EFarbe::Gras)),
            VTestAction::PlayStich([H9, HU, H8, EU]),
            VTestAction::AssertNotFrei(EPlayerIndex::EPI1, VTrumpfOrFarbe::Farbe(EFarbe::Gras)),
            VTestAction::PlayStich([H7, E7, HA, SU]),
            VTestAction::AssertNotFrei(EPlayerIndex::EPI1, VTrumpfOrFarbe::Farbe(EFarbe::Gras)),
            VTestAction::AssertFrei(EPlayerIndex::EPI2, VTrumpfOrFarbe::Trumpf),
            VTestAction::PlayStich([G7, G8, GA, SO]),
            VTestAction::AssertFrei(EPlayerIndex::EPI3, VTrumpfOrFarbe::Farbe(EFarbe::Gras)),
            VTestAction::PlayStich([S8, S7, SA, GZ]),
            VTestAction::AssertFrei(EPlayerIndex::EPI2, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
            // Remaining stichs: "ho g9 go gu" "e8 eo sz e9" "gk hk ek sk" "hz ea ez s9"
        ]
    );
    test_game(
        [[SZ, GA, HK, G8, EA, E8, G9, E7], [S7, GZ, H7, HO, G7, SA, S8, S9], [E9, EK, GU, GO, GK, SU, SK, HU], [SO, EZ, EO, H9, HZ, H8, HA, EU]],
        &SRulesRufspiel::new(EPlayerIndex::EPI0, EFarbe::Schelln, SPayoutDeciderParams::new(/*n_payout_base*/ 20, /*n_payout_schneider_schwarz*/ 10, SLaufendeParams::new(10, 3))),
        /*epi_first*/ EPlayerIndex::EPI1,
        vec![
            VTestAction::AssertNotFrei(EPlayerIndex::EPI0, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
            VTestAction::PlayStich([S9, SK, HZ, SZ]),
            VTestAction::AssertFrei(EPlayerIndex::EPI0, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
            VTestAction::AssertFrei(EPlayerIndex::EPI2, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
            VTestAction::AssertFrei(EPlayerIndex::EPI3, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
        ]
    );
}

#[test]
fn test_very_expensive_exploration() { // this kind of abuses the test mechanism to benchmark the performance
    use crate::card::card_values::*;
    use crate::game::*;
    use crate::rules::{ruleset::*, rulessolo::*, payoutdecider::*};
    use crate::game_analysis::TPayoutDeciderSoloLikeDefault;
    let epi_first_and_active_player = EPlayerIndex::EPI0;
    let n_payout_base = 50;
    let n_payout_schneider_schwarz = 10;
    let mut game = SGame::new(
        EPlayerIndex::map_from_raw([
            [EO,EU,HA,HZ,HK,H9,H8,H7],
            [GO,GU,E7,G7,S7,EA,EZ,EK],
            [HO,HU,E8,G8,S8,GA,GZ,GK],
            [SO,SU,E9,G9,S9,SA,SZ,SK],
        ]).map(|acard_hand|
            SHand::new_from_vec(acard_hand.iter().copied().collect())
        ),
        SDoublings::new(epi_first_and_active_player),
        Some(SStossParams::new(
            /*n_stoss_max*/ 4,
        )),
        TRules::box_clone(sololike(
            epi_first_and_active_player,
            EFarbe::Herz,
            ESoloLike::Solo,
            SPayoutDeciderPointBased::default_payoutdecider(n_payout_base, n_payout_schneider_schwarz, SLaufendeParams::new(10, 3)),
        ).as_ref()),
        /*n_stock*/ 0,
    );
    for acard_stich in [[EO, GO, HO, SO], [EU, GU, HU, SU], [HA, E7, E8, E9], [HZ, S7, S8, S9], [HK, G7, G8, G9]].iter() {
        assert_eq!(EPlayerIndex::values().nth(0), Some(epi_first_and_active_player));
        for (epi, card) in EPlayerIndex::values().zip(acard_stich.iter()) {
            debug_verify!(game.zugeben(*card, epi)).unwrap();
        }
    }
    for ahand in all_possible_hands(
        &game.stichseq,
        game.ahand[epi_first_and_active_player].clone(),
        epi_first_and_active_player,
        game.rules.as_ref(),
    ) {
        let stich_current = game.current_playable_stich();
        assert!(!stich_current.is_full());
        let determinebestcard = SDetermineBestCard::new_from_game(&game);
        let determinebestcardresult = determine_best_card(
            &determinebestcard,
            Some(ahand).into_iter(),
            /*func_filter_allowed_cards*/&branching_factor(|_stichseq| (1, 2)),
            &SMinReachablePayout(SMinReachablePayoutParams::new_from_game(&game)),
            /*opath_out_dir*/None, //Some(&format!("suspicion_test/{:?}", ahand)), // to inspect search tree
        );
        for card in [H7, H8, H9].iter() {
            assert!(determinebestcard.veccard_allowed.contains(card));
            assert!(
                determinebestcardresult.mapcardt[*card] == std::isize::MAX
                || determinebestcardresult.mapcardt[*card] == 3*(n_payout_base+2*n_payout_schneider_schwarz)
            );
        }
    }
}
