#![cfg_attr(feature = "cargo-clippy", allow(clippy::blocks_in_if_conditions, clippy::just_underscores_and_digits))]
#![deny(bare_trait_objects)]

#[macro_use]
mod util;
mod ai;
mod game;
mod game_analysis;
mod player;
mod primitives;
mod rules;
mod skui;
mod subcommands;

use crate::primitives::*;
use crate::util::*;

fn main() -> Result<(), Error> {
    openschafkopf_logging::init_logging()?;
    // TODO clean up command line arguments and possibly avoid repetitions
    let clapmatches = clap::App::new("schafkopf")
        .subcommand(subcommands::cli::subcommand("cli"))
        .subcommand(subcommands::rank_rules::subcommand("rank-rules"))
        .subcommand(subcommands::suggest_card::subcommand("suggest-card"))
        .subcommand(subcommands::analyze::subcommand("analyze"))
        .subcommand(subcommands::websocket::subcommand("websocket"))
        .get_matches();
    if let Some(clapmatches_websocket)=clapmatches.subcommand_matches("websocket") {
        return subcommands::websocket::run(clapmatches_websocket);
    }
    if let Some(clapmatches_analyze)=clapmatches.subcommand_matches("analyze") {
        return subcommands::analyze::analyze(clapmatches_analyze);
    }
    if let Some(clapmatches_rank_rules)=clapmatches.subcommand_matches("rank-rules") {
        return subcommands::rank_rules::rank_rules(clapmatches_rank_rules);
    }
    if let Some(clapmatches_suggest_card)=clapmatches.subcommand_matches("suggest-card") {
        return subcommands::suggest_card::suggest_card(clapmatches_suggest_card);
    }
    if let Some(clapmatches_cli)=clapmatches.subcommand_matches("cli") {
        return subcommands::cli::game_loop_cli(clapmatches_cli);
    }
    Ok(())
}


