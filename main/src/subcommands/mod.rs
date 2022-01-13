pub mod analyze;
pub mod cli;
pub mod dl;
pub mod hand_stats;
pub mod parse;
pub mod rank_rules;
pub mod suggest_card;
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

fn clap_arg(str_long: &'static str, str_default: &'static str) -> clap::Arg<'static, 'static> {
    clap::Arg::with_name(str_long)
        .long(str_long)
        .default_value(str_default)
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
