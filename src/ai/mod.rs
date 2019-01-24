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
    mem,
    sync::{
        Arc, Mutex,
    },
    cmp,
};
use rayon::prelude::*;
use crate::util::*;

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

    pub fn rank_rules(&self, hand_fixed: SFullHand, epi_first: EPlayerIndex, epi_rank: EPlayerIndex, rules: &TRules, n_stock: isize) -> f64 {
        // TODO: adjust interface to get whole game in case of VAIParams::Cheating
        let ekurzlang = EKurzLang::from_cards_per_player(hand_fixed.get().cards().len());
        forever_rand_hands(&SStichSequence::new(epi_first, ekurzlang), hand_fixed.get().clone(), epi_rank, rules)
            .take(self.n_rank_rules_samples)
            .collect::<Vec<_>>() // TODO necessary?
            .into_par_iter()
            .map(|mut ahand| {
                explore_snapshots(
                    epi_rank,
                    &mut ahand,
                    rules,
                    &mut SStichSequence::new(epi_first, ekurzlang),
                    &branching_factor(|_stichseq| (1, 2)),
                    &SMinReachablePayoutLowerBoundViaHint(SMinReachablePayoutParams::new(
                        rules,
                        epi_rank,
                        /*tpln_stoss_doubling*/(0, 0), // TODO do we need tpln_stoss_doubling from somewhere? 
                        n_stock,
                    )),
                    /*ostr_file_out*/None,
                )
            })
            .sum::<isize>().as_num::<f64>() / (self.n_rank_rules_samples.as_num::<f64>())
    }

    pub fn suggest_card(&self, game: &SGame, ostr_file_out: Option<&str>) -> SCard {
        let veccard_allowed = game.rules.all_allowed_cards(
            &game.stichseq,
            &game.ahand[verify!(game.which_player_can_do_something()).unwrap().0]
        );
        assert!(1<=veccard_allowed.len());
        if 1==veccard_allowed.len() {
            veccard_allowed[0]
        } else if let Some(card) = game.rules.rulespecific_ai()
            .and_then(|airulespecific| airulespecific.suggest_card(game))
        {
            card
        } else {
            macro_rules! forward_to_determine_best_card_itahand{($itahand: expr, $func_filter_allowed_cards: expr, $foreachsnapshot: expr,) => { // TODORUST generic closures
                determine_best_card(
                    game,
                    $itahand,
                    $func_filter_allowed_cards,
                    $foreachsnapshot,
                    ostr_file_out,
                )
            }}
            macro_rules! forward_to_determine_best_card{($itahand_simulating: expr, $func_filter_allowed_cards: expr, $foreachsnapshot: expr,) => { // TODORUST generic closures
                match self.aiparams {
                    VAIParams::Cheating => { forward_to_determine_best_card_itahand!(
                        Some(game.ahand.clone()).into_iter(),
                        $func_filter_allowed_cards,
                        $foreachsnapshot,
                    ) },
                    VAIParams::Simulating{n_suggest_card_samples} => { forward_to_determine_best_card_itahand!(
                        $itahand_simulating(n_suggest_card_samples),
                        $func_filter_allowed_cards,
                        $foreachsnapshot,
                    ) },
                }
            }}
            let stich_current = game.current_playable_stich();
            assert!(!stich_current.is_full());
            let epi_fixed = verify!(stich_current.current_playerindex()).unwrap();
            let hand_fixed = &game.ahand[epi_fixed];
            assert!(!hand_fixed.cards().is_empty());
            // TODORUST exhaustive_integer_patterns
            // https://github.com/rust-lang/rfcs/issues/1550
            // https://github.com/rust-lang/rust/issues/50907
            match /*n_incomplete_stichs*/ game.stichseq.kurzlang().cards_per_player()-game.stichseq.completed_stichs().len() {
                1|2 => forward_to_determine_best_card!(
                    |_n_suggest_card_samples| all_possible_hands(&game.stichseq, hand_fixed.clone(), epi_fixed, game.rules.as_ref()),
                    &|_,_| (/*no filtering*/),
                    &SMinReachablePayout(SMinReachablePayoutParams::new_from_game(game)),
                ),
                3 => forward_to_determine_best_card!(
                    |_n_suggest_card_samples| all_possible_hands(&game.stichseq, hand_fixed.clone(), epi_fixed, game.rules.as_ref()),
                    &|_,_| (/*no filtering*/),
                    &SMinReachablePayoutLowerBoundViaHint(SMinReachablePayoutParams::new_from_game(game)),
                ),
                4|5|6|7|8 => forward_to_determine_best_card!(
                    |n_suggest_card_samples| forever_rand_hands(&game.stichseq, hand_fixed.clone(), epi_fixed, game.rules.as_ref())
                        .take(n_suggest_card_samples),
                    &branching_factor(|_stichseq| {
                        (1, self.n_suggest_card_branches+1)
                    }),
                    &SMinReachablePayoutLowerBoundViaHint(SMinReachablePayoutParams::new_from_game(game)),
                ),
                n_incomplete_stichs => panic!("internal_suggest_card called with {} remaining stichs", n_incomplete_stichs),
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
    for acard_stich in [[G7, G8, GA, G9], [S8, HO, S7, S9], [H7, HK, HU, SU], [EO, GO, HZ, H8], [E9, EK, E8, EA], [SA, EU, SO, HA]].into_iter() {
        for card in acard_stich.into_iter() {
            stichseq.zugeben_custom_winner_index(*card, |_stich| epi_irrelevant);
        }
    }
    let hand = &SHand::new_from_vec([GK, SK].into_iter().cloned().collect());
    let veccard_unplayed = unplayed_cards(&stichseq, &hand).collect::<Vec<_>>();
    let veccard_unplayed_check = [GZ, E7, SZ, H9, EZ, GU];
    assert_eq!(veccard_unplayed.len(), veccard_unplayed_check.len());
    assert!(veccard_unplayed.iter().all(|card| veccard_unplayed_check.contains(card)));
    assert!(veccard_unplayed_check.iter().all(|card| veccard_unplayed.contains(card)));
}

fn determine_best_card_internal(
    game: &SGame,
    itahand: impl Iterator<Item=EnumMap<EPlayerIndex, SHand>>,
    func_filter_allowed_cards: &(impl Fn(&SStichSequence, &mut SHandVector) + std::marker::Sync),
    foreachsnapshot: &(impl TForEachSnapshot<Output=isize> + Sync),
    ostr_file_out: Option<&str>
) -> (SHandVector, EnumMap<SCard, isize>) {
    let epi_fixed = verify!(game.current_playable_stich().current_playerindex()).unwrap();
    let veccard_allowed = game.rules.all_allowed_cards(&game.stichseq, &game.ahand[epi_fixed]);
    let mapcardn_payout = Arc::new(Mutex::new(
        // aggregate n_payout per card in some way
        SCard::map_from_fn(|_card| std::isize::MAX),
    ));
    itahand.enumerate()
        .collect::<Vec<_>>() // TODO necessary?
        .into_par_iter()
        .flat_map(|(i_susp, ahand)|
            veccard_allowed.par_iter()
                .map(move |card| (i_susp, ahand.clone(), *card))
        )
        .for_each(|(i_susp, ahand, card)| {
            debug_assert!(ahand[epi_fixed].cards().contains(&card));
            let mut ahand = ahand.clone();
            let mapcardn_payout = Arc::clone(&mapcardn_payout);
            assert!(ahand_vecstich_card_count_is_compatible(&game.stichseq, &ahand));
            let mut stichseq = game.stichseq.clone();
            ahand[epi_fixed].play_card(card);
            stichseq.zugeben(card, game.rules.as_ref());
            let n_payout = explore_snapshots(
                epi_fixed,
                &mut ahand,
                game.rules.as_ref(),
                &mut stichseq,
                func_filter_allowed_cards,
                foreachsnapshot,
                ostr_file_out.map(|str_file_out| {
                    verify!(std::fs::create_dir_all(str_file_out)).unwrap();
                    format!("{}/{}_{}", str_file_out, i_susp, card)
                }).as_ref().map(|str_file_out| &str_file_out[..]),
            );
            let mut mapcardn_payout = verify!(mapcardn_payout.lock()).unwrap();
            mapcardn_payout[card] = cmp::min(mapcardn_payout[card], n_payout);
        });
    let mapcardn_payout = verify!(
        verify!(Arc::try_unwrap(mapcardn_payout)).unwrap() // "Returns the contained value, if the Arc has exactly one strong reference"   
            .into_inner() // "If another user of this mutex panicked while holding the mutex, then this call will return an error instead"
    ).unwrap();
    assert!(<SCard as TPlainEnum>::values().any(|card| {
        veccard_allowed.contains(&card) && mapcardn_payout[card] < std::isize::MAX
    }));
    (veccard_allowed, mapcardn_payout)
}

pub fn branching_factor(fn_stichseq_to_intvl: impl Fn(&SStichSequence)->(usize, usize)) -> impl Fn(&SStichSequence, &mut SHandVector) {
    move |stichseq, veccard_allowed| {
        assert!(!veccard_allowed.is_empty());
        let (n_lo, n_hi) = fn_stichseq_to_intvl(stichseq);
        assert!(n_lo < n_hi);
        let vect_sampled_tmp = veccard_allowed.iter().cloned().choose_multiple(&mut rand::thread_rng(), rand::thread_rng().gen_range(n_lo, n_hi)); // TODO can we choose_multiple directly into SHandVector?
        let mut vect_sampled = vect_sampled_tmp.into_iter().collect::<SHandVector>();
        mem::swap(&mut vect_sampled, veccard_allowed);
    }
}

fn determine_best_card(
    game: &SGame,
    itahand: impl Iterator<Item=EnumMap<EPlayerIndex, SHand>>,
    func_filter_allowed_cards: &(impl Fn(&SStichSequence, &mut SHandVector) + std::marker::Sync),
    foreachsnapshot: &(impl TForEachSnapshot<Output=isize> + Sync),
    ostr_file_out: Option<&str>,
) -> SCard {
    let (veccard_allowed, mapcardn_payout) = determine_best_card_internal(
        game,
        itahand,
        func_filter_allowed_cards,
        foreachsnapshot,
        ostr_file_out,
    );
    verify!(veccard_allowed.into_iter()
        .max_by_key(|card| mapcardn_payout[*card]))
        .unwrap()
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
    let test_game = |aacard_hand: [[SCard; 8]; 4], rules: &TRules, epi_first, vectestaction: Vec<VTestAction>| {
        let ahand = EPlayerIndex::map_from_raw(aacard_hand)
            .map(|acard_hand|
                SHand::new_from_vec(acard_hand.into_iter().cloned().collect())
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
                    for card in acard.into_iter() {
                        let epi = verify!(game.which_player_can_do_something()).unwrap().0;
                        verify!(game.zugeben(*card, epi)).unwrap();
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
                game.ahand[verify!(game.which_player_can_do_something()).unwrap().0].clone(),
                verify!(game.which_player_can_do_something()).unwrap().0,
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
    use crate::rules::{ruleset::*, rulessolo::*, payoutdecider::*, trumpfdecider::*, tests::TPayoutDeciderSoloLikeDefault};
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
            SHand::new_from_vec(acard_hand.into_iter().cloned().collect())
        ),
        SDoublings::new(epi_first_and_active_player),
        Some(SStossParams::new(
            /*n_stoss_max*/ 4,
        )),
        TRules::box_clone(&SRulesSoloLike::<SCoreSolo<STrumpfDeciderFarbe<SStaticFarbeHerz>>, SPayoutDeciderPointBased<VGameAnnouncementPrioritySoloLike>>::new(
            epi_first_and_active_player,
            SPayoutDeciderPointBased::default_payoutdecider(n_payout_base, n_payout_schneider_schwarz, SLaufendeParams::new(10, 3)),
            /*str_rulename*/"-", // should not matter within those tests
        )),
        /*n_stock*/ 0,
    );
    for acard_stich in [[EO, GO, HO, SO], [EU, GU, HU, SU], [HA, E7, E8, E9], [HZ, S7, S8, S9], [HK, G7, G8, G9]].into_iter() {
        assert_eq!(EPlayerIndex::values().nth(0), Some(epi_first_and_active_player));
        for (epi, card) in EPlayerIndex::values().zip(acard_stich.into_iter()) {
            verify!(game.zugeben(*card, epi)).unwrap();
        }
    }
    for ahand in all_possible_hands(
        &game.stichseq,
        game.ahand[epi_first_and_active_player].clone(),
        epi_first_and_active_player,
        game.rules.as_ref(),
    ) {
        let (veccard_allowed, mapcardpayout) = determine_best_card_internal(
            &game,
            Some(ahand).into_iter(),
            /*func_filter_allowed_cards*/&branching_factor(|_stichseq| (1, 2)),
            &SMinReachablePayout(SMinReachablePayoutParams::new_from_game(&game)),
            /*ostr_file_out*/None, //Some(&format!("suspicion_test/{:?}", ahand)), // to inspect search tree
        );
        for card in [H7, H8, H9].into_iter() {
            assert!(veccard_allowed.contains(card));
            assert!(
                mapcardpayout[*card] == std::isize::MAX
                || mapcardpayout[*card] == 3*(n_payout_base+2*n_payout_schneider_schwarz)
            );
        }
    }
}
