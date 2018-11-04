pub mod suspicion;
pub mod handiterators;
pub mod rulespecific;
#[cfg(test)]
pub mod test;

use primitives::*;
use rules::{
    *,
};
use game::*;
use ai::{
    suspicion::*,
    handiterators::*,
};
use rand;
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
use util::*;

pub trait TAi {
    fn rank_rules(&self, hand_fixed: SFullHand, epi_first: EPlayerIndex, epi_rank: EPlayerIndex, rules: &TRules, n_stock: isize) -> f64;
    fn suggest_card(&self, game: &SGame, ostr_file_out: Option<&str>) -> SCard {
        let veccard_allowed = game.rules.all_allowed_cards(
            &game.vecstich,
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

pub fn random_sample_from_vec(vecstich: &mut Vec<SStich>, n_size: usize) {
    let mut vecstich_sample = match rand::seq::sample_iter(&mut rand::thread_rng(), vecstich.iter().cloned(), n_size) {
        Ok(vecstich) | Err(vecstich) => vecstich,
    };
    mem::swap(vecstich, &mut vecstich_sample);
}

pub fn unplayed_cards<'lifetime>(slcstich: &'lifetime [SStich], hand_fixed: &'lifetime SHand) -> impl Iterator<Item=SCard> + 'lifetime {
    assert!(slcstich.iter().all(|stich| 4==stich.size())); // TODO is this really required?
    SCard::values(EKurzLang::from_cards_per_player(slcstich.len() + hand_fixed.cards().len()))
        .filter(move |card| 
             !hand_fixed.contains(*card)
             && !slcstich.iter().any(|stich|
                stich.iter().any(|(_epi, card_played)|
                    card_played==card
                )
             )
        )
}

#[test]
fn test_unplayed_cards() {
    use card::card_values::*;
    let vecstich = [[G7, G8, GA, G9], [S8, HO, S7, S9], [H7, HK, HU, SU], [EO, GO, HZ, H8], [E9, EK, E8, EA], [SA, EU, SO, HA]].into_iter()
        .map(|acard| {
            SStich::new_full(/*epi_first irrelevant*/EPlayerIndex::EPI0, *acard)
        })
        .collect::<Vec<_>>();
    let hand = &SHand::new_from_vec([GK, SK].into_iter().cloned().collect());
    let veccard_unplayed = unplayed_cards(&vecstich, &hand).collect::<Vec<_>>();
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
            Some(EPlayerIndex::map_from_fn(|epi|
                SHand::new_from_vec(
                    game.current_stich().get(epi).cloned().into_iter()
                        .chain(game.ahand[epi].cards().iter().cloned())
                        .collect()
                )
            )).into_iter(),
            /*n_branches*/1,
            ostr_file_out,
        )
    }
}

pub fn is_compatible_with_game_so_far(
    ahand: &EnumMap<EPlayerIndex, SHand>,
    rules: &TRules,
    slcstich: &[SStich],
) -> bool {
    let stich_current = current_stich(slcstich);
    assert!(stich_current.size()<4);
    // hands must contain respective cards from stich_current...
    stich_current.iter()
        .all(|(epi, card)| ahand[epi].contains(*card))
    // ... and must not contain other cards preventing farbe/trumpf frei
    && {
        let mut vecstich_complete_and_current_stich = completed_stichs(slcstich).get().to_vec();
        vecstich_complete_and_current_stich.push(SStich::new(stich_current.first_playerindex()));
        stich_current.iter()
            .all(|(epi, card_played)| {
                let b_valid = rules.card_is_allowed(
                    &vecstich_complete_and_current_stich,
                    &ahand[epi],
                    *card_played
                );
                current_stich_mut(&mut vecstich_complete_and_current_stich).push(*card_played);
                b_valid
            })
    }
    && {
        assert_ahand_same_size(ahand);
        let mut ahand_simulate = ahand.clone();
        for stich in completed_stichs(slcstich).get().iter().rev() {
            for epi in EPlayerIndex::values() {
                ahand_simulate[epi].cards_mut().push(stich[epi]);
            }
        }
        assert_ahand_same_size(&ahand_simulate);
        rules.playerindex().map_or(true, |epi_active|
            rules.can_be_played(SFullHand::new(
                &ahand_simulate[epi_active],
                {
                    let cards_per_player = |epi| {
                        completed_stichs(slcstich).get().len() + ahand[epi].cards().len()
                    };
                    assert!(EPlayerIndex::values().all(|epi| cards_per_player(epi)==cards_per_player(EPlayerIndex::EPI0)));
                    EKurzLang::from_cards_per_player(cards_per_player(EPlayerIndex::EPI0))
                },
            ))
        )
        && {
            let mut b_valid_up_to_now = true;
            let mut vecstich_simulate = Vec::new();
            'loopstich: for stich in completed_stichs(slcstich).get().iter() {
                vecstich_simulate.push(SStich::new(stich.epi_first));
                for (epi, card) in stich.iter() {
                    if rules.card_is_allowed(
                        &vecstich_simulate,
                        &ahand_simulate[epi],
                        *card
                    ) {
                        assert!(ahand_simulate[epi].contains(*card));
                        ahand_simulate[epi].play_card(*card);
                        current_stich_mut(&mut vecstich_simulate).push(*card);
                    } else {
                        b_valid_up_to_now = false;
                        break 'loopstich;
                    }
                }
            }
            b_valid_up_to_now
        }
    }
}

