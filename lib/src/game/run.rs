use super::*;
use crate::player::*;
use std::sync::mpsc;

pub struct SAtTable {
    pub player: Box<dyn TPlayer>,
    pub n_money: isize,
}

fn communicate_via_channel<T: std::fmt::Debug>(f: impl FnOnce(mpsc::Sender<T>)) -> T {
    let (txt, rxt) = mpsc::channel::<T>();
    f(txt);
    unwrap!(rxt.recv())
}

pub fn internal_run_simple_game_loop<ItStockOrGame: Iterator<Item=VStockOrT<SGameResult<SRuleSet>, SGameGeneric<SRuleSet, (), ()>>>>(
    aplayer: EnumMap<EPlayerIndex, Box<dyn TPlayer>>,
    n_games: usize,
    ruleset: SRuleSet,
    fn_gamepreparations_to_stockorgame: impl Fn(SGamePreparations, &EnumMap<EPlayerIndex, SAtTable>)->ItStockOrGame,
    fn_print_account_balance: impl Fn(&EnumMap<EPlayerIndex, isize>, isize),
) -> ([SAtTable; EPlayerIndex::SIZE], isize) {
    let mut aattable = aplayer.map_into(|player| SAtTable{player, n_money:0});
    let mut n_stock = 0;
    for _i_game in 0..n_games {
        let mut dealcards = SDealCards::new(ruleset.clone(), n_stock);
        while let Some(epi) = dealcards.which_player_can_do_something() {
            unwrap!(dealcards.announce_doubling(
                epi,
                /*b_doubling*/communicate_via_channel(|txb_doubling| {
                    aattable[epi].player.ask_for_doubling(
                        dealcards.first_hand_for(epi),
                        txb_doubling
                    );
                })
            ));
        }
        for stockorgame in fn_gamepreparations_to_stockorgame(unwrap!(dealcards.finish()), &aattable) {
            let gameresult = match stockorgame {
                VStockOrT::OrT(mut game) => {
                    while let Some(gameaction)=game.which_player_can_do_something() {
                        if !gameaction.1.is_empty() {
                            if let Some(epi_stoss) = gameaction.1.iter()
                                .find(|epi| {
                                    communicate_via_channel(|txb_stoss| {
                                        aattable[**epi].player.ask_for_stoss(
                                            **epi,
                                            &game.rules,
                                            &game.ahand[**epi],
                                            &game.stichseq,
                                            &game.expensifiers,
                                            txb_stoss,
                                        );
                                    })
                                })
                            {
                                unwrap!(game.stoss(*epi_stoss));
                                continue;
                            }
                        }
                        unwrap!(game.zugeben(
                            communicate_via_channel(|txcard| {
                                aattable[gameaction.0].player.ask_for_card(
                                    &game,
                                    txcard,
                                );
                            }),
                            gameaction.0
                        ));
                    }
                    unwrap!(game.finish())
                },
                VStockOrT::Stock(gameresult) => gameresult,
            };
            gameresult.apply_payout(&mut n_stock, |epi, n_payout| {
                aattable[epi].n_money += n_payout;
            });
            assert_eq!(n_stock + aattable.iter().map(|attable| attable.n_money).sum::<isize>(), 0);
            fn_print_account_balance(&aattable.map(|attable| attable.n_money), n_stock);
        }
        aattable.as_raw_mut().rotate_left(1);
    }
    (aattable.into_raw(), n_stock)
}

