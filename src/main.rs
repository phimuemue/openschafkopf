#![cfg_attr(feature="cargo-clippy", allow(block_in_if_condition_stmt))]

extern crate rand;
extern crate ncurses;
#[macro_use]
extern crate itertools;
extern crate permutohedron;
#[macro_use]
extern crate clap;
extern crate arrayvec;
extern crate crossbeam;
#[macro_use]
extern crate failure;
extern crate as_num;
#[macro_use]
extern crate plain_enum;
#[macro_use]
extern crate derive_new;
extern crate toml;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate chrono;

#[macro_use]
mod util;
mod primitives;
mod rules;
mod game;
mod player;
mod ai;
mod skui;

use game::*;
use primitives::*;
use rules::{
    TActivelyPlayableRules, // TODO improve trait-object behaviour
    ruleset::*,
};
use ai::*;
use std::{
    path::Path,
    sync::mpsc,
};
use player::{
    *,
    playerhuman::*,
    playercomputer::*,
};
use util::*;

fn main() {
    env_logger::init();
    let clap_arg = |str_long, str_default| {
        clap::Arg::with_name(str_long)
            .long(str_long)
            .default_value(str_default)
    };
    // TODO clean up command line arguments and possibly avoid repetitions
    let clapmatches = clap::App::new("schafkopf")
        .subcommand(clap::SubCommand::with_name("cli")
            .arg(clap_arg("ruleset", "ruleset_default.toml"))
            .arg(clap_arg("ai", "cheating"))
            .arg(clap_arg("numgames", "4"))
        )
        .subcommand(clap::SubCommand::with_name("rank-rules")
            .arg(clap_arg("ruleset", "ruleset_default.toml"))
            .arg(clap_arg("ai", "cheating"))
            .arg(clap_arg("hand", ""))
            .arg(clap_arg("position", "0"))
        )
        .get_matches();
    let ai = |subcommand_matches: &clap::ArgMatches| {
        match verify!(subcommand_matches.value_of("ai")).unwrap() {
            "cheating" => Box::new(ai::SAiCheating::new(/*n_rank_rules_samples*/50)) as Box<TAi>,
            "simulating" => 
                Box::new(ai::SAiSimulating::new(
                    /*n_suggest_card_branches*/2,
                    /*n_suggest_card_samples*/10,
                    /*n_rank_rules_samples*/50,
                )) as Box<TAi>,
            _ => {
                println!("Warning: AI not recognized. Defaulting to 'cheating'");
                Box::new(ai::SAiCheating::new(/*n_rank_rules_samples*/50)) as Box<TAi>
            }
        }
    };
    if let Some(subcommand_matches)=clapmatches.subcommand_matches("rank-rules") {
        if let Ok(ruleset) =SRuleSet::from_file(Path::new(verify!(subcommand_matches.value_of("ruleset")).unwrap())) {
            if let Some(str_hand) = subcommand_matches.value_of("hand") {
                if let Some(hand_fixed) = cardvector::parse_cards(str_hand).map(SHand::new_from_vec) {
                    let epi_rank = value_t!(subcommand_matches.value_of("position"), EPlayerIndex).unwrap_or(EPlayerIndex::EPI0);
                    println!("Hand: {}", hand_fixed);
                    let hand_fixed = SFullHand::new(&hand_fixed, ruleset.ekurzlang); // TODO panics if incorrect size => error handling
                    for rules in allowed_rules(&ruleset.avecrulegroup[epi_rank], hand_fixed)
                        .filter_map(|orules| orules) // do not rank None
                    {
                        println!("{}: {}",
                            rules,
                            ai(subcommand_matches).rank_rules(
                                hand_fixed,
                                EPlayerIndex::EPI0,
                                epi_rank,
                                rules.upcast(),
                                /*n_stock*/0, // assume no stock in subcommand rank-rules
                            )
                        );
                    }
                } else {
                    println!("Could not convert \"{}\" to cards.", str_hand);
                }
            }
        }
    }
    if let Some(subcommand_matches)=clapmatches.subcommand_matches("cli") {
        if let Ok(ruleset) =SRuleSet::from_file(Path::new(verify!(subcommand_matches.value_of("ruleset")).unwrap())) {
            skui::init_ui();
            let accountbalance = game_loop_cli(
                &EPlayerIndex::map_from_fn(|epi| -> Box<TPlayer> {
                    if EPlayerIndex::EPI1==epi {
                        Box::new(SPlayerHuman{ai : ai(subcommand_matches)})
                    } else {
                        Box::new(SPlayerComputer{ai: ai(subcommand_matches)})
                    }
                }),
                /*n_games*/ verify!(subcommand_matches.value_of("numgames")).unwrap().parse::<usize>().unwrap_or(4),
                &ruleset,
            );
            println!("Results: {}", skui::account_balance_string(&accountbalance));
            skui::end_ui();
        }
    }
}

fn communicate_via_channel<T, Func>(f: Func) -> T
    where Func: FnOnce(mpsc::Sender<T>) -> (),
          T: std::fmt::Debug,
{
    let (txt, rxt) = mpsc::channel::<T>();
    f(txt.clone());
    verify!(rxt.recv()).unwrap()
}

