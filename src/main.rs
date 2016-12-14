extern crate rand;
extern crate ncurses;
#[macro_use]
extern crate itertools;
extern crate permutohedron;
#[macro_use]
extern crate clap;
extern crate arrayvec;
extern crate crossbeam;

#[macro_use]
mod util;
mod primitives;
mod rules;
mod game;
mod player;
mod ai;
mod skui;

use game::*;
use std::sync::mpsc;
use primitives::*;
use rules::ruleset::*;
use rules::wrappers::*;
use ai::*;
use std::path::Path;
use player::*;
use player::playerhuman::*;
use player::playercomputer::*;

fn main() {
    let clapmatches = clap::App::new("schafkopf")
        .arg(clap::Arg::with_name("rulesetpath")
            .long("ruleset")
            .default_value(".schafkopfruleset")
        )
        .arg(clap::Arg::with_name("numgames")
            .long("numgames")
            .default_value("4")
        )
        .arg(clap::Arg::with_name("ai")
            .long("ai")
            .default_value("cheating")
        )
        .subcommand(clap::SubCommand::with_name("rank-rules")
            .arg(clap::Arg::with_name("hand")
                .long("hand")
                .default_value("")
            )
            .arg(clap::Arg::with_name("pos")
                 .long("position")
                 .default_value("0")
            )
        )
        .get_matches();

    let ai = || {
        match clapmatches.value_of("ai").unwrap().as_ref() {
            "cheating" => Box::new(ai::SAiCheating{}) as Box<TAi>,
            "simulating" => 
                Box::new(ai::SAiSimulating::new(
                    /*n_suggest_card_branches*/2,
                    /*n_suggest_card_samples*/10,
                )) as Box<TAi>,
            _ => {
                println!("Warning: AI not recognized. Defaulting to 'cheating'");
                Box::new(ai::SAiCheating{}) as Box<TAi>
            }
        }
    };

    let ruleset = SRuleSet::from_file(Path::new(clapmatches.value_of("rulesetpath").unwrap()));

    if let Some(subcommand_matches)=clapmatches.subcommand_matches("rank-rules") {
        if let Some(str_hand) = subcommand_matches.value_of("hand") {
            if let Some(hand_fixed) = util::cardvectorparser::parse_cards(str_hand).map(|veccard| SHand::new_from_vec(veccard)) {
                let eplayerindex_rank = value_t!(subcommand_matches.value_of("pos"), EPlayerIndex).unwrap_or(0);
                println!("Hand: {}", hand_fixed);
                for rules in allowed_rules(&ruleset.m_avecrulegroup[eplayerindex_rank]).iter() 
                    .filter(|rules| rules.can_be_played(&SFullHand::new(&hand_fixed)))
                {
                    println!("{}: {}",
                        rules,
                        ai().rank_rules(
                            &SFullHand::new(&hand_fixed),
                            0,
                            eplayerindex_rank,
                            rules.as_rules().clone(),
                            /*n_stock*/0, // assume no stock in subcommand rank-rules
                            100
                        )
                    );
                }
            } else {
                println!("Could not convert \"{}\" to cards.", str_hand);
            }
        }
        return;
    }

    skui::init_ui();
    let accountbalance = game_loop(
        &vec![
            Box::new(SPlayerHuman{m_ai : ai()}),
            Box::new(SPlayerComputer::new(ai(), /*n_samples_per_rules*/50)),
            Box::new(SPlayerComputer::new(ai(), /*n_samples_per_rules*/50)),
            Box::new(SPlayerComputer::new(ai(), /*n_samples_per_rules*/50))
        ],
        /*n_games*/ clapmatches.value_of("numgames").unwrap().parse::<usize>().unwrap_or(4),
        &ruleset,
    );
    skui::end_ui();
    println!("Results: {}", skui::account_balance_string(&accountbalance));
}

