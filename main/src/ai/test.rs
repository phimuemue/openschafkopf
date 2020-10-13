use crate::ai::{suspicion::*, *};
use crate::game;
use crate::player::{playerrandom::SPlayerRandom, TPlayer};
use crate::primitives::*;
use crate::rules::{ruleset::*, *};
use crate::util::*;

#[test]
fn test_determine_best_card() {
    // https://www.sauspiel.de/spiele/785105783
    use crate::card::card_values::*;
    let mut game = game::SGame::new(
        EPlayerIndex::map_from_raw([
            [EO, HO, EU, SU, HZ, E8, SZ, S7],
            [HK, EK, E7, GA, GZ, G9, G8, S8],
            [GO, SO, GU, HU, HA, EA, EZ, G7],
            [H9, H8, H7, E9, GK, SA, SK, S9],
        ]).map(|acard_hand|
            SHand::new_from_vec(acard_hand.iter().copied().collect())
        ),
        game::SDoublings::new(SStaticEPI0{}),
        Some(SStossParams::new(
            /*n_stoss_max*/4,
        )),
        TRules::box_clone(&rulesrufspiel::SRulesRufspiel::new(EPlayerIndex::EPI0, EFarbe::Eichel, payoutdecider::SPayoutDeciderParams::new(
            /*n_payout_base*/100,
            /*n_payout_schneider_schwarz*/50,
            payoutdecider::SLaufendeParams::new(
                /*n_payout_per_lauf*/50,
                /*n_lauf_lbound*/3,
            ),
        ))),
        /*n_stock*/0,
    );
    fn play_stichs(game: &mut SGame, slctplepistich: &[(EPlayerIndex, [SCard; 4])]) {
        for (epi_stich_first, acard_stich) in slctplepistich.iter() {
            for (epi, card) in EPlayerIndex::values()
                .map(|epi| epi.wrapping_add(epi_stich_first.to_usize()))
                .zip(acard_stich.iter())
            {
                unwrap!(game.zugeben(*card, epi));
            }
        }
    };
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
    assert_ne!(aicheating.suggest_card(&game, /*opath_out_dir*/None), HO);
    // If we do not cheat, tests indicated that playing HO is the best solution.
    // As far as I can tell, it is at least not necessarily wrong.
    // (HO ensures at least that no other player can take away rufsau.)
    // TODO examine optimal solution to this case.
    #[cfg(not(debug_assertions))] {
        assert_eq!(aisimulating.suggest_card(&game, /*opath_out_dir*/None), HO);
    }
    play_stichs(&mut game, &[
        (EPlayerIndex::EPI0, [HO, E7, HU, GK]),
    ]);
    #[cfg(not(debug_assertions))] {
        assert_eq!(aicheating.suggest_card(&game, /*opath_out_dir*/None), E8);
        assert_eq!(aisimulating.suggest_card(&game, /*opath_out_dir*/None), E8);
    }
    play_stichs(&mut game, &[
        (EPlayerIndex::EPI0, [SZ, EK, G7, SA]),
        (EPlayerIndex::EPI3, [SK, S7, GZ, EZ]),
        (EPlayerIndex::EPI3, [S9, E8, G9, EA]),
    ]);
}

#[test]
fn detect_expensive_all_possible_hands() {
    crate::subcommands::cli::game_loop_cli_internal(
        EPlayerIndex::map_from_fn(|_epi| Box::new(SPlayerRandom::new(
            /*fn_check_ask_for_card*/|game: &SGame| {
                if game.kurzlang().cards_per_player() - 4 < game.completed_stichs().len() {
                    let epi_fixed = unwrap!(game.current_playable_stich().current_playerindex());
                    let vecahand = all_possible_hands(
                        &game.stichseq,
                        game.ahand[epi_fixed].clone(),
                        epi_fixed,
                        game.rules.as_ref()
                    )
                        .collect::<Vec<_>>();
                    let assert_bound = |n, n_detect| {
                        assert!(n < n_detect,
                            "n: {}\nrules: {}\nahand: {:#?}\nvecstich:{:?}",
                            n,
                            game.rules,
                            game.ahand.iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>(),
                            game.stichseq.visible_stichs()
                                .iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>(),
                        );
                    };
                    assert_bound(vecahand.len(), 2000);
                    for mut ahand in vecahand {
                        struct SLeafCounter;
                        impl TForEachSnapshot for SLeafCounter {
                            type Output = usize;
                            fn final_output(&self, _slcstich: SStichSequenceGameFinished, _rulestatecache: &SRuleStateCache) -> Self::Output {
                                1 // leaf
                            }
                            fn pruned_output(&self, _stichseq: &SStichSequence, _ahand: &EnumMap<EPlayerIndex, SHand>, _rulestatecache: &SRuleStateCache) -> Option<Self::Output> {
                                None
                            }
                            fn combine_outputs<ItTplCardOutput: Iterator<Item=(SCard, Self::Output)>>(
                                &self,
                                _epi_card: EPlayerIndex,
                                ittplcardoutput: ItTplCardOutput,
                            ) -> Self::Output {
                                ittplcardoutput.map(|tplcardoutput| tplcardoutput.1).sum()
                            }
                        }
                        assert_bound(
                            explore_snapshots(
                                &mut ahand,
                                game.rules.as_ref(),
                                &mut game.stichseq.clone(),
                                &|_vecstich_complete, _vecstich_successor| {/*no filtering*/},
                                &SLeafCounter{},
                                /*opath_out_dir*/None,
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
        ))
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
