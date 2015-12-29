mod card;
mod stich;
mod combinatorics;
mod cardvectorparser;
mod hand;
mod rules;
mod rulesrufspiel;
mod gamestate;
mod game;
mod player;
mod playercomputer;
mod suspicion;

use game::*;
use std::io::{self, Read};
use std::sync::mpsc;
use card::CCard;

fn main() {
    let mut game = CGame::new();
    println!("Hand 0 : {}", game.m_gamestate.m_ahand[0]);
    let eplayerindex_first = game.start_game(0);
    while let Some(eplayerindex)=game.which_player_can_do_something() {
        if 0==eplayerindex {
            // TODO: factor this out into CPlayerHuman
            println!("Player {} has: {}", eplayerindex, game.m_gamestate.m_ahand[eplayerindex]);
            let veccard_allowed = game.m_gamestate.m_rules.all_allowed_cards(
                &game.m_gamestate.m_vecstich,
                &game.m_gamestate.m_ahand[eplayerindex]
            );
            println!(
                "Please choose a card (0 to {})",
                veccard_allowed.len()-1,
            );
            let mut str_index = String::new();
            if let Err(e) = (io::stdin().read_line(&mut str_index)) {
                return;
            }
            let mut str_index = str_index.trim();
            match str_index.parse::<usize>() {
                Ok(nIndex) if nIndex < veccard_allowed.len() => {
                    game.zugeben(veccard_allowed[nIndex], eplayerindex);
                }
                Ok(_) => {
                    println!("Error. Number not within suggested bounds.");
                }
                _ => {
                    println!("Error. Input not a number");
                }
            }
        } else {
            let (txcard, rxcard) = mpsc::channel::<CCard>();
            game.m_vecplayer[eplayerindex].take_control(
                &game.m_gamestate,
                txcard.clone()
            );
            let card_played = rxcard.recv().unwrap();
            println!("Ja genau");
            game.zugeben(card_played, eplayerindex);
        }

    }
}
