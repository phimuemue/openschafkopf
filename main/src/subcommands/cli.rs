use crate::game::*;
use crate::player::{*, playercomputer::*, playerhuman::*};
use crate::primitives::*;
use crate::rules::{
    ruleset::*,
    TActivelyPlayableRulesBoxClone, // TODO improve trait-object behaviour
};
use crate::skui;
use crate::util::*;
use std::sync::mpsc;

pub fn subcommand(str_subcommand: &str) -> clap::App {
    use super::clap_arg;
    clap::SubCommand::with_name(str_subcommand)
        .about("Simulate players to play against")
        .arg(clap_arg("ruleset", "rulesets/default.toml"))
        .arg(clap_arg("ai", "cheating"))
        .arg(clap_arg("numgames", "4"))
}

pub struct SAtTable {
    player: Box<dyn TPlayer>,
    n_money: isize,
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let _tui = skui::STuiGuard::init_ui();
    let (mut aattable, n_stock) = game_loop_cli_internal(
        /*aplayer*/EPlayerIndex::map_from_fn(|epi| -> Box<dyn TPlayer> {
            if EPlayerIndex::EPI1==epi {
                Box::new(SPlayerHuman{ai : super::ai(clapmatches)})
            } else {
                Box::new(SPlayerComputer{ai: super::ai(clapmatches)})
            }
        }),
        /*n_games*/unwrap!(clapmatches.value_of("numgames")).parse::<usize>().unwrap_or(4),
        super::get_ruleset(clapmatches)?,
    );
    aattable.sort_unstable_by_key(|attable| attable.n_money);
    println!("Results:");
    for attable in aattable.iter() {
        println!("{} {}", attable.player.name(), attable.n_money);
    }
    println!("Stock: {}", n_stock);
    Ok(())
}

pub fn game_loop_cli_internal(aplayer: EnumMap<EPlayerIndex, Box<dyn TPlayer>>, n_games: usize, ruleset: SRuleSet) -> ([SAtTable; 4], isize) {
    let mut aattable = aplayer.map_into(|player| SAtTable{player, n_money:0});
    let mut n_stock = 0;
    for _i_game in 0..n_games {
        fn communicate_via_channel<T: std::fmt::Debug>(f: impl FnOnce(mpsc::Sender<T>)) -> T {
            let (txt, rxt) = mpsc::channel::<T>();
            f(txt);
            unwrap!(rxt.recv())
        }
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
        let mut gamepreparations = unwrap!(dealcards.finish());
        while let Some(epi) = gamepreparations.which_player_can_do_something() {
            info!("Asking player {} for game", epi);
            unwrap!(gamepreparations.announce_game(
                epi,
                communicate_via_channel(|txorules| {
                    aattable[epi].player.ask_for_game(
                        epi,
                        gamepreparations.fullhand(epi),
                        &gamepreparations.gameannouncements,
                        &gamepreparations.ruleset.avecrulegroup[epi],
                        stoss_and_doublings(/*vecstoss*/&[], &gamepreparations.doublings),
                        gamepreparations.n_stock,
                        None,
                        txorules
                    );
                }).map(TActivelyPlayableRulesBoxClone::box_clone)
            ));
        }
        info!("Asked players if they want to play. Determining rules");
        let stockorgame = match unwrap!(gamepreparations.finish()) {
            VGamePreparationsFinish::DetermineRules(mut determinerules) => {
                while let Some((epi, vecrulegroup_steigered))=determinerules.which_player_can_do_something() {
                    if let Some(rules) = communicate_via_channel(|txorules| {
                        aattable[epi].player.ask_for_game(
                            epi,
                            determinerules.fullhand(epi),
                            /*gameannouncements*/&SPlayersInRound::new(SStaticEPI0{}),
                            &vecrulegroup_steigered,
                            stoss_and_doublings(/*vecstoss*/&[], &determinerules.doublings),
                            determinerules.n_stock,
                            Some(determinerules.currently_offered_prio()),
                            txorules
                        );
                    }).map(TActivelyPlayableRulesBoxClone::box_clone) {
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
        };
        let gameresult = match stockorgame {
            VStockOrT::OrT(mut game) => {
                while let Some(gameaction)=game.which_player_can_do_something() {
                    if !gameaction.1.is_empty() {
                        if let Some(epi_stoss) = gameaction.1.iter()
                            .find(|epi| {
                                communicate_via_channel(|txb_stoss| {
                                    aattable[**epi].player.ask_for_stoss(
                                        **epi,
                                        &game.doublings,
                                        game.rules.as_ref(),
                                        &game.ahand[**epi],
                                        &game.vecstoss,
                                        game.n_stock,
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
        skui::print_account_balance(&aattable.map(|attable| attable.n_money), n_stock);
        aattable.as_raw_mut().rotate_left(1);
    }
    (aattable.into_raw(), n_stock)
}

#[test]
fn test_game_loop() {
    use rand::prelude::IteratorRandom;
    use crate::ai;
    use crate::player::{
        *,
        playercomputer::*,
    };
    let mut rng = rand::thread_rng();
    use itertools::iproduct;
    for ruleset in
        iproduct!(
            [10, 20].iter(), // n_base_price
            [50, 100].iter(), // n_solo_price
            [2, 3].iter(), // n_lauf_min
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
            ].iter(),
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
            ].iter(),
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
            ].iter()
        )
            .map(|(n_base_price, n_solo_price, n_lauf_min, str_allowed_games, str_no_active_game, str_extras)| {
                let str_ruleset = format!(
                    "base-price={}
                    solo-price={}
                    lauf-min={}
                    {}
                    {}
                    {}",
                    n_base_price, n_solo_price, n_lauf_min, str_allowed_games, str_no_active_game, str_extras
                );
                println!("{}", str_ruleset);
                unwrap!(SRuleSet::from_string(&str_ruleset))
            })
            .choose_multiple(&mut rng, 2)
    {
        game_loop_cli_internal(
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
        );
    }
}
