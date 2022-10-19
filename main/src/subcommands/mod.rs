pub mod analyze;
pub mod cli;
pub mod dl;
pub mod hand_stats;
pub mod parse;
pub mod rank_rules;
pub mod suggest_card;
pub mod webext;
pub mod websocket;
mod handconstraint;
mod common_given_game;

use crate::util::*;
use crate::rules::ruleset::{SRuleSet, VStockOrT};
use crate::ai::SAi;
use crate::primitives::hand::SHand;
use crate::primitives::card::EKurzLang;
use crate::game::*;
use std::io::Read;

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

    pub fn rules_arg() -> clap::Arg<'static> {
        clap::Arg::new("rules")
            .long("rules")
            .takes_value(true)
            .required(false)
            .help("Rules as plain text")
            .long_help("Rules, given in plain text. The program tries to be lenient in the input format, so that all of the following should be accepted: \"gras wenz von 1\", \"farbwenz gras von 1\", \"BlauWenz von 1\". Players are numbere from 0 to 3, where 0 is the player to open the first stich (1, 2, 3 follow accordingly).")
    }

    pub fn ai_arg() -> clap::Arg<'static> {
        super::clap_arg("ai", "cheating")
            .help("Describes whether AI has access to all players' cards")
            .long_help("Describes whether the AI plays fair or has access to all players' cards.")
            .possible_values(["cheating", "simulating"]) // TODO custom validator?
    }

    pub fn input_files_arg(str_name: &'static str) -> clap::Arg<'static> { // TODO? unify str_name
        clap::Arg::new(str_name)
            .required(true)
            .takes_value(true)
            .multiple_occurrences(true)
            .help("Paths to files containing played games")
            .long_help("Paths or glob patterns to files containing played games. Files can either be saved HTML from sauspiel.de or plain text files containing one game per line (each line consisting of the rules, followed by a colon and the cards in the order they have been played).")
    }
}

pub fn get_ruleset(clapmatches: &clap::ArgMatches) -> Result<SRuleSet, Error> {
    SRuleSet::from_file(std::path::Path::new(unwrap!(clapmatches.value_of("ruleset"))))
}

pub fn get_rules(clapmatches: &clap::ArgMatches) -> Option<Result<Box<dyn crate::rules::TRules>, Error>> {
    clapmatches.value_of("rules").map(crate::rules::parser::parse_rule_description_simple)
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

pub fn str_to_hand(str_hand: &str) -> Result<SHand, Error> {
    Ok(SHand::new_from_vec(crate::primitives::cardvector::parse_cards(str_hand).ok_or_else(||format_err!("Could not parse hand."))?))
}

pub fn glob_files<'str_glob>(
    itstr_glob: impl Iterator<Item=&'str_glob str>,
    mut fn_ok: impl FnMut(std::path::PathBuf, String),
) -> Result<(), Error> {
    for str_glob in itstr_glob {
        for globresult in glob::glob(str_glob)? {
            match globresult {
                Ok(path) => {
                    let str_input = via_out_param_result(|str_html|
                        std::fs::File::open(&path)?.read_to_string(str_html)
                    )?.0;
                    fn_ok(path, str_input);
                },
                Err(e) => {
                    eprintln!("Error: {:?}. Trying to continue.", e);
                },
            }
        }
    }
    Ok(())
}

pub fn gameresult_to_dir<Ruleset, GameAnnouncements, DetermineRules>(
    gameresult: &SGameResultGeneric<Ruleset, GameAnnouncements, DetermineRules>,
) -> std::path::PathBuf {
    let path_dst = std::path::PathBuf::new();
    match &gameresult.stockorgame {
        VStockOrT::Stock(()) => path_dst.join("stock"),
        VStockOrT::OrT(game) => {
            let mut path_gameresult = path_dst
                .join(match game.kurzlang() {
                    EKurzLang::Kurz => "kurz",
                    EKurzLang::Lang => "lang",
                })
                .join(game.rules.to_string());
            if let Some(epi) = game.rules.playerindex() {
                path_gameresult = path_gameresult
                    .join(&format!("von {}", epi.to_usize()));
            }
            path_gameresult
        },
    }
}
