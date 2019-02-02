use crate::game_loop_cli;
use crate::primitives::*;
use crate::rules::{
    *,
    ruleset::*,
};
use crate::player::{
    TPlayer,
    playerrandom::SPlayerRandom,
};
use crate::util::*;
use crate::ai::{
    *,
    suspicion::*,
};
use crate::game;

#[test]
fn test_determine_best_card() {
    // https://www.sauspiel.de/spiele/785105783
    use crate::card::card_values::*;
    let epi_first_and_active_player = EPlayerIndex::EPI0;
    let mut game = game::SGame::new(
        EPlayerIndex::map_from_raw([
            [EO, HO, EU, SU, HZ, E8, SZ, S7],
            [HK, EK, E7, GA, GZ, G9, G8, S8],
            [GO, SO, GU, HU, HA, EA, EZ, G7],
            [H9, H8, H7, E9, GK, SA, SK, S9],
        ]).map(|acard_hand|
            SHand::new_from_vec(acard_hand.iter().cloned().collect())
        ),
        game::SDoublings::new(epi_first_and_active_player),
        Some(SStossParams::new(
            /*n_stoss_max*/4,
        )),
        TRules::box_clone(&rulesrufspiel::SRulesRufspiel::new(epi_first_and_active_player, EFarbe::Eichel, payoutdecider::SPayoutDeciderParams::new(
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
                verify!(game.zugeben(*card, epi)).unwrap();
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
    let aisimulating = SAi::new_simulating(
        /*n_suggest_card_branches*/1,
        /*n_suggest_card_samples*/1,
        /*n_samples_per_rules*/1,
    );
    // If we cheat (i.e. we know each players' cards), it makes - intuitively, not mathematically
    // proven - sense not to play HO since it only weakens the own partner.
    assert_ne!(aicheating.suggest_card(&game, /*ostr_file_out*/None), HO);
    // If we do not cheat, tests indicated that playing HO is the best solution.
    // As far as I can tell, it is at least not necessarily wrong.
    // (HO ensures at least that no other player can take away rufsau.)
    // TODO examine optimal solution to this case.
    assert_eq!(aisimulating.suggest_card(&game, /*ostr_file_out*/None), HO);
    play_stichs(&mut game, &[
        (EPlayerIndex::EPI0, [HO, E7, HU, GK]),
    ]);
    // TODO these asserts should hold
    //assert_ne!(aicheating.suggest_card(&game, /*ostr_file_out*/None), SZ);
    //assert_ne!(aisimulating.suggest_card(&game, /*ostr_file_out*/None), SZ);
    //assert_eq!(aicheating.suggest_card(&game, /*ostr_file_out*/None), E8);
    //assert_eq!(aisimulating.suggest_card(&game, /*ostr_file_out*/None), E8);
    play_stichs(&mut game, &[
        (EPlayerIndex::EPI0, [SZ, EK, G7, SA]),
        (EPlayerIndex::EPI3, [SK, S7, GZ, EZ]),
        (EPlayerIndex::EPI3, [S9, E8, G9, EA]),
    ]);
}

#[test]
fn detect_expensive_all_possible_hands() {
    game_loop_cli(
        &EPlayerIndex::map_from_fn(|_epi| Box::new(SPlayerRandom::new(
            /*fn_check_ask_for_card*/|game| {
                if game.kurzlang().cards_per_player() - 4 < game.completed_stichs().len() {
                    let epi_fixed = verify!(game.current_playable_stich().current_playerindex()).unwrap();
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
                                _epi_self: EPlayerIndex,
                                _epi_card: EPlayerIndex,
                                ittplcardoutput: ItTplCardOutput,
                            ) -> Self::Output {
                                ittplcardoutput.map(|tplcardoutput| tplcardoutput.1).sum()
                            }
                        }
                        assert_bound(
                            explore_snapshots(
                                EPlayerIndex::EPI0, // TODO do this for all EPlayerIndex::values()?
                                &mut ahand,
                                game.rules.as_ref(),
                                &mut game.stichseq.clone(),
                                &|_vecstich_complete, _vecstich_successor| {/*no filtering*/},
                                &SLeafCounter{},
                                /*ostr_file_out*/None,
                            ),
                            2000
                        );
                    }
                }
            },
        )) as Box<TPlayer>),
        /*n_games*/4,
        &verify!(SRuleSet::from_string(
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
        )).unwrap()
    );
}
