extern crate rand;
extern crate ncurses;
extern crate itertools;
extern crate permutohedron;

mod card;
mod stich;
mod combinatorics;
mod cardvectorparser;
mod hand;
mod rules;
mod rulesrufspiel;
mod rulessolo;
mod gamestate;
mod game;
mod player;
mod playercomputer;
mod playerhuman;
mod suspicion;
mod ruleset;
mod accountbalance;
mod skui;

use game::*;
use std::sync::mpsc;
use card::*;
use stich::*;
use hand::*;
use rules::*;
use accountbalance::SAccountBalance;
use suspicion::SSuspicion;
use rulesrufspiel::CRulesRufspiel;

fn main() {
    let rules = CRulesRufspiel {
        m_eplayerindex : 0,
        m_efarbe : efarbeGRAS,
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
            add_stich(0, "g7 ga hz g9");
            add_stich(2, "s8 ho s7 s9");
            add_stich(1, "h7 hk hu su");
            add_stich(2, "eo go g8 h8");
            add_stich(2, "e9 ek e8 ea");
        }
        vecstich_internal
    };

    let mut n_susp = 0;
    combinatorics::for_each_suspicion(
        &CHand::new_from_vec(cardvectorparser::parse_cards("sa gk sk")),
        &cardvectorparser::parse_cards("eu gz e7 so sz ha h9 ez gu"),
        0, // eplayerindex
        |mut susp| {
            n_susp += 1;
            println!("{} {} {} {}",
                susp.hands()[0],
                susp.hands()[1],
                susp.hands()[2],
                susp.hands()[3]
            );
            susp.compute_successors(&rules);
            let eplayerindex_current_stich = rules.winner_index(vecstich.last().unwrap());
            susp.print_suspicion(8, 8, &rules, &mut vecstich, Some(CStich::new(eplayerindex_current_stich)));
        }
    );
    println!("{}", n_susp);

    return;


    skui::init_ui();
    let mut accountbalance = SAccountBalance::new();
    for i_game in 0..4 { // TODO make number of rounds adjustable
        let mut game = CGame::new();
        skui::logln(&format!("Hand 0 : {}", game.m_gamestate.m_ahand[0]));
        if game.start_game(i_game % 4) {
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
        skui::logln("Account balance:");
        for eplayerindex in 0..4 {
            skui::logln(&format!("Player {}: {}", eplayerindex, accountbalance.get(eplayerindex)));
        }
    }
    skui::end_ui();
}
