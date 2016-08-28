extern crate rand;
extern crate ncurses;
#[macro_use]
extern crate itertools;
extern crate permutohedron;
extern crate clap;
extern crate arrayvec;

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
        )
        .get_matches();
    {
        println!(
            "{} suspicions", 
            ai::handiterators::all_possible_hands(
                &[(0, "g7 g8 ga g9"), (2, "s8 ho s7 s9"), (1, "h7 hk hu su"), (2, "eo go hz h8"), (2, "e9 ek e8 ea"), (3, "sa eu so ha")].iter()
                    .map(|&(eplayerindex, str_stich)| {
                        let mut stich = SStich::new(eplayerindex);
                        for card in util::cardvectorparser::parse_cards::<Vec<_>>(str_stich).unwrap().iter().cycle().skip(eplayerindex).take(4) {
                            stich.zugeben(card.clone());
                        }
                        stich
                    })
                    .collect::<Vec<_>>(),
                SHand::new_from_vec(util::cardvectorparser::parse_cards("gk sk").unwrap()),
                0, // eplayerindex_fixed
            ).count()
        );
    }

    let ai : Box<TAi> = {
        match clapmatches.value_of("ai").unwrap().as_ref() {
            "cheating" => Box::new(ai::SAiCheating{}),
            "simulating" => Box::new(ai::SAiSimulating{}),
            _ => {
                println!("Warning: AI not recognized. Defaulting to 'cheating'");
                Box::new(ai::SAiCheating{})
            }
        }
    };

    let ruleset = read_ruleset(Path::new(clapmatches.value_of("rulesetpath").unwrap()));

    if let Some(subcommand_matches)=clapmatches.subcommand_matches("rank-rules") {
        if let Some(str_hand) = subcommand_matches.value_of("hand") {
            if let Some(hand_fixed) = util::cardvectorparser::parse_cards(str_hand).map(|veccard| SHand::new_from_vec(veccard)) {
                let eplayerindex_fixed = 0;
                println!("Hand: {}", hand_fixed);
                for rules in allowed_rules(&ruleset.m_avecrulegroup[eplayerindex_fixed]).iter() 
                    .filter(|rules| rules.can_be_played(&hand_fixed))
                {
                    println!("{}: {}",
                        rules,
                        ai.rank_rules(&hand_fixed, eplayerindex_fixed, rules.clone(), 100)
                    );
                }
            } else {
                println!("Could not convert \"{}\" to cards.", str_hand);
            }
        }
        return;
    }

    skui::init_ui();
    let mut vecplayer : Vec<Box<TPlayer>> = vec![
        Box::new(SPlayerHuman{m_ai : ai.as_ref()}),
        Box::new(SPlayerComputer{m_ai : ai.as_ref()}),
        Box::new(SPlayerComputer{m_ai : ai.as_ref()}),
        Box::new(SPlayerComputer{m_ai : ai.as_ref()})
    ];
    let mut accountbalance = SAccountBalance::new();
    for i_game in 0..clapmatches.value_of("numgames").unwrap().parse::<usize>().unwrap_or(4) {
        let mut gamepreparations = SGamePreparations::new(
            &ruleset,
            /*eplayerindex_first*/i_game % 4,
        );
        while let Some(eplayerindex) = gamepreparations.which_player_can_do_something() {
            skui::logln(&format!("Asking player {} for game", eplayerindex));
            let orules = vecplayer[eplayerindex].ask_for_game(
                &gamepreparations.m_ahand[eplayerindex],
                &gamepreparations.m_vecgameannouncement,
                &gamepreparations.m_ruleset.m_avecrulegroup[eplayerindex]
            );
            gamepreparations.announce_game(eplayerindex, orules).unwrap();
        }
        skui::logln("Asked players if they want to play. Determining rules");
        if let Some(mut game) = gamepreparations.determine_rules() {
            while let Some(eplayerindex)=game.which_player_can_do_something() {
                let (txcard, rxcard) = mpsc::channel::<SCard>();
                vecplayer[eplayerindex].take_control(
                    &game,
                    txcard.clone()
                );
                let card_played = rxcard.recv().unwrap();
                game.zugeben(card_played, eplayerindex).unwrap();
            }
            skui::logln("Results");
            for eplayerindex in 0..4 {
                skui::logln(&format!("Player {}: {} points", eplayerindex, game.points_per_player(eplayerindex)));
            }
            accountbalance.apply_payout(&game.payout());
        }
        skui::print_account_balance(&accountbalance);
    }
    skui::end_ui();
    println!("Results: {}", skui::account_balance_string(&accountbalance));
}
