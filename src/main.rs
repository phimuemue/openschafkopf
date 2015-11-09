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

use card::CCard;
use card::EFarbe::*;
use card::ESchlag::*;
use hand::*;
use cardvectorparser::*;
use gamestate::*;
use game::*;

fn main() {
    let mut stich = stich::CStich::new(2);
    stich.zugeben(CCard::new(efarbeEICHEL, eschlagU));
    stich.zugeben(CCard::new(efarbeGRAS, eschlag7));
    for (i_player, card) in stich.indices_and_cards() {
        println!("{} hat {}", i_player, card);
    }
    println!("{}", stich);
    let hand = CHand::new_from_vec(
        vec!(
            CCard::new(efarbeEICHEL, eschlagU),
            CCard::new(efarbeHERZ, eschlagK),
            CCard::new(efarbeSCHELLN, eschlag7),
        )
    );
    println!("{}", hand);
    let hand2 = hand.new_from_hand(CCard::new(efarbeHERZ, eschlagK));
    println!("{}", hand2);

    let mut game = CGame::new();
    println!("Hand 0 : {}", game.m_gamestate.m_ahand[0]);
    game.run_game(0);
}
