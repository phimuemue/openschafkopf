#![cfg_attr(feature = "cargo-clippy", allow(
    clippy::blocks_in_if_conditions,
    clippy::just_underscores_and_digits,
    clippy::upper_case_acronyms,
    clippy::type_complexity, // TODO could this be solved via type-complexity-threshold?
    clippy::new_without_default,
    clippy::enum_variant_names,
    clippy::let_unit_value,
    clippy::nonminimal_bool, // TODO? I got this lint, but could not see where the expression could be simplified.
    clippy::clone_on_copy, // TODORUST I think that some types that implement Copy should not implement it (in particular: large arrays)
))]
#![cfg_attr(all(not(debug_assertions), feature="cargo-clippy"), allow(clippy::let_and_return))]

#[macro_use]
mod util;
mod ai;
mod game;
mod game_analysis;
mod player;
mod primitives;
mod rules;
mod subcommands;

use crate::util::*;

fn main() -> Result<(), Error> {
    logging::init_logging("openschafkopf")?; // TODO? split for certain subcommands (e.g. webext)
    macro_rules! subcommands{($(($([$($t:tt)*])? $mod:ident, $str_cmd:expr))*) => {
        let clapmatches = clap::Command::new("schafkopf")
            .arg_required_else_help(true);
            let mut i_subcommand = 0; // TODO(clap) support for subcommand grouping (see https://github.com/clap-rs/clap/issues/1553)
            $(
                i_subcommand += 1;
                $(#[$($t)*])?
                let clapmatches = clapmatches
                    .subcommand(subcommands::$mod::subcommand($str_cmd)
                        .display_order(i_subcommand)
                    );
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
        // play
        ([cfg(feature="cli")] cli, "cli")
        ([cfg(feature="websocket")] websocket, "websocket")
        // analyze
        ([cfg(feature="analyze")] analyze, "analyze")
        ([cfg(feature="suggest-card")] suggest_card, "suggest-card")
        ([cfg(feature="hand-stats")] hand_stats, "hand-stats")
        // misc
        ([cfg(feature="dl")] dl, "dl")
        ([cfg(feature="parse")] parse, "parse")
        ([cfg(feature="webext")] webext, "webext")
    );
    Ok(())
}