fn game_loop_cli(aplayer: &EnumMap<EPlayerIndex, Box<TPlayer>>, n_games: usize, ruleset: &SRuleSet) -> SAccountBalance {
    let mut accountbalance = SAccountBalance::new(EPlayerIndex::map_from_fn(|_epi| 0), 0);
    for i_game in 0..n_games {
        let mut dealcards = SDealCards::new(/*epi_first*/EPlayerIndex::wrapped_from_usize(i_game), ruleset, accountbalance.get_stock());
        while let Some(epi) = dealcards.which_player_can_do_something() {
            let b_doubling = communicate_via_channel(|txb_doubling| {
                aplayer[epi].ask_for_doubling(
                    dealcards.first_hand_for(epi),
                    txb_doubling
                );
            });
            verify!(dealcards.announce_doubling(epi, b_doubling)).unwrap();
        }
        let mut gamepreparations = verify!(dealcards.finish()).unwrap();
        while let Some(epi) = gamepreparations.which_player_can_do_something() {
            info!("Asking player {} for game", epi);
            let orules = communicate_via_channel(|txorules| {
                aplayer[epi].ask_for_game(
                    epi,
                    gamepreparations.fullhand(epi),
                    &gamepreparations.gameannouncements,
                    &gamepreparations.ruleset.avecrulegroup[epi],
                    gamepreparations.n_stock,
                    None,
                    txorules
                );
            });
            verify!(gamepreparations.announce_game(epi, orules.map(|rules| TActivelyPlayableRules::box_clone(rules)))).unwrap();
        }
        info!("Asked players if they want to play. Determining rules");
        let stockorgame = match verify!(gamepreparations.finish()).unwrap() {
            VGamePreparationsFinish::DetermineRules(mut determinerules) => {
                while let Some((epi, vecrulegroup_steigered))=determinerules.which_player_can_do_something() {
                    let orules = communicate_via_channel(|txorules| {
                        aplayer[epi].ask_for_game(
                            epi,
                            determinerules.fullhand(epi),
                            /*gameannouncements*/&SPlayersInRound::new(determinerules.doublings.first_playerindex()),
                            &vecrulegroup_steigered,
                            determinerules.n_stock,
                            Some(determinerules.currently_offered_prio()),
                            txorules
                        );
                    });
                    if let Some(rules) = orules.map(|rules| TActivelyPlayableRules::box_clone(rules)) {
                        verify!(determinerules.announce_game(epi, rules)).unwrap();
                    } else {
                        verify!(determinerules.resign(epi)).unwrap();
                    }
                }
                VStockOrT::OrT(verify!(determinerules.finish()).unwrap())
            },
            VGamePreparationsFinish::DirectGame(game) => {
                VStockOrT::OrT(game)
            },
            VGamePreparationsFinish::Stock(n_stock) => {
                VStockOrT::Stock(n_stock)
            }
        };
        match stockorgame {
            VStockOrT::OrT(mut game) => {
                while let Some(gameaction)=game.which_player_can_do_something() {
                    || -> () { // TODO get rid of this closure
                        if !gameaction.1.is_empty() {
                            if let Some(epi_stoss) = gameaction.1.iter()
                                .find(|epi| {
                                    communicate_via_channel(|txb_stoss| {
                                        aplayer[**epi].ask_for_stoss(
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
                                verify!(game.stoss(*epi_stoss)).unwrap();
                                return;
                            }
                        }
                        let card = communicate_via_channel(|txcard| {
                            aplayer[gameaction.0].ask_for_card(
                                &game,
                                txcard.clone()
                            );
                        });
                        verify!(game.zugeben(card, gameaction.0)).unwrap();
                    }();
                }
                accountbalance.apply_payout(&verify!(game.finish()).unwrap().accountbalance);
            },
            VStockOrT::Stock(n_stock) => {
                accountbalance.apply_payout(&SAccountBalance::new(
                    EPlayerIndex::map_from_fn(|_epi| -n_stock),
                    4*n_stock,
                ));
            }
        }
        skui::print_account_balance(&accountbalance);
    }
    accountbalance
}

#[test]
fn test_game_loop() {
    let mut rng = rand::thread_rng();
    for ruleset in verify!(rand::seq::sample_iter(
        &mut rng,
        iproduct!(
            [10, 20].into_iter(), // n_base_price
            [50, 100].into_iter(), // n_solo_price
            [2, 3].into_iter(), // n_lauf_min
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
            ].into_iter(),
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
            ].into_iter(),
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
            ].into_iter()
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
                verify!(SRuleSet::from_string(&str_ruleset)).unwrap()
            }),
            1,
        )).unwrap()
    {
        game_loop_cli(
            &EPlayerIndex::map_from_fn(|epi| -> Box<TPlayer> {
                Box::new(SPlayerComputer{ai: {
                    if epi<EPlayerIndex::EPI2 {
                        Box::new(ai::SAiCheating::new(/*n_rank_rules_samples*/1))
                    } else {
                        Box::new(ai::SAiSimulating::new(/*n_suggest_card_branches*/1, /*n_suggest_card_samples*/1, /*n_samples_per_rules*/1))
                    }
                }})
            }),
            /*n_games*/4,
            &ruleset,
        );
    }
}
