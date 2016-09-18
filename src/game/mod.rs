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

pub fn random_hand(n_size: usize, veccard : &mut Vec<SCard>) -> SHand {
    assert!(veccard.len()>=n_size);
    SHand::new_from_vec({
        let mut veccard_hand = SHandVector::new();
        for _i in 0..n_size {
            let i_card = rand::thread_rng().gen_range(0, veccard.len());
            veccard_hand.push(veccard.swap_remove(i_card));
        }
        assert_eq!(veccard_hand.len(), n_size);
        veccard_hand
    })
}

pub fn random_hands() -> [SHand; 4] {
    let mut veccard : Vec<_> = SCard::all_values().into_iter().collect();
    assert!(veccard.len()==32);
    create_playerindexmap(move |_eplayerindex|
        random_hand(8, &mut veccard)
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

    pub fn announce_game(&mut self, eplayerindex: EPlayerIndex, orules: Option<&'rules TActivelyPlayableRules>) -> Result<(), &'static str> {
        if Some(eplayerindex)!=self.which_player_can_do_something() {
            return Err("Wrong player index");
        }
        if orules.map_or(false, |rules| Some(eplayerindex)!=rules.playerindex()) {
            return Err("Only actively playable rules can be announced");
        }
        self.m_vecgameannouncement.push(SGameAnnouncement{
            m_eplayerindex : eplayerindex,
            m_orules : orules,
        });
        assert!(!self.m_vecgameannouncement.is_empty());
        Ok(())
    }

    // TODO: extend return value to support stock, etc.
    pub fn determine_rules(self) -> Option<SPreGame<'rules>> {
        // TODO: find sensible way to deal with multiple game announcements (currently, we choose highest priority)
        let eplayerindex_first = self.m_eplayerindex_first;
        let create_game = move |ahand, rules| {
            Some(SPreGame {
                m_ahand : ahand,
                m_rules : rules,
                m_eplayerindex_first : eplayerindex_first,
                m_vecstoss : vec![],
            })
        };
        let vecrules_announced : Vec<&TActivelyPlayableRules> = self.m_vecgameannouncement.into_iter()
            .filter_map(|gameannouncement| gameannouncement.m_orules)
            .collect();
        if 0<vecrules_announced.len() {
            let prio_best = vecrules_announced.iter()
                .map(|rules| rules.priority())
                .max()
                .unwrap();
            let rules_actively_played = vecrules_announced.into_iter()
                .find(|rules| rules.priority()==prio_best)
                .unwrap();
            create_game(self.m_ahand, rules_actively_played.as_rules())
        } else if let Some(ref rulesramsch) = self.m_ruleset.m_orulesramsch {
            create_game(self.m_ahand, rulesramsch.as_ref())
        } else {
            None
        }
    }
}

pub struct SPreGame<'rules> {
    pub m_eplayerindex_first : EPlayerIndex,
    pub m_ahand : [SHand; 4],
    pub m_rules : &'rules TRules,
    pub m_vecstoss : Vec<SStoss>,
}

impl<'rules> SPreGame<'rules> {
    pub fn which_player_can_do_something(&self) -> Vec<EPlayerIndex> {
        if self.m_vecstoss.len() < 4 {
            (0..4)
                .map(|eplayerindex| (eplayerindex + self.m_eplayerindex_first) % 4)
                .filter(|eplayerindex| self.m_rules.stoss_allowed(*eplayerindex, &self.m_vecstoss, &self.m_ahand[*eplayerindex]))
                .collect()
        } else {
            vec![]
        }
    }

    pub fn stoss(&mut self, eplayerindex_stoss: EPlayerIndex) -> Result<(), &'static str> {
        if !self.which_player_can_do_something().into_iter()
            .any(|eplayerindex| eplayerindex==eplayerindex_stoss)
        {
            return Err("Stoss not allowed for specified eplayerindex");
        }
        self.m_vecstoss.push(SStoss{m_eplayerindex : eplayerindex_stoss});
        Ok(())
    }

    // TODO: extend return value to support stock, etc.
    pub fn finish(self) -> SGame<'rules> {
        SGame {
            m_ahand : self.m_ahand,
            m_rules : self.m_rules,
            m_vecstich : vec![SStich::new(self.m_eplayerindex_first)],
            m_vecstoss : self.m_vecstoss,
        }
    }
}

pub struct SGame<'rules> {
    pub m_ahand : [SHand; 4],
    pub m_rules : &'rules TRules,
    pub m_vecstoss : Vec<SStoss>,
    pub m_vecstich : Vec<SStich>,
}

pub struct SGameAnnouncement<'rules> {
    pub m_eplayerindex : EPlayerIndex,
    pub m_orules: Option<&'rules TActivelyPlayableRules>,
}

impl<'rules> SGame<'rules> {
    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        if 8==self.m_vecstich.len() && 4==self.m_vecstich.last().unwrap().size() {
            None
        } else {
            Some(self.m_vecstich.last().unwrap().current_player_index())
        }
    }

    pub fn zugeben(&mut self, card_played: SCard, eplayerindex: EPlayerIndex) -> Result<(), &'static str> {
        // returns the EPlayerIndex of the player who is the next in row to do something
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
        assert!(self.which_player_can_do_something().is_none());
        let an_payout_raw = self.m_rules.payout(&SGameFinishedStiche::new(&self.m_vecstich));
        create_playerindexmap(|eplayerindex| {
            an_payout_raw[eplayerindex] * 2isize.pow(self.m_vecstoss.len() as u32)
        })
    }

    pub fn completed_stichs(&self) -> &[SStich] {
        assert!(self.m_vecstich.last().unwrap().size()<4);
        assert_eq!(self.m_vecstich[0..self.m_vecstich.len()-1].len(), self.m_vecstich.len()-1);
        assert!(self.m_vecstich[0..self.m_vecstich.len()-1].iter().all(|stich| stich.size()==4));
        &self.m_vecstich[0..self.m_vecstich.len()-1]
    }
}
