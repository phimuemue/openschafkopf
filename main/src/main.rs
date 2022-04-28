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
    macro_rules! subcommands{($(($([$($t:tt)*])? $mod:ident, $str_cmd:expr))*) => {
        let clapmatches = clap::App::new("schafkopf")
            .setting(clap::AppSettings::ArgRequiredElseHelp);
            $(
                $(#[$($t)*])?
                let clapmatches = clapmatches
                    .subcommand(subcommands::$mod::subcommand($str_cmd));
            )*
            let clapmatches = clapmatches.get_matches();
        $(
            $(#[$($t)*])?
            if let Some(clapmatches_subcommand)=clapmatches.subcommand_matches($str_cmd) {
                return subcommands::$mod::run(clapmatches_subcommand);
            }
        )*
    }}
    subcommands!(
        ([cfg(feature="cli")] cli, "cli")
        ([cfg(feature="rank-rules")] rank_rules, "rank-rules")
        ([cfg(feature="suggest-card")] suggest_card, "suggest-card")
        ([cfg(feature="parse")] parse, "parse")
        ([cfg(feature="analyze")] analyze, "analyze")
        ([cfg(feature="websocket")] websocket, "websocket")
        ([cfg(feature="hand-stats")] hand_stats, "hand-stats")
        ([cfg(feature="dl")] dl, "dl")
    );
    Ok(())
}


