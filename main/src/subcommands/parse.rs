use crate::subcommands::analyze::{analyze_sauspiel_html, analyze_plain}; // TODO move functions to own module
use crate::game::*;
use crate::rules::ruleset::{VStockOrT};
use crate::util::*;
use std::io::Read;
use itertools::Itertools;

pub fn subcommand(str_subcommand: &str) -> clap::App {
    clap::SubCommand::with_name(str_subcommand)
        .about("Parse a game into a simple format")
        .arg(clap::Arg::with_name("file") // TODO? shared function to glob for files
            .required(true)
            .takes_value(true)
            .multiple(true)
        )
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let itstr_file = unwrap!(clapmatches.values_of("file"));
    for str_file in itstr_file {
        for globresult in glob::glob(str_file)? {
            match globresult {
                Ok(path) => {
                    let str_input = &via_out_param_result(|str_html|
                        std::fs::File::open(&path)?.read_to_string(str_html)
                    )?.0;
                    if let Ok(SGameResultGeneric{stockorgame: VStockOrT::OrT(game), ..}) = analyze_sauspiel_html(str_input) {
                        let str_out = format!("{}{}: {}",
                            game.rules.to_string(),
                            if let Some(epi) = game.rules.playerindex() {
                                format!(" von {}", epi)
                            } else {
                                "".into()
                            },
                            game.stichseq.visible_stichs().iter()
                                .flat_map(|stich| stich.iter().map(|(_epi, card)| card))
                                .join(" "),
                        );
                        let game_check = unwrap!(unwrap!(analyze_plain(&str_out).exactly_one()));
                        assert_eq!(game_check.rules.to_string(), game.rules.to_string()); // TODO? better comparison?
                        assert_eq!(game_check.rules.playerindex(), game.rules.playerindex());
                        assert_eq!(game_check.stichseq, game.stichseq);
                        println!("{}", str_out);
                    } else {
                        eprintln!("Nothing found in {:?}: Trying to continue.", path);
                    }
                },
                Err(e) => {
                    eprintln!("Error: {:?}. Trying to continue.", e);
                },
            }
        }
    }
    Ok(())
}
