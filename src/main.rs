extern crate rand;
extern crate ncurses;
#[macro_use]
extern crate itertools;
extern crate permutohedron;
extern crate clap;

mod card;
mod stich;
mod combinatorics;
mod cardvectorparser;
mod hand;
mod rules;
mod game;
mod player;
mod ai;
mod accountbalance;
mod skui;

use game::*;
use std::sync::mpsc;
use card::*;
use stich::*;
use hand::*;
use rules::*;
use accountbalance::SAccountBalance;
use rules::rulesrufspiel::CRulesRufspiel;
use std::collections::HashSet;
use rules::ruleset::*;
use ai::*;
use std::path::Path;

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
        .get_matches();
    {
        let rules = CRulesRufspiel {
            m_eplayerindex : 0,
            m_efarbe : EFarbe::Gras,
        };
        let mut vecstich = {
            let mut vecstich_internal = Vec::new();
            {
                let mut add_stich = |eplayerindex, str_stich| {
                    vecstich_internal.push(CStich::new(eplayerindex));
                    for card in cardvectorparser::parse_cards(str_stich).iter().cycle().skip(eplayerindex).take(4) {
                        vecstich_internal.last_mut().unwrap().zugeben(card.clone());
                    }
                };
                add_stich(0, "g7 g8 ga g9");
                add_stich(2, "s8 ho s7 s9");
                add_stich(1, "h7 hk hu su");
                add_stich(2, "eo go hz h8");
                add_stich(2, "e9 ek e8 ea");
                add_stich(3, "sa eu so ha");
            }
            vecstich_internal
        };

        println!("sdf");
        macro_rules! new_hashset {
            () => { {
                let mut settrumpforfarbe = HashSet::new();
                settrumpforfarbe.insert(VTrumpfOrFarbe::Trumpf);
                for efarbe in EFarbe::all_values().iter() {
                    settrumpforfarbe.insert(VTrumpfOrFarbe::Farbe(*efarbe));
                }
                settrumpforfarbe
            } };
        };
        let mut mapeplayerindexsettrumpforfarbe : [HashSet<VTrumpfOrFarbe>; 4] = [
            new_hashset!(), new_hashset!(), new_hashset!(), new_hashset!(),
        ];
        println!("asdlkf");
        for stich in vecstich.iter() {
            println!("{}", stich);
            let trumpforfarbe_first_card = rules.trumpf_or_farbe(stich.first_card());
            for (eplayerindex, card) in stich.indices_and_cards() {
                let trumpforfarbe = rules.trumpf_or_farbe(card);
                if trumpforfarbe_first_card != trumpforfarbe {
                    println!("Removing {} from player {}",
                        match trumpforfarbe_first_card {
                            VTrumpfOrFarbe::Trumpf => "Trumpf".to_string(),
                            VTrumpfOrFarbe::Farbe(efarbe) => format!("{}", efarbe),
                        },
                        eplayerindex
                    );
                    mapeplayerindexsettrumpforfarbe[eplayerindex].remove(&trumpforfarbe_first_card);
                } else {
                    // assert consistency over the whole game
                    assert!(mapeplayerindexsettrumpforfarbe[eplayerindex].contains(&trumpforfarbe_first_card));
                }
            }
        }

        let mut n_susp = 0;
        combinatorics::for_each_suspicion(
            &CHand::new_from_vec(cardvectorparser::parse_cards("gk sk")),
            &cardvectorparser::parse_cards("gz e7 sz h9 ez gu"),
            0, // eplayerindex
            |susp| {
                susp.hands().iter()
                    .enumerate()
                    .all(|(eplayerindex, hand)| {
                        hand.cards().iter()
                            .map(|card| rules.trumpf_or_farbe(card.clone()))
                            .all(|trumpforfarbe| {
                                mapeplayerindexsettrumpforfarbe[eplayerindex].contains(&trumpforfarbe)
                            })
                    })
            },
            |mut susp| {
                n_susp += 1;
                println!("{} {} {} {}",
                    susp.hands()[0],
                    susp.hands()[1],
                    susp.hands()[2],
                    susp.hands()[3]
                );
                susp.compute_successors(&rules, &mut vecstich.clone(), &|_vecstich_complete, vecstich_successor| {
                    assert!(!vecstich_successor.is_empty());
                    random_sample_from_vec(vecstich_successor, 1);
                });
                let eplayerindex_current_stich = rules.winner_index(vecstich.last().unwrap());
                susp.print_suspicion(8, 9, &rules, &mut vecstich, Some(CStich::new(eplayerindex_current_stich)));
            }
        );
        println!("{} suspicions", n_susp);
    }

    let aruleset = read_ruleset(Path::new(clapmatches.value_of("rulesetpath").unwrap()));
    {
        let hand_fixed = random_hand(8, &mut CCard::all_values().into_iter().map(|card| Some(card)).collect());
        let eplayerindex_fixed = 0;

        println!("Hand: {}", hand_fixed);
        for rules in aruleset[eplayerindex_fixed].allowed_rules().iter() 
            .filter(|rules| rules.can_be_played(&hand_fixed))
        {
            let f_payout_avg = ai::rank_rules(&hand_fixed, eplayerindex_fixed, rules, 100);
            println!("{}", rules);
            println!("{}", f_payout_avg);
        }
    }
    //return;


    skui::init_ui();
    let mut accountbalance = SAccountBalance::new();
    for i_game in 0..clapmatches.value_of("numgames").unwrap().parse::<usize>().unwrap_or(4) {
        let gameprep = SGamePreparations::new(&aruleset);
        skui::logln(&format!("Hand 0 : {}", gameprep.m_ahand[0]));
        if let Some(mut game)=gameprep.start_game(i_game % 4) {
            while let Some(eplayerindex)=game.which_player_can_do_something() {
                let (txcard, rxcard) = mpsc::channel::<CCard>();
                game.m_vecplayer[eplayerindex].take_control(
                    &game.m_gamestate,
                    txcard.clone()
                );
                let card_played = rxcard.recv().unwrap();
                game.zugeben(card_played, eplayerindex);
            }
            let an_points = game.points_per_player();
            skui::logln("Results");
            for eplayerindex in 0..4 {
                skui::logln(&format!("Player {}: {} points", eplayerindex, an_points[eplayerindex]));
            }
            accountbalance.apply_payout(&game.payout());
        }
        skui::print_account_balance(&accountbalance);
    }
    skui::end_ui();
    println!("Results: {}", skui::account_balance_string(&accountbalance));
}
