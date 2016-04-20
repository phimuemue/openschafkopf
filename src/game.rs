use card::*;
use hand::*;
use stich::*;
use rules::*;
use rules::ruleset::*;
use player::*;
use player::playercomputer::*;
use player::playerhuman::*;
use skui;

use rand::{self, Rng};

pub struct SGamePreparations<'rules> {
    pub m_ahand : [SHand; 4],
    pub m_vecplayer : Vec<Box<TPlayer>>, // TODO: good idea to have players in here?
    m_aruleset : &'rules [SRuleSet; 4],
}

pub fn random_hand(n_size: usize, vecocard : &mut Vec<Option<SCard>>) -> SHand {
    let n_card_total = 32;
    assert_eq!(vecocard.len(), n_card_total);
    assert!(vecocard.iter().filter(|ocard| ocard.is_some()).count()>=n_size);
    SHand::new_from_vec({
        let mut veccard = SHandVector::new();
        for _i in 0..n_size {
            let mut i_card = rand::thread_rng().gen_range(0, n_card_total);
            while vecocard[i_card].is_none() {
                i_card = rand::thread_rng().gen_range(0, n_card_total);
            }
            veccard.push(vecocard[i_card].unwrap());
            vecocard[i_card] = None;
        }
        assert_eq!(veccard.len(), n_size);
        veccard
    })
}

pub fn random_hands() -> [SHand; 4] {
    let mut vecocard : Vec<Option<SCard>> = SCard::all_values().into_iter().map(|card| Some(card)).collect();
    assert!(vecocard.len()==32);
    create_playerindexmap(move |_eplayerindex|
        random_hand(8, &mut vecocard)
    )
}

impl<'rules> SGamePreparations<'rules> {
    pub fn new(aruleset : &'rules [SRuleSet; 4]) -> SGamePreparations<'rules> {
        SGamePreparations {
            m_ahand : random_hands(),
            m_vecplayer : vec![ // TODO: take players in ctor?
                Box::new(SPlayerHuman),
                Box::new(SPlayerComputer),
                Box::new(SPlayerComputer),
                Box::new(SPlayerComputer)
            ],
            m_aruleset : aruleset,
        }
    }

    // TODO: extend return value to support stock, etc.
    pub fn start_game(mut self, eplayerindex_first : EPlayerIndex) -> Option<SGame<'rules>> {
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
            vecgameannouncement.push(SGameAnnouncement{
                m_eplayerindex : eplayerindex, 
                m_opairrulespriority : orules.map(|rules| (
                    rules,
                    0 // priority, TODO determine priority
                )),
            });
        }
        skui::logln("Asked players if they want to play. Determining rules");
        // TODO: find sensible way to deal with multiple game announcements (currently, we choose highest priority)
        assert!(!vecgameannouncement.is_empty());
        vecgameannouncement.iter()
            .map(|gameannouncement| gameannouncement.m_opairrulespriority)
            .max_by_key(|opairrulespriority| opairrulespriority.map(|(_orules, priority)| priority)) 
            .unwrap()
            .map(move |(rules, _priority)| {
                assert!(rules.playerindex().is_some());
                skui::logln(&format!(
                    "Rules determined ({} plays {}). Sorting hands",
                    rules.playerindex().unwrap(),
                    rules
                ));
                for hand in self.m_ahand.iter_mut() {
                    hand.sort(|&card_fst, &card_snd| rules.compare_in_stich(card_fst, card_snd).reverse());
                    skui::logln(&format!("{}", hand));
                }
                SGame {
                    m_gamestate : SGameState {
                        m_ahand : self.m_ahand,
                        m_rules : rules,
                        m_vecstich : vec![SStich::new(eplayerindex_first)],
                    },
                    m_vecplayer : self.m_vecplayer,
                }
            })
    }
}

pub struct SGameState<'rules> {
    pub m_ahand : [SHand; 4],
    pub m_rules : &'rules TRules,
    pub m_vecstich : Vec<SStich>,
}

impl<'rules> SGameState<'rules> {
    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        if 8==self.m_vecstich.len() && 4==self.m_vecstich.last().unwrap().size() {
            None
        } else {
            Some(
                (self.m_vecstich.last().unwrap().first_player_index() + self.m_vecstich.last().unwrap().size() ) % 4
            )
        }
    }
}

pub struct SGame<'rules> {
    pub m_gamestate : SGameState<'rules>,
    pub m_vecplayer : Vec<Box<TPlayer>>, // TODO: good idea to use Box<TPlayer>, maybe shared_ptr equivalent?
}

pub type SGameAnnouncementPriority = isize;

pub struct SGameAnnouncement<'rules> {
    pub m_eplayerindex : EPlayerIndex,
    pub m_opairrulespriority : Option<(&'rules TRules, SGameAnnouncementPriority)>,
}

impl<'rules> SGame<'rules> {

    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        self.m_gamestate.which_player_can_do_something()
    }

    fn new_stich(&mut self, eplayerindex_last_stich: EPlayerIndex) {
        skui::logln(&format!("Opening new stich starting at {}", eplayerindex_last_stich));
        assert!(self.m_gamestate.m_vecstich.is_empty() || 4==self.m_gamestate.m_vecstich.last().unwrap().size());
        self.m_gamestate.m_vecstich.push(SStich::new(eplayerindex_last_stich));
        self.notify_game_listeners();
    }

    pub fn zugeben(&mut self, card_played: SCard, eplayerindex: EPlayerIndex) -> EPlayerIndex { // TODO: should invalid inputs be indicated by return value?
        // returns the EPlayerIndex of the player who is the next in row to do something
        // TODO: how to cope with finished game?
        skui::logln(&format!("Player {} wants to play {}", eplayerindex, card_played));
        assert_eq!(eplayerindex, self.which_player_can_do_something().unwrap());
        assert!(self.m_gamestate.m_ahand[eplayerindex].contains(card_played));
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
