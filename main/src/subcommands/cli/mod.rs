mod skui;
mod playerhuman;
use failure::*;

use openschafkopf_lib::{
    game::run::run_simple_game_loop,
    player::{*, playercomputer::*},
    primitives::*,
};
use openschafkopf_util::*;
use plain_enum::PlainEnum;

pub fn subcommand(str_subcommand: &'static str) -> clap::Command<'static> {
    use super::clap_arg;
    use super::shared_args::*;
    clap::Command::new(str_subcommand)
        .about("Play in command line")
        .arg(ruleset_arg())
        .arg(ai_arg())
        .arg(clap_arg("numgames", "4")
            .help("Number of games to play")
        )
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let _tui = skui::STuiGuard::init_ui();
    let (mut aattable, n_stock) = run_simple_game_loop(
        /*aplayer*/EPlayerIndex::map_from_fn(|epi| -> Box<dyn TPlayer> {
            if EPlayerIndex::EPI1==epi {
                Box::new(playerhuman::SPlayerHuman{ai : super::ai(clapmatches)})
            } else {
                Box::new(SPlayerComputer{ai: super::ai(clapmatches)})
            }
        }),
        /*n_games*/unwrap!(clapmatches.value_of("numgames")).parse::<usize>().unwrap_or(4),
        super::get_ruleset(clapmatches)?,
        /*fn_print_account_balance*/skui::print_account_balance,
    );
    aattable.sort_unstable_by_key(|attable| attable.n_money);
    println!("Results:");
    for attable in aattable.iter() {
        println!("{} {}", attable.player.name(), attable.n_money);
    }
    println!("Stock: {}", n_stock);
    Ok(())
}

