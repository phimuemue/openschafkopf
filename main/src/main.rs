#![allow(
    clippy::blocks_in_conditions,
    clippy::clone_on_copy, // TODORUST I think that some types that implement Copy should not implement it (in particular: large arrays)
)]

mod subcommands;

use openschafkopf_util::*;
use failure::*;

fn main() -> Result<(), Error> {
    logging::init_logging("openschafkopf")?; // TODO? split for certain subcommands (e.g. webext)
    macro_rules! subcommands{($(($mod:ident, $str_cmd:expr))*) => {
        let clapmatches = clap::Command::new("schafkopf")
            .arg_required_else_help(true);
            let mut i_subcommand = 0; // TODO(clap) support for subcommand grouping (see https://github.com/clap-rs/clap/issues/1553)
            $(
                i_subcommand += 1;
                let clapmatches = clapmatches
                    .subcommand(subcommands::$mod::subcommand($str_cmd)
                        .display_order(i_subcommand)
                    );
            )*
            let clapmatches = clapmatches.get_matches();
        $(
            if let Some(clapmatches_subcommand)=clapmatches.subcommand_matches($str_cmd) {
                return subcommands::$mod::run(clapmatches_subcommand);
            }
        )*
    }}
    subcommands!(
        // play
        (cli, "cli")
        (websocket, "websocket")
        // analyze
        (analyze, "analyze")
        (suggest_card, "suggest-card")
        (hand_stats, "hand-stats")
        // misc
        (parse, "parse")
        (webext, "webext")
    );
    Ok(())
}