pub fn run_simple_game_loop(
    aplayer: EnumMap<EPlayerIndex, Box<dyn TPlayer>>,
    n_games: usize,
    ruleset: SRuleSet,
    fn_print_account_balance: impl Fn(&EnumMap<EPlayerIndex, isize>, isize),
) -> ([SAtTable; EPlayerIndex::SIZE], isize) {
    internal_run_simple_game_loop(
        aplayer,
        n_games,
        ruleset,
        /*fn_gamepreparations_to_stockorgame*/|mut gamepreparations, aattable| {
            while let Some(epi) = gamepreparations.which_player_can_do_something() {
                info!("Asking player {epi} for game");
                unwrap!(gamepreparations.announce_game(
                    epi,
                    communicate_via_channel(|txorules| {
                        aattable[epi].player.ask_for_game(
                            epi,
                            gamepreparations.fullhand(epi),
                            &gamepreparations.gameannouncements,
                            &gamepreparations.ruleset.avecrulegroup[epi],
                            &gamepreparations.expensifiers.clone().into_with_stoss(),
                            None,
                            txorules
                        );
                    }).cloned()
                ));
            }
            info!("Asked players if they want to play. Determining rules");
            std::iter::once(match unwrap!(gamepreparations.finish()) {
                VGamePreparationsFinish::DetermineRules(mut determinerules) => {
                    while let Some((epi, vecrulegroup_steigered))=determinerules.which_player_can_do_something() {
                        if let Some(rules) = communicate_via_channel(|txorules| {
                            aattable[epi].player.ask_for_game(
                                epi,
                                determinerules.fullhand(epi),
                                /*gameannouncements*/&SPlayersInRound::new(SStaticEPI0{}),
                                &vecrulegroup_steigered,
                                &determinerules.expensifiers.clone().into_with_stoss(),
                                Some(determinerules.currently_offered_prio()),
                                txorules
                            );
                        }).cloned() {
                            unwrap!(determinerules.announce_game(epi, rules));
                        } else {
                            unwrap!(determinerules.resign(epi));
                        }
                    }
                    VStockOrT::OrT(unwrap!(determinerules.finish()))
                },
                VGamePreparationsFinish::DirectGame(game) => {
                    VStockOrT::OrT(game)
                },
                VGamePreparationsFinish::Stock(gameresult) => {
                    VStockOrT::Stock(gameresult)
                }
            })
        },
        fn_print_account_balance,
    )
}

#[test]
fn test_game_loop() {
    use rand::prelude::IteratorRandom;
    use crate::ai;
    use crate::player::{
        *,
        playercomputer::*,
    };
    let mut rng = rand::rng();
    use itertools::iproduct;
    for ruleset in
        iproduct!(
            [10, 20], // n_base_price
            [50, 100], // n_solo_price
            [2, 3], // n_lauf_min
            [ // str_allowed_games
                r"
                [rufspiel]
                [solo]
                [wenz]
                lauf-min=2
                ",
                r"
                [solo]
                [farbwenz]
                [wenz]
                [geier]
                ",
                r"
                [solo]
                [wenz]
                [bettel]
                ",
                r"
                [solo]
                [wenz]
                [bettel]
                stichzwang=true
                ",
            ],
            [ // str_no_active_game
                r"[ramsch]
                price=20
                ",
                r"[ramsch]
                price=50
                durchmarsch = 75",
                r#"[ramsch]
                price=50
                durchmarsch = "all""#,
                r"[stock]",
                r"[stock]
                price=30",
                r"",
                r#"[ramsch]
                price=20
                jungfrau="DoubleAll"
                "#,
                r#"[ramsch]
                price=20
                jungfrau="DoubleIndividuallyOnce"
                "#,
                r#"[ramsch]
                price=20
                jungfrau="DoubleIndividuallyMultiple"
                "#,
            ],
            [ // str_extras
                r"[steigern]",
                r"[steigern]
                step=15
                ",
                r"[doubling]",
                r#"deck = "kurz""#,
                r"[stoss]",
                r"[stoss]
                max=3
                ",
            ]
        )
            .map(|(n_base_price, n_solo_price, n_lauf_min, str_allowed_games, str_no_active_game, str_extras)| {
                let str_ruleset = format!(
                    "base-price={n_base_price}
                    solo-price={n_solo_price}
                    lauf-min={n_lauf_min}
                    {str_allowed_games}
                    {str_no_active_game}
                    {str_extras}"
                );
                println!("{str_ruleset}");
                unwrap!(crate::rules::ruleset::SRuleSet::from_string(&str_ruleset))
            })
            .choose_multiple(&mut rng, 2)
    {
        run_simple_game_loop(
            EPlayerIndex::map_from_fn(|epi| -> Box<dyn TPlayer> {
                Box::new(SPlayerComputer{ai: {
                    if epi<EPlayerIndex::EPI2 {
                        ai::SAi::new_cheating(/*n_rank_rules_samples*/1, /*n_suggest_card_branches*/2)
                    } else {
                        ai::SAi::new_simulating(/*n_rank_rules_samples*/1, /*n_suggest_card_branches*/1, /*n_suggest_card_samples*/1)
                    }
                }})
            }),
            /*n_games*/4,
            ruleset,
            /*fn_print_account_balance*/|_,_| {/* no output */},
        );
    }
}