fn game_loop(vecplayer: &Vec<Box<TPlayer>>, n_games: usize, ruleset: &SRuleSet) -> SAccountBalance {
    let mut accountbalance = SAccountBalance::new(create_playerindexmap(|_eplayerindex| 0), 0);
    for i_game in 0..n_games {
        let mut dealcards = SDealCards::new(/*eplayerindex_first*/i_game % 4);
        while let Some(eplayerindex) = dealcards.which_player_can_do_something() {
            let (txb_doubling, rxb_doubling) = mpsc::channel::<bool>();
            vecplayer[eplayerindex].ask_for_doubling(
                dealcards.first_hand_for(eplayerindex),
                txb_doubling.clone(),
            );
            dealcards.announce_doubling(eplayerindex, rxb_doubling.recv().unwrap()).unwrap();
        }
        let mut gamepreparations = dealcards.finish_dealing(&ruleset, accountbalance.get_stock());
        while let Some(eplayerindex) = gamepreparations.which_player_can_do_something() {
            skui::logln(&format!("Asking player {} for game", eplayerindex));
            let (txorules, rxorules) = mpsc::channel::<Option<_>>();
            vecplayer[eplayerindex].ask_for_game(
                &SFullHand::new(&gamepreparations.m_ahand[eplayerindex]),
                &gamepreparations.m_gameannouncements,
                &gamepreparations.m_ruleset.m_avecrulegroup[eplayerindex],
                gamepreparations.m_n_stock,
                txorules.clone()
            );
            gamepreparations.announce_game(eplayerindex, rxorules.recv().unwrap()).unwrap();
        }
        skui::logln("Asked players if they want to play. Determining rules");
        match gamepreparations.determine_rules() {
            VStockOrT::OrT(mut pregame) => {
                while let Some(eplayerindex_stoss) = pregame.which_player_can_do_something().into_iter()
                    .find(|eplayerindex| {
                        let (txb_stoss, rxb_stoss) = mpsc::channel::<bool>();
                        vecplayer[*eplayerindex].ask_for_stoss(
                            *eplayerindex,
                            &pregame.m_doublings,
                            pregame.m_rules,
                            &pregame.m_ahand[*eplayerindex],
                            &pregame.m_vecstoss,
                            pregame.m_n_stock,
                            txb_stoss,
                        );
                        rxb_stoss.recv().unwrap()
                    })
                {
                    pregame.stoss(eplayerindex_stoss).unwrap();
                }
                let mut game = pregame.finish();
                while let Some(eplayerindex)=game.which_player_can_do_something() {
                    let (txcard, rxcard) = mpsc::channel::<SCard>();
                    vecplayer[eplayerindex].ask_for_card(
                        &game,
                        txcard.clone()
                    );
                    game.zugeben(rxcard.recv().unwrap(), eplayerindex).unwrap();
                }
                accountbalance.apply_payout(&game.payout());
            },
            VStockOrT::Stock(n_stock) => {
                // TODO Rules must we respect doublings?
                accountbalance.apply_payout(&SAccountBalance::new(
                    create_playerindexmap(|_eplayerindex| -n_stock),
                    4*n_stock,
                ));
            }
        }
        skui::print_account_balance(&accountbalance);
    }
    accountbalance
}

#[test]
fn test_game_loop() {
    game_loop(
        &vec![
            Box::new(SPlayerComputer::new(Box::new(ai::SAiCheating{}), /*n_samples_per_rules*/1)),
            Box::new(SPlayerComputer::new(Box::new(ai::SAiCheating{}), /*n_samples_per_rules*/1)),
            Box::new(SPlayerComputer::new(Box::new(ai::SAiSimulating::new(/*n_suggest_card_branches*/1, /*n_suggest_card_samples*/1)), /*n_samples_per_rules*/1)),
            Box::new(SPlayerComputer::new(Box::new(ai::SAiSimulating::new(/*n_suggest_card_branches*/1, /*n_suggest_card_samples*/1)), /*n_samples_per_rules*/1)),
        ],
        /*n_games*/4,
        &SRuleSet::from_strings(
            ["rufspiel", "solo", "ramsch", "wenz"].iter().map(|str| str.to_string())
        ),
    );
}
