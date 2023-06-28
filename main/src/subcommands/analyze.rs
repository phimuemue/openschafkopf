use crate::game_analysis::{*, parser::*};
use crate::game::*;
use crate::util::*;
use crate::rules::ruleset::VStockOrT;

pub fn subcommand(str_subcommand: &'static str) -> clap::Command {
    use super::shared_args::*;
    clap::Command::new(str_subcommand)
        .about("Analyze played games and spot suboptimal decisions")
        .arg(input_files_arg("sauspiel-files"))
        .arg(clap::Arg::new("include-no-findings") // TODO replace this by interactive option in resulting HTML
            .long("include-no-findings")
        )
        .arg(clap::Arg::new("simulate-all-hands")
            .long("simulate-all-hands")
            .help("Perform analysis as if other cards are unknown")
            .long_help("Perform analysis not only for the given distribution of cards, but instead for all possible combinations from the respective player's point of view.")
        )
        .arg(super::clap_arg("max-remaining-cards", if_dbg_else!({"2"}{"4"}))
            .help("Analyze only if a hand contains at most a certain amount of cards")
            .long_help("Perform analysis only if the respective player at the respective point of the game has at most a certain amount of cards left. Can be used to reduce computation time.")
        )
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let mut vecgamewithdesc = Vec::new();
    super::glob_files(
        unwrap!(clapmatches.values_of("sauspiel-files")),
        |path, str_input| {
            println!("Opened {:?}", path);
            let mut b_found = false;
            let mut push_game = |str_description, resgameresult: Result<_, _>| {
                b_found = b_found || resgameresult.is_ok();
                vecgamewithdesc.push(SGameWithDesc{
                    str_description,
                    str_link: format!("file://{}", path.to_string_lossy()),
                    resgameresult,
                });
            };
            if let resgameresult@Ok(_) = analyze_sauspiel_html(&str_input) {
                push_game(
                    path.to_string_lossy().into_owned(),
                    resgameresult.map(|game| game.map(|_|(), |_|(), |_|()))
                )
            } else {
                let mut b_found_plain = false;
                for (i, resgame) in analyze_plain(&str_input)
                    .chain(analyze_netschafkopf(&str_input).into_iter().flatten()
                        .map(|resgameresult| resgameresult.and_then(|gameresult| {
                            match gameresult.stockorgame {
                                VStockOrT::Stock(()) => Err(format_err!("Nothing to analyze.")),
                                VStockOrT::OrT(game) => Ok(game),
                            }
                        }))
                    )
                    .filter(|res| res.is_ok())
                    .enumerate()
                {
                    b_found_plain = true;
                    push_game(
                        format!("{}_{}", path.to_string_lossy(), i),
                        resgame.and_then(|game| game.finish().map_err(|_game| format_err!("Could not game.finish")))
                    )
                }
                if !b_found_plain {
                    push_game(path.to_string_lossy().into_owned(), Err(format_err!("Nothing found in {:?}: Trying to continue.", path)));
                }
            }
            if !b_found {
                eprintln!("Nothing found in {:?}: Trying to continue.", path);
            }
        },
    )?;
    let path_out = analyze_games(
        std::path::Path::new("./analyze"), // TODO make customizable
        /*fn_link*/|str_description: &str| str_description.to_string(),
        vecgamewithdesc,
        /*b_include_no_findings*/clapmatches.is_present("include-no-findings"),
        /*n_max_remaining_cards*/unwrap!(clapmatches.value_of("max-remaining-cards")).parse()?,
        /*b_simulate_all_hands*/clapmatches.is_present("simulate-all-hands"),
    )?;
    println!("Analysis written to {}.", path_out.display());
    Ok(())
}
