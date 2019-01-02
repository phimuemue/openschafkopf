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
        atomic::{AtomicIsize, Ordering},
    },
    cmp,
};
use crossbeam;
use crate::util::*;

pub fn remaining_cards_per_hand(stichseq: &SStichSequence, ekurzlang: EKurzLang) -> EnumMap<EPlayerIndex, usize> {
    EPlayerIndex::map_from_fn(|epi| {
        ekurzlang.cards_per_player()
            - stichseq.completed_stichs().get().len()
            - match stichseq.current_stich().get(epi) {
                None => 0,
                Some(_card) => 1,
            }
    })
}

pub fn ahand_vecstich_card_count_is_compatible(stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, ekurzlang: EKurzLang) -> bool {
    ahand.map(|hand| hand.cards().len()) == remaining_cards_per_hand(stichseq, ekurzlang)
}

pub trait TAi {
    fn rank_rules(&self, hand_fixed: SFullHand, epi_first: EPlayerIndex, epi_rank: EPlayerIndex, rules: &TRules, n_stock: isize) -> f64;
    fn suggest_card(&self, game: &SGame, ostr_file_out: Option<&str>) -> SCard {
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
            self.internal_suggest_card(game, ostr_file_out)
        }
    }
    fn internal_suggest_card(&self, game: &SGame, ostr_file_out: Option<&str>) -> SCard;
}

pub fn random_sample_from_vec(vect: &mut SHandVector, n_size: usize) {
    assert!(n_size<=vect.len());
    let vect_sampled_tmp = vect.iter().cloned().choose_multiple(&mut rand::thread_rng(), n_size); // TODO can we choose_multiple directly into SHandVector?
    let mut vect_sampled = vect_sampled_tmp.into_iter().collect::<SHandVector>();
    assert_eq!(vect_sampled.len(), n_size);
    mem::swap(&mut vect_sampled, vect);
}

pub fn unplayed_cards<'lifetime>(stichseq: &'lifetime SStichSequence, hand_fixed: &'lifetime SHand, ekurzlang: EKurzLang) -> impl Iterator<Item=SCard> + 'lifetime {
    SCard::values(ekurzlang)
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
    let veccard_unplayed = unplayed_cards(&stichseq, &hand, EKurzLang::Lang).collect::<Vec<_>>();
    let veccard_unplayed_check = [GZ, E7, SZ, H9, EZ, GU];
    assert_eq!(veccard_unplayed.len(), veccard_unplayed_check.len());
    assert!(veccard_unplayed.iter().all(|card| veccard_unplayed_check.contains(card)));
    assert!(veccard_unplayed_check.iter().all(|card| veccard_unplayed.contains(card)));
}

#[derive(new)]
pub struct SAiCheating {
    n_rank_rules_samples: usize,
}

impl TAi for SAiCheating {
    fn rank_rules (&self, hand_fixed: SFullHand, epi_first: EPlayerIndex, epi_rank: EPlayerIndex, rules: &TRules, n_stock: isize) -> f64 {
        // TODO: adjust interface to get whole game
        SAiSimulating::new(
            /*n_suggest_card_branches*/2,
            /*n_suggest_card_samples*/10,
            self.n_rank_rules_samples,
        ).rank_rules(hand_fixed, epi_first, epi_rank, rules, n_stock)
    }

    fn internal_suggest_card(&self, game: &SGame, ostr_file_out: Option<&str>) -> SCard {
        determine_best_card(
            game,
            Some(game.ahand.clone()).into_iter(),
            /*n_branches*/1,
            &SMinReachablePayout(SMinReachablePayoutParams::new_from_game(game)),
            ostr_file_out,
        )
    }
}

