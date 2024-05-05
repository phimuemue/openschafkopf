pub mod analyze;
pub mod cli;
pub mod hand_stats;
pub mod parse;
pub mod suggest_card;
pub mod webext;
pub mod websocket;
mod handconstraint;
mod common_given_game;

use openschafkopf_util::*;
use openschafkopf_lib::{
    rules::{
        TRules,
        ruleset::{SRuleSet, VStockOrT},
    },
    ai::SAi,
    primitives::card::EKurzLang,
    game::*,
};
use std::io::Read;
use failure::*;
use plain_enum::PlainEnum;

fn clap_arg(str_long: &'static str, str_default: &'static str) -> clap::Arg<'static> {
    clap::Arg::new(str_long)
        .long(str_long)
        .default_value(str_default)
}

mod shared_args {
    pub fn ruleset_arg() -> clap::Arg<'static> {
        super::clap_arg("ruleset", "rulesets/default.toml")
            .help("Path to a ruleset TOML file.")
            .long_help("The TOML ruleset file describes the set of available rules, prices, what to do if no one wants to play, whether stoss and doubling is allowed.")
    }

    pub fn ai_arg() -> clap::Arg<'static> {
        super::clap_arg("ai", "cheating")
            .help("Describes whether AI has access to all players' cards")
            .long_help("Describes whether the AI plays fair or has access to all players' cards.")
            .possible_values(["cheating", "simulating"]) // TODO custom validator?
    }

    pub fn input_files_arg(str_name: &'static str) -> clap::Arg<'static> { // TODO? unify str_name
        clap::Arg::new(str_name)
            .takes_value(true)
            .multiple_occurrences(true)
            .help("Paths to files containing played games")
            .long_help("Paths or glob patterns to files containing played games. Files can either be saved HTML from sauspiel.de or plain text files containing one game per line (each line consisting of the rules, followed by a colon and the cards in the order they have been played).")
    }
}

pub fn get_ruleset(clapmatches: &clap::ArgMatches) -> Result<SRuleSet, Error> {
    SRuleSet::from_file(std::path::Path::new(unwrap!(clapmatches.value_of("ruleset"))))
}

pub fn ai(subcommand_matches: &clap::ArgMatches) -> SAi {
    match unwrap!(subcommand_matches.value_of("ai")) {
        "cheating" => SAi::new_cheating(/*n_rank_rules_samples*/50, /*n_suggest_card_branches*/2),
        "simulating" => 
            SAi::new_simulating(
                /*n_rank_rules_samples*/50,
                /*n_suggest_card_branches*/2,
                /*n_suggest_card_samples*/10,
            ),
        _ => {
            println!("Warning: AI not recognized. Defaulting to 'cheating'");
            SAi::new_cheating(/*n_rank_rules_samples*/50, /*n_suggest_card_branches*/2)
        }
    }
}

pub fn glob_files_or_read_stdin<'str_glob>(
    itstr_glob: impl Iterator<Item=&'str_glob str>,
    mut fn_ok: impl FnMut(Option<std::path::PathBuf>, String, usize),
) -> Result<(), Error> {
    let mut i_ok = 0;
    let mut fn_ok = move |opath, str_ok| {
        fn_ok(opath, str_ok, i_ok);
        i_ok += 1;
    };
    let mut b_from_file = false;
    for str_glob in itstr_glob {
        b_from_file = true;
        for globresult in glob::glob(str_glob)? {
            match globresult {
                Ok(path) => {
                    let str_input = String::from_utf8_lossy(&via_out_param_result(|vecu8|
                        std::fs::File::open(&path)?.read_to_end(vecu8)
                    )?.0).to_string();
                    fn_ok(Some(path), str_input);
                },
                Err(e) => {
                    eprintln!("Error: {:?}. Trying to continue.", e);
                },
            }
        }
    }
    if !b_from_file {
        fn_ok(
            /*opath*/None,
            std::io::read_to_string(std::io::stdin())?,
        );
    }
    Ok(())
}

pub fn gameresult_to_dir<GameAnnouncements, DetermineRules>(
    gameresult: &SGameResultGeneric</*Ruleset*/EKurzLang, GameAnnouncements, DetermineRules>,
) -> std::path::PathBuf {
    let path_to_kurzlang = |ekurzlang| {
        std::path::Path::new(match ekurzlang {
            EKurzLang::Kurz => "kurz",
            EKurzLang::Lang => "lang",
        })
    };
    match &gameresult.stockorgame {
        VStockOrT::Stock(ekurzlang_ruleset) => {
            path_to_kurzlang(*ekurzlang_ruleset)
                .join("stock")
        },
        VStockOrT::OrT(game) => {
            let mut path_gameresult = path_to_kurzlang(game.kurzlang())
                .join(game.rules.to_string());
            if let Some(epi) = game.rules.playerindex() {
                path_gameresult = path_gameresult
                    .join(format!("von {}", epi.to_usize()));
            }
            path_gameresult
        },
    }
}
