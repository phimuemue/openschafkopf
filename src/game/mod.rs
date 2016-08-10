pub mod accountbalance;
pub use self::accountbalance::*;

use primitives::*;
use rules::*;
use rules::ruleset::*;
use skui;

use rand::{self, Rng};

pub struct SGamePreparations<'rules> {
    pub m_ahand : [SHand; 4],
    pub m_ruleset : &'rules SRuleSet,
    m_eplayerindex_first : EPlayerIndex,
    pub m_vecgameannouncement : Vec<SGameAnnouncement<'rules>>,
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
    pub fn new(ruleset : &'rules SRuleSet, eplayerindex_first: EPlayerIndex) -> SGamePreparations<'rules> {
        SGamePreparations {
            m_ahand : {
                let ahand = random_hands();
                skui::logln("Preparing game");
                for hand in ahand.iter() {
                    skui::log(&format!("{} |", hand));
                }
                skui::logln("");
                ahand
            },
            m_ruleset : ruleset,
            m_eplayerindex_first : eplayerindex_first,
            m_vecgameannouncement : Vec::new(),
        }
    }

    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        if self.m_vecgameannouncement.len() == 4 {
            return None;
        } else {
            return Some((self.m_eplayerindex_first + self.m_vecgameannouncement.len()) % 4);
        }
    }

    pub fn announce_game(&mut self, eplayerindex: EPlayerIndex, orules: Option<&'rules TRules>) { // TODO return value: Result<(), Err> or similar
        assert_eq!(eplayerindex, self.which_player_can_do_something().unwrap());
        assert!(orules.as_ref().map_or(true, |rules| eplayerindex==rules.playerindex().unwrap()));
        self.m_vecgameannouncement.push(SGameAnnouncement{
            m_eplayerindex : eplayerindex,
            m_opairrulespriority : orules.map(|rules| (
                rules,
                0 // priority, TODO determine priority
            )),
        });
        assert!(!self.m_vecgameannouncement.is_empty());
    }

    // TODO: extend return value to support stock, etc.
    pub fn determine_rules<'players>(self) -> Option<SGame<'rules>> {
        // TODO: find sensible way to deal with multiple game announcements (currently, we choose highest priority)
        let orules_actively_played = self.m_vecgameannouncement.iter()
            .map(|gameannouncement| gameannouncement.m_opairrulespriority)
            .max_by_key(|opairrulespriority| opairrulespriority.map(|(_orules, priority)| priority)) 
            .unwrap()
            .map(|(rules, _priority)| {
                assert!(rules.playerindex().is_some());
                rules
            });
        let eplayerindex_first = self.m_eplayerindex_first;
        let create_game = move |ahand, rules| {
            Some(SGame {
                m_ahand : ahand,
                m_rules : rules,
                m_vecstich : vec![SStich::new(eplayerindex_first)],
            })
        };
        if let Some(rules) = orules_actively_played {
            create_game(self.m_ahand, rules)
        } else if let Some(ref rulesramsch) = self.m_ruleset.m_orulesramsch {
            create_game(self.m_ahand, rulesramsch.as_ref())
        } else {
            None
        }
    }
}

pub struct SGame<'rules> {
    pub m_ahand : [SHand; 4],
    pub m_rules : &'rules TRules,
    pub m_vecstich : Vec<SStich>,
}

pub type SGameAnnouncementPriority = isize;

pub struct SGameAnnouncement<'rules> {
    pub m_eplayerindex : EPlayerIndex,
    pub m_opairrulespriority : Option<(&'rules TRules, SGameAnnouncementPriority)>,
}

impl<'rules> SGame<'rules> {
    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        if 8==self.m_vecstich.len() && 4==self.m_vecstich.last().unwrap().size() {
            None
        } else {
            Some(
                (self.m_vecstich.last().unwrap().first_player_index() + self.m_vecstich.last().unwrap().size() ) % 4
            )
        }
    }

    pub fn zugeben(&mut self, card_played: SCard, eplayerindex: EPlayerIndex) -> Result<(), &'static str> {
        // returns the EPlayerIndex of the player who is the next in row to do something
        // TODO: how to cope with finished game?
        skui::logln(&format!("Player {} wants to play {}", eplayerindex, card_played));
        if Some(eplayerindex)!=self.which_player_can_do_something() {
            return Err("Wrong player index");
        }
        if !self.m_ahand[eplayerindex].contains(card_played) {
            return Err("card not contained in player's hand");
        }
        {
            let ref mut hand = self.m_ahand[eplayerindex];
            assert!(self.m_rules.card_is_allowed(&self.m_vecstich, hand, card_played));
            hand.play_card(card_played);
            self.m_vecstich.last_mut().unwrap().zugeben(card_played);
        }
        for eplayerindex in 0..4 {
            skui::logln(&format!("Hand {}: {}", eplayerindex, self.m_ahand[eplayerindex]));
        }
        if 4==self.m_vecstich.last().unwrap().size() {
            if 8==self.m_vecstich.len() { // TODO kurze Karte?
                skui::logln("Game finished.");
                skui::print_vecstich(&self.m_vecstich);
                Ok(())
            } else {
                // TODO: all players should have to acknowledge the current stich in some way
                let eplayerindex_last_stich = {
                    let stich = self.m_vecstich.last().unwrap();
                    skui::logln(&format!("Stich: {}", stich));
                    let eplayerindex_last_stich = self.m_rules.winner_index(stich);
                    skui::logln(&format!("{} made by {}, ({} points)",
                        stich,
                        eplayerindex_last_stich,
                        self.m_rules.points_stich(stich)
                    ));
                    eplayerindex_last_stich
                };
                skui::logln(&format!("Opening new stich starting at {}", eplayerindex_last_stich));
                assert!(self.m_vecstich.is_empty() || 4==self.m_vecstich.last().unwrap().size());
                self.m_vecstich.push(SStich::new(eplayerindex_last_stich));
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    pub fn points_per_player(&self, eplayerindex: EPlayerIndex) -> isize {
        self.m_rules.points_per_player(&self.m_vecstich, eplayerindex)
    }

    pub fn payout(&self) -> [isize; 4] {
        self.m_rules.payout(&self.m_vecstich)
    }
}
