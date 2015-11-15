use card::*;
use hand::*;
use stich::*;
use rules::*;
use gamestate::*;
use player::*;
use cardvectorparser::*;
use rulesrufspiel::*;
use playercomputer::*;
use playerhuman::*;

use std::sync::mpsc;

pub struct CGame {
    pub m_gamestate : SGameState,
    //m_vecplayer : Vec<Rc<CPlayer>> ,
    m_vecplayer : Vec<Box<CPlayer>>, // TODO: good idea to use Box<CPlayer>, maybe shared_ptr equivalent?
}

impl CGame {
    //fn new_by_random(bShort : bool/*TODO: is it a good idea to have players in CGame?*/) -> CGame; // shall replace DealCards
    pub fn new(vecplayer: Vec<Box<CPlayer>>) -> CGame {
        CGame {
            m_gamestate : SGameState {
                m_ahand : [ // TODO: shuffle cards
                    CHand::new_from_vec(parse_cards("g7 hk ga so gu e9 ho sk")),
                    CHand::new_from_vec(parse_cards("gz g8 ek s8 h7 ez sz s7")),
                    CHand::new_from_vec(parse_cards("s9 g9 ea sa eu hz h9 eo")),
                    CHand::new_from_vec(parse_cards("h8 e8 ha e7 hu gk su go"))
                ],
                m_rules : Box::new(CRulesRufspiel {m_eplayerindex : 0, m_efarbe: efarbeEICHEL} ),
                m_vecstich : Vec::new()
            },
            m_vecplayer : vecplayer
        }
    }

    pub fn run_game(mut self, eplayerindex_first : EPlayerIndex) {

        // prepare
        self.m_gamestate.m_vecstich.clear();
        println!("Starting game");
        let mut veccard_all : Vec<CCard> = Vec::new();
        for efarbe in EFarbe::all_values().iter() {
            for eschlag in ESchlag::all_values().iter() {
                veccard_all.push(CCard::new(*efarbe, *eschlag));
            }
        }
        for hand in self.m_gamestate.m_ahand.iter() {
            print!("{} |", hand);
        }
        println!("");

        // decide which game is played
        println!("Asking players if they want to play");
        let mut vecpaireplayerindexgameannounce : Vec<(EPlayerIndex, Box<TRules>)> = Vec::new();
        for eplayerindex in (eplayerindex_first..eplayerindex_first+4).map(|eplayerindex| eplayerindex%4) {
            if let Some(gameannounce) = self.m_vecplayer[eplayerindex].ask_for_game(
                eplayerindex,
                &self.m_gamestate.m_ahand[eplayerindex]
            ) {
                vecpaireplayerindexgameannounce.push((eplayerindex, gameannounce));
            }
        }
        if vecpaireplayerindexgameannounce.is_empty() {
            return self.run_game(eplayerindex_first + 1); // TODO: just return something like "took not place"
        }

        println!("Asked players if they want to play. Determining rules");
        // TODO: find sensible way to deal with multiple game announcements
        self.m_gamestate.m_rules = vecpaireplayerindexgameannounce.pop().unwrap().1;
        println!("Rules determined. Sorting hands");
        {
            let ref rules = self.m_gamestate.m_rules;
            for hand in self.m_gamestate.m_ahand.iter_mut() {
                hand.sort(|&card_fst, &card_snd| rules.compare_in_stich(card_fst, card_snd));
                println!("{}", hand);
            }
        }

        // starting game
        println!("Beginning first stich");
        println!("Giving control to player {}", eplayerindex_first);
        let (txcard, rxcard) = mpsc::channel();
        let mut eplayerindex_last_stich = eplayerindex_first;
        for /*i_stich*/ _ in 0..8 { // TODO: kurze Karte?
            self.m_gamestate.m_vecstich.push(CStich::new(eplayerindex_last_stich));
            self.notify_game_listeners();
            for eplayerindex in (eplayerindex_last_stich..eplayerindex_last_stich+4).map(|eplayerindex| eplayerindex%4) {
                assert!(eplayerindex< 4); // note: 0<=eplayerindex by type!
                {
                    self.m_vecplayer[eplayerindex].take_control(&self.m_gamestate, txcard.clone());
                }
                {
                    let card_played = rxcard.recv().ok().unwrap();
                    println!("Player {} played {}", eplayerindex, card_played);
                    {
                        let stich = self.m_gamestate.m_vecstich.last_mut().unwrap();
                        assert_eq!(eplayerindex, (stich.first_player_index() + stich.size())%4);
                    }
                    {
                        let ref mut hand = self.m_gamestate.m_ahand[eplayerindex];
                        assert!(self.m_gamestate.m_rules.card_is_allowed(&self.m_gamestate.m_vecstich, hand, card_played));
                        hand.play_card(card_played);
                        self.m_gamestate.m_vecstich.last_mut().unwrap().zugeben(card_played);
                    }
                    self.notify_game_listeners();
                }
            }
            {
                for eplayerindex in 0..4 {
                    println!("Hand {}: {}", eplayerindex, self.m_gamestate.m_ahand[eplayerindex]);
                }
            }
            // TODO: all players should have to acknowledge the current stich in some way
            {
                {
                    let ref stich = self.m_gamestate.m_vecstich.last().unwrap();
                    println!("Stich: {}", stich);
                    eplayerindex_last_stich = self.m_gamestate.m_rules.winner_index(stich);
                    println!("{} made by {}, ({} points)",
                        stich,
                        eplayerindex_last_stich,
                        self.m_gamestate.m_rules.points_stich(stich)
                    );
                }
                self.notify_game_listeners();
            }

        }

        println!("Game finished.");
        for (i_stich, stich) in self.m_gamestate.m_vecstich.iter().enumerate() {
            println!("Stich {}: {}", i_stich, stich);
        }

    }


    fn notify_game_listeners(&self) {
        // TODO: notify game listeners
    }
    
    // fn RegisterPlayer(&mut self, Rc<CPlayer> rcplayer) -> EPlayerIndex {
    //     assert!(self.m_vecplayer.len()<4);
    //     let eplayerindex = self.m_vecplayer.len();
    //     self.m_vecplayer.push(rcplayer);
    //     eplayerindex
    // }
}
