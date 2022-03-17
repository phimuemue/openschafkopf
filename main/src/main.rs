#![cfg_attr(feature = "cargo-clippy", allow(
    clippy::blocks_in_if_conditions,
    clippy::just_underscores_and_digits,
    clippy::upper_case_acronyms,
    clippy::type_complexity, // TODO could this be solved via type-complexity-threshold?
    clippy::new_without_default,
))]

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
    macro_rules! subcommands{($(($mod:ident, $str_cmd:expr))*) => {
        let clapmatches = clap::App::new("schafkopf")
            .setting(clap::AppSettings::ArgRequiredElseHelp)
            $(.subcommand(subcommands::$mod::subcommand($str_cmd)))*
            .get_matches();
        $(
            if let Some(clapmatches_subcommand)=clapmatches.subcommand_matches($str_cmd) {
                return subcommands::$mod::run(clapmatches_subcommand);
            }
        )*
    }}
    subcommands!(
        (cli, "cli")
        (rank_rules, "rank-rules")
        (suggest_card, "suggest-card")
        (parse, "parse")
        (analyze, "analyze")
        (websocket, "websocket")
        (hand_stats, "hand-stats")
        (dl, "dl")
    );
    Ok(())
}