fn determine_best_card_internal(
    game: &SGame,
    itahand: impl Iterator<Item=EnumMap<EPlayerIndex, SHand>>,
    n_branches: usize,
    foreachsnapshot: &(impl TForEachSnapshot<Output=isize> + Send + Clone),
    ostr_file_out: Option<&str>
) -> (SHandVector, EnumMap<SCard, isize>) {
    let epi_fixed = verify!(game.current_playable_stich().current_playerindex()).unwrap();
    let veccard_allowed = game.rules.all_allowed_cards(&game.stichseq, &game.ahand[epi_fixed]);
    let mapcardn_payout = Arc::new(Mutex::new(
        // aggregate n_payout per card in some way
        SCard::map_from_fn(|_card| std::isize::MAX),
    ));
    verify!(crossbeam::scope(|scope| {
        for (i_susp, ahand) in itahand.enumerate() {
            for &card in veccard_allowed.iter() {
                debug_assert!(ahand[epi_fixed].cards().contains(&card));
                let mut ahand = ahand.clone();
                let mut foreachsnapshot = foreachsnapshot.clone();
                let mapcardn_payout = Arc::clone(&mapcardn_payout);
                scope.spawn(move |_scope| {
                    assert!(ahand_vecstich_card_count_is_compatible(&game.stichseq, &ahand, game.kurzlang()));
                    let mut stichseq = game.stichseq.clone();
                    ahand[epi_fixed].play_card(card);
                    stichseq.zugeben(card, game.rules.as_ref());
                    let n_payout = explore_snapshots(
                        epi_fixed,
                        &mut ahand,
                        game.rules.as_ref(),
                        &mut stichseq,
                        &|slcstich, veccard_allowed| {
                            assert!(!veccard_allowed.is_empty());
                            assert!(slcstich.count_played_cards()>=game.stichseq.count_played_cards());
                            if slcstich.count_played_cards()!=game.stichseq.count_played_cards() && game.completed_stichs().get().len() < 6 {
                                // TODO: maybe keep more than one successor stich
                                random_sample_from_vec(
                                    veccard_allowed,
                                    std::cmp::min(
                                        veccard_allowed.len(),
                                        rand::thread_rng().gen_range(
                                            1,
                                            {assert!(0<n_branches); n_branches+1}
                                        ),
                                    ),
                                );
                            } else {
                                // if slcstich>=6, we hope that we can compute everything
                            }
                        },
                        &mut foreachsnapshot,
                        ostr_file_out.map(|str_file_out| {
                            verify!(std::fs::create_dir_all(str_file_out)).unwrap();
                            format!("{}/{}_{}", str_file_out, i_susp, card)
                        }).as_ref().map(|str_file_out| &str_file_out[..]),
                    );
                    let mut mapcardn_payout = verify!(mapcardn_payout.lock()).unwrap();
                    mapcardn_payout[card] = cmp::min(mapcardn_payout[card], n_payout);
                });
            }
        }
    })).unwrap();
    let mapcardn_payout = verify!(
        verify!(Arc::try_unwrap(mapcardn_payout)).unwrap() // "Returns the contained value, if the Arc has exactly one strong reference"   
            .into_inner() // "If another user of this mutex panicked while holding the mutex, then this call will return an error instead"
    ).unwrap();
    assert!(<SCard as TPlainEnum>::values().any(|card| {
        veccard_allowed.contains(&card) && mapcardn_payout[card] < std::isize::MAX
    }));
    (veccard_allowed, mapcardn_payout)
}

fn determine_best_card(
    game: &SGame,
    itahand: impl Iterator<Item=EnumMap<EPlayerIndex, SHand>>,
    n_branches: usize,
    foreachsnapshot: &(impl TForEachSnapshot<Output=isize> + Send + Clone),
    ostr_file_out: Option<&str>,
) -> SCard {
    let (veccard_allowed, mapcardn_payout) = determine_best_card_internal(
        game,
        itahand,
        n_branches,
        foreachsnapshot,
        ostr_file_out,
    );
    verify!(veccard_allowed.into_iter()
        .max_by_key(|card| mapcardn_payout[*card]))
        .unwrap()
}

