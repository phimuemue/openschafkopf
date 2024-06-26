use crate::ai::{gametree::*, *};
use crate::game;
use crate::player::{playerrandom::SPlayerRandom, TPlayer};
use crate::primitives::*;
use crate::rules::{ruleset::*, *};
use crate::util::*;

#[test]
fn test_determine_best_card() {
    // https://www.sauspiel.de/spiele/785105783
    use crate::primitives::card::ECard::*;
    let mut game = game::SGame::new(
        EPlayerIndex::map_from_raw([
            [EO, HO, EU, SU, HZ, E8, SZ, S7],
            [HK, EK, E7, GA, GZ, G9, G8, S8],
            [GO, SO, GU, HU, HA, EA, EZ, G7],
            [H9, H8, H7, E9, GK, SA, SK, S9],
        ]).map_into(|acard| acard.into()),
        game::SExpensifiersNoStoss::new(/*n_stock*/0),
        SActivelyPlayableRules::from(rulesrufspiel::SRulesRufspiel::new(
            EPlayerIndex::EPI0,
            EFarbe::Eichel,
            payoutdecider::SPayoutDeciderParams::new(
                /*n_payout_base*/100,
                /*n_payout_schneider_schwarz*/50,
                payoutdecider::SLaufendeParams::new(
                    /*n_payout_per_lauf*/50,
                    /*n_lauf_lbound*/3,
                ),
            ),
            SStossParams::new(
                /*n_stoss_max*/4,
            ),
        )).into(),
    );
    fn play_stichs(game: &mut SGame, slctplepistich: &[(EPlayerIndex, [ECard; EPlayerIndex::SIZE])]) {
        for (epi, card) in slctplepistich.iter()
            .flat_map(|&(epi_first, acard)| SStich::new_full(epi_first, acard))
        {
            unwrap!(game.zugeben(card, epi));
        }
    }
    play_stichs(&mut game, &[
        (EPlayerIndex::EPI0, [EO, HK, HA, H8]),
        (EPlayerIndex::EPI0, [SU, GA, GO, H9]),
        (EPlayerIndex::EPI2, [SO, H7, HZ, S8]),
        // I suppose that playing GU is wrong at this point
        // TODO update AI to recognize this and assert
        (EPlayerIndex::EPI2, [GU, E9, EU, G8]),
    ]);
    let aicheating = SAi::new_cheating(
        /*n_rank_rules_samples*/1,
        /*n_suggest_card_branches*/2,
    );
    #[cfg(not(debug_assertions))]
    let aisimulating = SAi::new_simulating(
        /*n_suggest_card_branches*/1,
        /*n_suggest_card_samples*/1,
        /*n_samples_per_rules*/1,
    );
    // If we cheat (i.e. we know each players' cards), it makes - intuitively, not mathematically
    // proven - sense not to play HO since it only weakens the own partner.
    assert_ne!(aicheating.suggest_card(&game, SNoVisualization::factory()), HO);
    // If we do not cheat, tests indicated that playing HO is the best solution.
    // As far as I can tell, it is at least not necessarily wrong.
    // (HO ensures at least that no other player can take away rufsau.)
    // TODO examine optimal solution to this case.
    #[cfg(not(debug_assertions))] {
        assert_eq!(aisimulating.suggest_card(&game, SNoVisualization::factory()), HO);
    }
    play_stichs(&mut game, &[
        (EPlayerIndex::EPI0, [HO, E7, HU, GK]),
    ]);
    #[cfg(not(debug_assertions))] {
        assert_eq!(aicheating.suggest_card(&game, SNoVisualization::factory()), E8);
        assert_eq!(aisimulating.suggest_card(&game, SNoVisualization::factory()), E8);
    }
    play_stichs(&mut game, &[
        (EPlayerIndex::EPI0, [SZ, EK, G7, SA]),
        (EPlayerIndex::EPI3, [SK, S7, GZ, EZ]),
        (EPlayerIndex::EPI3, [S9, E8, G9, EA]),
    ]);
}

#[test]
fn detect_expensive_all_possible_hands() {
    crate::game::run::run_simple_game_loop(
        EPlayerIndex::map_from_fn(|_epi| Box::new(SPlayerRandom::new(
            /*fn_check_ask_for_card*/|game: &SGameGeneric<SRuleSet, (), ()>| {
                if game.kurzlang().cards_per_player() - 4 < game.completed_stichs().len() {
                    let epi_current = unwrap!(game.current_playable_stich().current_playerindex());
                    let vecahand = all_possible_hands(
                        &game.stichseq,
                        (game.ahand[epi_current].clone(), epi_current),
                        &game.rules,
                        &game.expensifiers.vecstoss,
                    )
                        .collect::<Vec<_>>();
                    let assert_bound = |n, n_detect| {
                        assert!(n < n_detect,
                            "n: {}\nrules: {}\nahand: {}\nvecstich:{}",
                            n,
                            game.rules,
                            display_card_slices(&game.ahand, &game.rules, " | "),
                            game.stichseq.visible_stichs().iter().join(", "),
                        );
                    };
                    assert_bound(vecahand.len(), 2000);
                    for mut ahand in vecahand {
                        struct SLeafCounter;
                        impl TForEachSnapshot for SLeafCounter {
                            type Output = usize;
                            type InfoFromParent = ();
                            fn initial_info_from_parent() -> Self::InfoFromParent {
                            }
                            fn final_output(&self, _stichseq: SStichSequenceGameFinished, _rulestatecache: &SRuleStateCache) -> Self::Output {
                                1 // leaf
                            }
                            fn pruned_output(&self, _tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), _rulestatecache: &SRuleStateCache) -> Option<Self::Output> {
                                None
                            }
                            fn combine_outputs(
                                &self,
                                _epi_card: EPlayerIndex,
                                _infofromparent: Self::InfoFromParent,
                                itcard_allowed: impl Iterator<Item=ECard>,
                                mut fn_card_to_output: impl FnMut(ECard, Self::InfoFromParent) -> Self::Output,
                            ) -> Self::Output {
                                itcard_allowed.map(|card| fn_card_to_output(card, ())).sum()
                            }
                        }
                        assert_bound(
                            explore_snapshots(
                                (&mut ahand, &mut game.stichseq.clone()),
                                &game.rules,
                                &SNoFilter::factory(),
                                &SLeafCounter{},
                                &SSnapshotCacheNone::factory(),
                                &mut SNoVisualization,
                            ),
                            2000
                        );
                    }
                }
            },
        )) as Box<dyn TPlayer>),
        /*n_games*/4,
        unwrap!(SRuleSet::from_string(
            r"
            base-price=10
            solo-price=50
            lauf-min=3
            [rufspiel]
            [solo]
            [wenz]
            lauf-min=2
            [stoss]
            max=3
            ",
        )),
        /*fn_print_account_balance*/|_,_| {/* no output */},
    );
}

// TODO (Sauspiel 964899954)
// Rufspiel(EPI2), EPI2 is first
// 2 EK H7 E7 EA
// 3 S7 S8 Sa S9
// 1 HK HO H8 EO
// 0 E9 EZ HA EU
// 3 SZ SO G7 GO // TODO is SO best choice here?
// 2 SU H9 Hz HU
// 1 GK G9 GZ GA
// 0 E8 G8 GU SK
