pub mod analyze;
pub mod cli;
pub mod hand_stats;
pub mod parse;
pub mod suggest_card;
pub mod webext;
mod handconstraint;
mod common_given_game;

use openschafkopf_util::*;
use openschafkopf_lib::{
    rules::{SDisplayRules, TRulesPlayerIndex, ruleset::VStockOrT},
    ai::SAi,
    primitives::card::EKurzLang,
    game::*,
};
use std::io::Read;
use failure::*;
use plain_enum::PlainEnum;

mod shared_args {
    pub fn glob_files_arg() -> clap::Arg<'static> {
        clap::Arg::new("file")
            .takes_value(true)
            .multiple_occurrences(true)
            .help("Paths to files containing played games")
            .long_help("Paths or glob patterns to files containing played games. Files can either be saved HTML from sauspiel.de or plain text files containing one game per line (each line consisting of the rules, followed by a colon and the cards in the order they have been played).")
    }
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

pub fn glob_files_or_read_stdin(
    clapmatches: &clap::ArgMatches,
    mut fn_ok: impl FnMut(Option<std::path::PathBuf>, String, usize),
) -> Result<(), Error> {
    let mut i_ok = 0;
    let mut fn_ok = move |opath, str_ok| {
        fn_ok(opath, str_ok, i_ok);
        i_ok += 1;
    };
    let mut b_from_file = false;
    for str_glob in clapmatches.values_of("file").into_iter().flatten() {
        b_from_file = true;
        for globresult in glob::glob(str_glob)? {
            if let Ok(path) = verify_or_println!(globresult) {
                let str_input = String::from_utf8_lossy(&via_out_param_result(|vecu8|
                    std::fs::File::open(&path)?.read_to_end(vecu8)
                )?.0).to_string();
                fn_ok(Some(path), str_input);
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
                .join(SDisplayRules::new(&game.rules, /*b_include_playerindex*/false).to_string());
            if let Some(epi) = game.rules.playerindex() {
                path_gameresult = path_gameresult
                    .join(format!("von {}", epi.to_usize()));
            }
            path_gameresult
        },
    }
}
