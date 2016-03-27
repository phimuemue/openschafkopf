use card::*;
use hand::*;
use stich::*;
use rules::*;
use ruleset::*;
use gamestate::*;
use player::*;
use playercomputer::*;
use playerhuman::*;
use skui;

use rand::{self, Rng};

pub struct SGamePreparations<'rules> {
    pub m_ahand : [CHand; 4],
    pub m_vecplayer : Vec<Box<CPlayer>>, // TODO: good idea to have players in here?
    m_aruleset : &'rules [SRuleSet; 4],
}

impl<'rules> SGamePreparations<'rules> {
    pub fn new(aruleset : &'rules [SRuleSet; 4]) -> SGamePreparations<'rules> {
        SGamePreparations {
            m_ahand : {
                let mut veccard : Vec<CCard> = Vec::new();
                // TODO: doable via flat_map?
                for efarbe in EFarbe::all_values().iter() {
                    for eschlag in ESchlag::all_values().iter() {
                        veccard.push(CCard::new(*efarbe, *eschlag));
                    }
                }
                assert!(veccard.len()==32);
                rand::thread_rng().shuffle(&mut veccard);
                let hand_for_player = |eplayerindex| {
                    CHand::new_from_vec(veccard.iter().cloned().skip((eplayerindex as usize)*8).take(8).collect())
                };
                [hand_for_player(0), hand_for_player(1), hand_for_player(2), hand_for_player(3)]
            },
            m_vecplayer : vec![ // TODO: take players in ctor?
                Box::new(CPlayerHuman),
                Box::new(CPlayerComputer),
                Box::new(CPlayerComputer),
                Box::new(CPlayerComputer)
            ],
            m_aruleset : aruleset,
        }
    }

    // TODO: extend return value to support stock, etc.
    pub fn start_game(mut self, eplayerindex_first : EPlayerIndex) -> Option<CGame<'rules>> {
        // prepare
        skui::logln("Preparing game");
        for hand in self.m_ahand.iter() {
            skui::log(&format!("{} |", hand));
        }
        skui::logln("");

        // decide which game is played
        skui::logln("Asking players if they want to play");
        let mut vecgameannouncement : Vec<SGameAnnouncement> = Vec::new();
        for eplayerindex in (eplayerindex_first..eplayerindex_first+4).map(|eplayerindex| eplayerindex%4) {
            let orules = self.m_vecplayer[eplayerindex].ask_for_game(
                &self.m_ahand[eplayerindex],
                &vecgameannouncement,
                &self.m_aruleset[eplayerindex]
            );
            assert!(orules.as_ref().map_or(true, |rules| eplayerindex==rules.playerindex().unwrap()));
            vecgameannouncement.push((eplayerindex, orules));
        }
        skui::logln("Asked players if they want to play. Determining rules");
        // TODO: find sensible way to deal with multiple game announcements
        vecgameannouncement.retain(|&(_eplayerindex, ref orules)| {
            orules.is_some()
        });
        if vecgameannouncement.is_empty() {
            return None;
        }
        let paireplayerindexgameannounce = vecgameannouncement.pop().unwrap();

        skui::logln(&format!(
            "Rules determined ({} plays {}). Sorting hands",
            paireplayerindexgameannounce.0,
            paireplayerindexgameannounce.1.as_ref().unwrap()
        ));
        for hand in self.m_ahand.iter_mut() {
            hand.sort(|&card_fst, &card_snd| paireplayerindexgameannounce.1.as_ref().unwrap().compare_in_stich(card_fst, card_snd).reverse());
            skui::logln(&format!("{}", hand));
        }

        Some(CGame {
            m_gamestate : SGameState {
                m_ahand : self.m_ahand,
                m_rules : paireplayerindexgameannounce.1.unwrap(),
                m_vecstich : vec![CStich::new(eplayerindex_first)],
            },
            m_vecplayer : self.m_vecplayer,
        })
    }
}

pub struct CGame<'rules> {
    pub m_gamestate : SGameState<'rules>,
    pub m_vecplayer : Vec<Box<CPlayer>>, // TODO: good idea to use Box<CPlayer>, maybe shared_ptr equivalent?
}

pub type SGameAnnouncement<'rules> = (EPlayerIndex, Option<&'rules Box<TRules>>);

impl<'rules> CGame<'rules> {

    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        self.m_gamestate.which_player_can_do_something()
    }

    fn new_stich(&mut self, eplayerindex_last_stich: EPlayerIndex) {
        skui::logln(&format!("Opening new stich starting at {}", eplayerindex_last_stich));
        assert!(self.m_gamestate.m_vecstich.is_empty() || 4==self.m_gamestate.m_vecstich.last().unwrap().size());
        self.m_gamestate.m_vecstich.push(CStich::new(eplayerindex_last_stich));
        self.notify_game_listeners();
    }

    pub fn zugeben(&mut self, card_played: CCard, eplayerindex: EPlayerIndex) -> EPlayerIndex { // TODO: should invalid inputs be indicated by return value?
        // returns the EPlayerIndex of the player who is the next in row to do something
        // TODO: how to cope with finished game?
        skui::logln(&format!("Player {} wants to play {}", eplayerindex, card_played));
        {
            let eplayerindex_privileged = self.which_player_can_do_something().unwrap();
            assert_eq!(eplayerindex, eplayerindex_privileged);
            assert!(self.m_gamestate.m_ahand[eplayerindex].contains(card_played));
        }
        {
            let ref mut hand = self.m_gamestate.m_ahand[eplayerindex];
            assert!(self.m_gamestate.m_rules.card_is_allowed(&self.m_gamestate.m_vecstich, hand, card_played));
            hand.play_card(card_played);
            self.m_gamestate.m_vecstich.last_mut().unwrap().zugeben(card_played);
        }
        for eplayerindex in 0..4 {
            skui::logln(&format!("Hand {}: {}", eplayerindex, self.m_gamestate.m_ahand[eplayerindex]));
        }
        if 4==self.m_gamestate.m_vecstich.last().unwrap().size() {
            if 8==self.m_gamestate.m_vecstich.len() { // TODO kurze Karte?
                skui::logln("Game finished.");
                skui::print_vecstich(&self.m_gamestate.m_vecstich);
                self.notify_game_listeners();
                (self.m_gamestate.m_vecstich.first().unwrap().first_player_index() + 1) % 4 // for next game
            } else {
                // TODO: all players should have to acknowledge the current stich in some way
                let eplayerindex_last_stich = {
                    let stich = self.m_gamestate.m_vecstich.last().unwrap();
                    skui::logln(&format!("Stich: {}", stich));
                    let eplayerindex_last_stich = self.m_gamestate.m_rules.winner_index(stich);
                    skui::logln(&format!("{} made by {}, ({} points)",
                        stich,
                        eplayerindex_last_stich,
                        self.m_gamestate.m_rules.points_stich(stich)
                    ));
                    eplayerindex_last_stich
                };
                self.new_stich(eplayerindex_last_stich);
                self.notify_game_listeners();
                eplayerindex_last_stich
            }
        } else {
            self.notify_game_listeners();
            (eplayerindex + 1) % 4
        }
    }

    pub fn points_per_player(&self) -> [isize; 4] {
        self.m_gamestate.m_rules.points_per_player(&self.m_gamestate.m_vecstich)
    }

    fn notify_game_listeners(&self) {
        // TODO: notify game listeners
    }

    pub fn payout(&self) -> [isize; 4] {
        self.m_gamestate.m_rules.payout(&self.m_gamestate.m_vecstich)
    }
}
