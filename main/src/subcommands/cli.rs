use crate::game::run::run_simple_game_loop;
use crate::player::{*, playercomputer::*, playerhuman::*};
use crate::primitives::*;
use crate::skui;
use crate::util::*;

pub fn subcommand(str_subcommand: &str) -> clap::App {
    use super::clap_arg;
    clap::SubCommand::with_name(str_subcommand)
        .about("Simulate players to play against")
        .arg(clap_arg("ruleset", "rulesets/default.toml"))
        .arg(clap_arg("ai", "cheating"))
        .arg(clap_arg("numgames", "4"))
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let _tui = skui::STuiGuard::init_ui();
    let (mut aattable, n_stock) = run_simple_game_loop(
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
                    "base-price={}
                    solo-price={}
                    lauf-min={}
                    {}
                    {}
                    {}",
                    n_base_price, n_solo_price, n_lauf_min, str_allowed_games, str_no_active_game, str_extras
                );
                println!("{}", str_ruleset);
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
        );
    }
}
