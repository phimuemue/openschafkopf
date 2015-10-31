use card::*;
use hand::*;
use stich::*;
use rules::*;
use gamestate::*;
use player::*;

use std::rc::Rc;

struct CGame {
    m_gamestate : SGameState,
    //m_vecplayer : Vec<Rc<CPlayer>> ,
    m_vecplayer : Vec<Box<CPlayer>>, // TODO: good idea to use Box<CPlayer>, maybe shared_ptr equivalent?
}

impl CGame {
    //fn new_by_random(bShort : bool/*TODO: is it a good idea to have players in CGame?*/) -> CGame; // shall replace DealCards
    fn run_game(&mut self, eplayerindex_first : EPlayerIndex) {
        // prepare
        self.m_gamestate.m_vecstich.clear();
        println!("Starting game");
        unimplemented!(); // self.m_gamestate.m_ahand = CGame::deal_cards(); // TODO
        for hand in self.m_gamestate.m_ahand.iter() {
            print!("{} |", hand);
        }
        println!("");

        // decide which game is played
        println!("Asking players if they want to play");
        let mut vecpaireplayerindexgameannounce : Vec<(EPlayerIndex, Box<TRules>)> = Vec::new();
        unimplemented!();
        // TODO: this is full of rust-uncheckable stuff
        // for (eplayerindex, player, hand) in (0..4) // TODO: fill vecpaireplayerindexgameannounce more elegantly
        //     .map(|eplayerindex| (eplayerindex_first + eplayerindex) % 4)
        //     .map(|eplayerindex| (eplayerindex, self.m_vecplayer[eplayerindex], self.m_gamestate.m_ahand[eplayerindex]))
        // {
        //     unimplemented!();
        //     //if let Some(gameannounce) = player.ask_for_game(&hand) {
        //     //    vecpaireplayerindexgameannounce.push((eplayerindex, gameannounce));
        //     //}
        // }
        if vecpaireplayerindexgameannounce.is_empty() {
            return self.run_game(eplayerindex_first + 1); // TODO: just return something like "took not place"
        }

        println!("Asked players if they want to play. Determining rules");
        // TODO: find sensible way to deal with multiple game announcements
        self.m_gamestate.m_rules = vecpaireplayerindexgameannounce.pop().unwrap().1;
        unimplemented!();

        println!("Rules determined. Sorting hands");
        {
            let ref rules = self.m_gamestate.m_rules;
            for hand in self.m_gamestate.m_ahand.iter_mut() {
                hand.sort(|&cardFst, &cardSnd| rules.compare_in_stich(cardFst, cardSnd));
                println!("{}", hand);
            }
        }

        // starting game
        println!("Beginning first stich");
        self.m_gamestate.m_vecstich.push(CStich::new(eplayerindex_first));
        println!("Giving control to player {}", eplayerindex_first);
        self.wait_for_card(eplayerindex_first);
        println!("Game started, control given to player {}", eplayerindex_first);
    }

    fn wait_for_card(&self, eplayerindex: EPlayerIndex) { // was GiveControlTo
        assert!(eplayerindex < 4);
        assert!(0 <= eplayerindex);
        unimplemented!();
        //self.m_vecplayer[eplayerindex].take_control(
        //    self.m_gamestate,
        //    |card| self.zugeben(card)
        //);
    }

    fn zugeben(&mut self, card : CCard) {
        let mut stich = self.m_gamestate.m_vecstich.last_mut().unwrap();
        println!("Player {} played card", (stich.first_player_index() + stich.size())%4);
        let ref mut hand = self.m_gamestate.m_ahand[(stich.first_player_index() + stich.size())%4];
        unimplemented!();
        assert!(self.m_gamestate.m_rules.card_is_allowed(&self.m_gamestate.m_vecstich, hand, card));
        hand.play_card(card);
        stich.zugeben(card);
        self.notify_game_listeners();
        if 4==stich.size() {
            // TODO: wait for all players to acknowledge stich
        }
        else {
            self.wait_for_card((stich.first_player_index() + stich.size())%4);
        }
    }

    fn finish_stich(&mut self) {
        // TODO this should be necessary for all 4 players (all should acknowledge)
        if 8==self.m_gamestate.m_vecstich.len() {
            // TODO: make this sensible, currently this is just an endless loop!
            // TODO: possibly return some game result or similar
            unimplemented!();
        } else {
            let mut i_player_last_stich = 0;
            {
                let ref stich = self.m_gamestate.m_vecstich.last().unwrap();
                let i_player_last_stich_internal = self.m_gamestate.m_rules.winner_index(stich);
                println!("{} made by {}, ({} points)",
                    stich,
                    i_player_last_stich_internal,
                    self.m_gamestate.m_rules.points_stich(stich)
                );
                i_player_last_stich = i_player_last_stich_internal;
            }
            self.m_gamestate.m_vecstich.push(CStich::new(i_player_last_stich)); // open new stich
            self.notify_game_listeners();
        }
    }

    fn notify_game_listeners(&self) {
        unimplemented!();
    }
    
    // fn RegisterPlayer(&mut self, Rc<CPlayer> rcplayer) -> EPlayerIndex {
    //     assert!(self.m_vecplayer.len()<4);
    //     let eplayerindex = self.m_vecplayer.len();
    //     self.m_vecplayer.push(rcplayer);
    //     eplayerindex
    // }
}
