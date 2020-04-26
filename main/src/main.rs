#![cfg_attr(feature = "cargo-clippy", allow(clippy::block_in_if_condition_stmt))]
#![deny(bare_trait_objects)]

#[macro_use]
mod util;
mod ai;
mod game;
mod game_analysis;
mod player;
mod primitives;
mod rules;
mod sauspiel;
mod skui;
mod subcommands;

use crate::ai::*;
use crate::player::{playercomputer::*, playerhuman::*, *};
use crate::primitives::*;
use crate::rules::ruleset::*;
use crate::util::*;
use std::path::Path;

fn main() -> Result<(), Error> {
    openschafkopf_logging::init_logging()?;
    let clap_arg = |str_long, str_default| {
        clap::Arg::with_name(str_long)
            .long(str_long)
            .default_value(str_default)
    };
    // TODO clean up command line arguments and possibly avoid repetitions
    let clapmatches = clap::App::new("schafkopf")
        .subcommand(clap::SubCommand::with_name("cli")
            .about("Simulate players to play against")
            .arg(clap_arg("ruleset", "rulesets/default.toml"))
            .arg(clap_arg("ai", "cheating"))
            .arg(clap_arg("numgames", "4"))
        )
        .subcommand(clap::SubCommand::with_name("rank-rules")
            .about("Estimate strength of own hand")
            .arg(clap_arg("ruleset", "rulesets/default.toml"))
            .arg(clap_arg("ai", "cheating"))
            .arg(clap_arg("hand", ""))
            .arg(clap_arg("position", "0"))
        )
        .subcommand(clap::SubCommand::with_name("suggest-card")
            .about("Suggest a card to play given the game so far")
            .arg(clap::Arg::with_name("first_player_index")
                .long("first")
                .required(true)
                .takes_value(true)
            )
            .arg(clap::Arg::with_name("rules")
                .long("rules")
                .required(true)
                .takes_value(true)
            )
            .arg(clap::Arg::with_name("hand")
                .long("hand")
                .required(true)
                .takes_value(true)
            )
            .arg(clap::Arg::with_name("cards_on_table")
                .long("cards-on-table")
                .required(true)
                .takes_value(true)
            )
        )
        .subcommand(clap::SubCommand::with_name("analyze")
            .about("Analyze played games and spot suboptimal decisions")
            .arg(clap::Arg::with_name("sauspiel-files")
                 .required(true)
                 .takes_value(true)
                 .multiple(true)
            )
        )
        .get_matches();
    if let Some(subcommand_matches_analyze)=clapmatches.subcommand_matches("analyze") {
        if let Some(itstr_sauspiel_html_file) = subcommand_matches_analyze.values_of("sauspiel-files") {
            return subcommands::analyze::analyze(
                &std::path::Path::new("./analyze"),
                itstr_sauspiel_html_file,
            );
        }
    }
    let ai = |subcommand_matches: &clap::ArgMatches| {
        match debug_verify!(subcommand_matches.value_of("ai")).unwrap() {
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
    };
    fn get_ruleset(subcommand_matches: &clap::ArgMatches) -> Result<SRuleSet, Error> {
        SRuleSet::from_file(Path::new(debug_verify!(subcommand_matches.value_of("ruleset")).unwrap()))
    }
    fn str_to_hand(str_hand: &str) -> Result<SHand, Error> {
        Ok(SHand::new_from_vec(cardvector::parse_cards(str_hand).ok_or_else(||format_err!("Could not parse hand."))?))
    }
    if let Some(subcommand_matches)=clapmatches.subcommand_matches("rank-rules") {
        let ruleset = get_ruleset(subcommand_matches)?;
        let str_hand = subcommand_matches.value_of("hand").ok_or_else(||format_err!("No hand given as parameter."))?;
        let hand = str_to_hand(&str_hand)?;
        let hand = Some(hand).filter(|hand| hand.cards().len()==ruleset.ekurzlang.cards_per_player()).ok_or_else(||format_err!("Could not convert hand to a full hand of cards"))?;
        use clap::value_t;
        subcommands::rank_rules::rank_rules(
            &ruleset,
            SFullHand::new(&hand, ruleset.ekurzlang),
            /*epi_rank*/value_t!(subcommand_matches.value_of("position"), EPlayerIndex).unwrap_or(EPlayerIndex::EPI0),
            &ai(subcommand_matches),
        );
        return Ok(())
    }
    if let Some(subcommand_matches)=clapmatches.subcommand_matches("suggest-card") {
        return subcommands::suggest_card::suggest_card(
            &debug_verify!(subcommand_matches.value_of("first_player_index")).unwrap(),
            &debug_verify!(subcommand_matches.value_of("rules")).unwrap(),
            &str_to_hand(&debug_verify!(subcommand_matches.value_of("hand")).unwrap())?,
            &cardvector::parse_cards::<Vec<_>>(
                &debug_verify!(subcommand_matches.value_of("cards_on_table")).unwrap(),
            ).ok_or_else(||format_err!("Could not parse played cards"))?,
        )
    }
    if let Some(subcommand_matches)=clapmatches.subcommand_matches("cli") {
        subcommands::cli::game_loop_cli(
            &EPlayerIndex::map_from_fn(|epi| -> Box<dyn TPlayer> {
                if EPlayerIndex::EPI1==epi {
                    Box::new(SPlayerHuman{ai : ai(subcommand_matches)})
                } else {
                    Box::new(SPlayerComputer{ai: ai(subcommand_matches)})
                }
            }),
            /*n_games*/ debug_verify!(subcommand_matches.value_of("numgames")).unwrap().parse::<usize>().unwrap_or(4),
            &get_ruleset(subcommand_matches)?,
        );
        return Ok(())
    }
    Ok(())
}


