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
mod playerhuman;
mod suspicion;
mod ui;
mod mainwindow;

use game::*;
use mainwindow::*;
use playerhuman::*;
use playercomputer::*;

extern crate gtk;

use std::thread;

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize gtk");
        return;
    }
    main_window();
    gtk::main();

}
