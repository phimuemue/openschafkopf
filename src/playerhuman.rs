use stich::*;
use card::*;
use hand::*;
use player::*;
use gamestate::*;
use rules::*;
use rulesrufspiel::*;

use std::sync::mpsc;
use std::io::{self, Read};

pub struct CPlayerHuman;

impl CPlayer for CPlayerHuman {
    fn take_control(&mut self, gamestate: &SGameState, txcard: mpsc::Sender<CCard>) {
        loop {
            let eplayerindex = gamestate.which_player_can_do_something().unwrap();
            println!("Human player has: {}", gamestate.m_ahand[eplayerindex]);
            let veccard_allowed = gamestate.m_rules.all_allowed_cards(
                &gamestate.m_vecstich,
                &gamestate.m_ahand[eplayerindex]
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
                    txcard.send(veccard_allowed[nIndex]);
                    return;
                }
                Ok(_) => {
                    println!("Error. Number not within suggested bounds.");
                }
                _ => {
                    println!("Error. Input not a number");
                }
            }
        }
    }

    fn ask_for_game(&self, eplayerindex: EPlayerIndex, _: &CHand) -> Option<Box<TRules>> {
        None // TODO: implement this properly
    }
}
