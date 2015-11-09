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

use game::*;

fn main() {
    let mut game = CGame::new();
    println!("Hand 0 : {}", game.m_gamestate.m_ahand[0]);
    game.run_game(0);
}