#[derive(new)]
pub struct SAiSimulating {
    n_suggest_card_branches: usize,
    n_suggest_card_samples: usize,
    n_rank_rules_samples: usize,
}

impl TAi for SAiSimulating {
    fn rank_rules (&self, hand_fixed: SFullHand, epi_first: EPlayerIndex, epi_rank: EPlayerIndex, rules: &TRules, n_stock: isize) -> f64 {
        let n_payout_sum = Arc::new(AtomicIsize::new(0));
        let ekurzlang = EKurzLang::from_cards_per_player(hand_fixed.get().cards().len());
        verify!(crossbeam::scope(|scope| {
            for mut ahand in forever_rand_hands(&SStichSequence::new(epi_first, ekurzlang), hand_fixed.get().clone(), epi_rank, ekurzlang, rules).take(self.n_rank_rules_samples) {
                let n_payout_sum = Arc::clone(&n_payout_sum);
                scope.spawn(move |_scope| {
                    let n_payout = explore_snapshots(
                        epi_rank,
                        &mut ahand,
                        rules,
                        &mut SStichSequence::new(epi_first, ekurzlang),
                        &|_slcstich, veccard_allowed| {
                            assert!(!veccard_allowed.is_empty());
                            random_sample_from_vec(veccard_allowed, 1);
                        },
                        &mut SMinReachablePayoutLowerBoundViaHint(SMinReachablePayoutParams::new(
                            rules,
                            epi_rank,
                            /*tpln_stoss_doubling*/(0, 0), // TODO do we need tpln_stoss_doubling from somewhere? 
                            n_stock,
                        )),
                        /*ostr_file_out*/None,
                    );
                    n_payout_sum.fetch_add(n_payout, Ordering::SeqCst);
                });
            }
        })).unwrap();
        let n_payout_sum = n_payout_sum.load(Ordering::SeqCst);
        (n_payout_sum.as_num::<f64>()) / (self.n_rank_rules_samples.as_num::<f64>())
    }

    fn internal_suggest_card(&self, game: &SGame, ostr_file_out: Option<&str>) -> SCard {
        let stich_current = game.current_playable_stich();
        assert!(!stich_current.is_full());
        let epi_fixed = verify!(stich_current.current_playerindex()).unwrap();
        let hand_fixed = &game.ahand[epi_fixed];
        assert!(!hand_fixed.cards().is_empty());
        macro_rules! forward_to_determine_best_card{($itahand: expr, $foreachsnapshot: expr,) => { // TODORUST generic closures
            determine_best_card(
                game,
                $itahand,
                self.n_suggest_card_branches,
                $foreachsnapshot,
                ostr_file_out,
            )
        }}
        if hand_fixed.cards().len()<=2 {
            forward_to_determine_best_card!(
                all_possible_hands(&game.stichseq, hand_fixed.clone(), epi_fixed, game.kurzlang(), game.rules.as_ref()),
                &SMinReachablePayout(SMinReachablePayoutParams::new_from_game(game)),
            )
        } else if hand_fixed.cards().len()<=5 {
            forward_to_determine_best_card!(
                forever_rand_hands(&game.stichseq, hand_fixed.clone(), epi_fixed, game.kurzlang(), game.rules.as_ref())
                    .take(self.n_suggest_card_samples),
                &SMinReachablePayoutLowerBoundViaHint(SMinReachablePayoutParams::new_from_game(game)),
            )
        } else {
            forward_to_determine_best_card!(
                forever_rand_hands(&game.stichseq, hand_fixed.clone(), epi_fixed, game.kurzlang(), game.rules.as_ref())
                    .take(self.n_suggest_card_samples),
                &SMinReachablePayout(SMinReachablePayoutParams::new_from_game(game)),
            )
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
                game.kurzlang(),
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
        game.kurzlang(),
        game.rules.as_ref(),
    ) {
        let (veccard_allowed, mapcardpayout) = determine_best_card_internal(
            &game,
            Some(ahand).into_iter(),
            /*n_branches*/1,
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