fn determine_best_card_internal<HandsIterator>(game: &SGame, itahand: HandsIterator, n_branches: usize, ostr_file_out: Option<&str>) -> (SHandVector, EnumMap<SCard, isize>)
    where HandsIterator: Iterator<Item=EnumMap<EPlayerIndex, SHand>>
{
    let epi_fixed = verify!(game.current_stich().current_playerindex()).unwrap();
    let mapcardn_payout = Arc::new(Mutex::new(
        // aggregate n_payout per card in some way
        SCard::map_from_fn(|_card| std::isize::MAX),
    ));
    crossbeam::scope(|scope| {
        for (i_susp, ahand) in itahand.enumerate() {
            let mapcardn_payout = Arc::clone(&mapcardn_payout);
            scope.spawn(move || {
                assert_ahand_same_size(&ahand);
                let mut vecstich_complete_mut = game.completed_stichs().get().to_vec();
                let n_stich_complete = vecstich_complete_mut.len();
                let (card, n_payout) = explore_snapshots(
                    &ahand,
                    game.rules.as_ref(),
                    &mut SVecStichPushPop::new(&mut vecstich_complete_mut),
                    game.current_stich(),
                    &|slcstich_complete_successor: &[SStich], vecstich_successor: &mut Vec<SStich>| {
                        assert!(!vecstich_successor.is_empty());
                        assert!(n_stich_complete<=slcstich_complete_successor.len());
                        if slcstich_complete_successor.len()!=n_stich_complete && n_stich_complete < 6 {
                            // TODO: maybe keep more than one successor stich
                            random_sample_from_vec(vecstich_successor, n_branches);
                        } else {
                            // if slcstich_complete_successor>=6, we hope that we can compute everything
                        }
                    },
                    &mut SMinReachablePayout::new(
                        game.rules.as_ref(),
                        epi_fixed,
                        /*tpln_stoss_doubling*/stoss_and_doublings(&game.vecstoss, &game.doublings),
                        game.n_stock,
                    ),
                    ostr_file_out.map(|str_file_out| {
                        verify!(std::fs::create_dir_all(str_file_out)).unwrap();
                        format!("{}/{}", str_file_out, i_susp)
                    }).as_ref().map(|str_file_out| &str_file_out[..]),
                );
                assert!(ahand[epi_fixed].cards().contains(&card));
                let mut mapcardn_payout = verify!(mapcardn_payout.lock()).unwrap();
                mapcardn_payout[card] = cmp::min(mapcardn_payout[card], n_payout);
            });
        }
    });
    let mapcardn_payout = verify!(
        verify!(Arc::try_unwrap(mapcardn_payout)).unwrap() // "Returns the contained value, if the Arc has exactly one strong reference"   
            .into_inner() // "If another user of this mutex panicked while holding the mutex, then this call will return an error instead"
    ).unwrap();
    let veccard_allowed_fixed = game.rules.all_allowed_cards(&game.vecstich, &game.ahand[epi_fixed]);
    assert!(<SCard as TPlainEnum>::values().any(|card| {
        veccard_allowed_fixed.contains(&card) && mapcardn_payout[card] < std::isize::MAX
    }));
    (veccard_allowed_fixed, mapcardn_payout)
}

fn determine_best_card<HandsIterator>(game: &SGame, itahand: HandsIterator, n_branches: usize, ostr_file_out: Option<&str>) -> SCard
    where HandsIterator: Iterator<Item=EnumMap<EPlayerIndex, SHand>>
{
    let (veccard_allowed, mapcardn_payout) = determine_best_card_internal(game, itahand, n_branches, ostr_file_out);
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
        crossbeam::scope(|scope| {
            for ahand in forever_rand_hands(SCompletedStichs::new(&Vec::new()), hand_fixed.get(), epi_rank).take(self.n_rank_rules_samples) {
                let n_payout_sum = Arc::clone(&n_payout_sum);
                scope.spawn(move || {
                    let n_payout = 
                        explore_snapshots(
                            &ahand,
                            rules,
                            &mut SVecStichPushPop::new(&mut Vec::new()),
                            &SStich::new(epi_first),
                            &|_vecstich_complete, vecstich_successor| {
                                assert!(!vecstich_successor.is_empty());
                                random_sample_from_vec(vecstich_successor, 1);
                            },
                            &mut SMinReachablePayoutLowerBoundViaHint::new(
                                rules,
                                epi_rank,
                                /*tpln_stoss_doubling*/(0, 0), // TODO do we need tpln_stoss_doubling from somewhere? 
                                n_stock,
                            ),
                            /*ostr_file_out*/None,
                        ).1
                    ;
                    n_payout_sum.fetch_add(n_payout, Ordering::SeqCst);
                });
            }
        });
        let n_payout_sum = n_payout_sum.load(Ordering::SeqCst);
        (n_payout_sum.as_num::<f64>()) / (self.n_rank_rules_samples.as_num::<f64>())
    }

    fn internal_suggest_card(&self, game: &SGame, ostr_file_out: Option<&str>) -> SCard {
        let stich_current = game.current_stich();
        assert!(stich_current.size()<4);
        let epi_fixed = verify!(stich_current.current_playerindex()).unwrap();
        let hand_fixed = &game.ahand[epi_fixed];
        assert!(!hand_fixed.cards().is_empty());
        if hand_fixed.cards().len()<=2 {
            determine_best_card(
                game,
                all_possible_hands(game.completed_stichs(), hand_fixed.clone(), epi_fixed)
                    .filter(|ahand| is_compatible_with_game_so_far(ahand, game.rules.as_ref(), &game.vecstich)),
                self.n_suggest_card_branches,
                ostr_file_out,
            )
        } else {
            determine_best_card(
                game,
                forever_rand_hands(game.completed_stichs(), hand_fixed, epi_fixed)
                    .filter(|ahand| is_compatible_with_game_so_far(ahand, game.rules.as_ref(), &game.vecstich))
                    .take(self.n_suggest_card_samples),
                self.n_suggest_card_branches,
                ostr_file_out,
            )
        }
    }
}

#[test]
fn test_is_compatible_with_game_so_far() {
    use rules::rulesrufspiel::*;
    use rules::payoutdecider::*;
    use card::card_values::*;
    use game;
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
        use rules::ruleset::*;
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
                game.completed_stichs(),
                &game.ahand[verify!(game.which_player_can_do_something()).unwrap().0],
                verify!(game.which_player_can_do_something()).unwrap().0
            )
                .filter(|ahand| is_compatible_with_game_so_far(ahand, game.rules.as_ref(), &game.vecstich))
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
    use card::card_values::*;
    use game::*;
    use rules::{ruleset::*, rulessolo::*, payoutdecider::*, trumpfdecider::*, tests::TPayoutDeciderSoloLikeDefault};
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
        game.completed_stichs(),
        game.ahand[epi_first_and_active_player].clone(),
        epi_first_and_active_player,
    )
        .filter(|ahand| is_compatible_with_game_so_far(ahand, game.rules.as_ref(), &game.vecstich))
    {
        let (veccard_allowed, mapcardpayout) = determine_best_card_internal(
            &game,
            Some(ahand).into_iter(),
            /*n_branches*/1,
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
