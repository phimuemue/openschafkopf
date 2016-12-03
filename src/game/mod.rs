pub mod accountbalance;

use primitives::*;
use rules::*;
use rules::ruleset::*;
use skui;

use rand::{self, Rng};

pub type SDoublings = SPlayersInRound<bool>;

pub struct SDealCards {
    pub m_ahand : SPlayerIndexMap<SHand>,
    pub m_doublings : SDoublings,
}

impl SDealCards {
    pub fn new(eplayerindex_first: EPlayerIndex) -> SDealCards {
        SDealCards {
            m_ahand : random_hands(),
            m_doublings: SDoublings::new(eplayerindex_first),
        }
    }

    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        // TODO make doublings adjustable (possibly within SRuleSet)
        self.m_doublings.current_playerindex()
    }

    pub fn first_hand_for(&self, eplayerindex: EPlayerIndex) -> &[SCard] {
        &self.m_ahand[eplayerindex].cards()[eplayerindex_values()]
    }

    pub fn announce_doubling(&mut self, eplayerindex: EPlayerIndex, b_doubling: bool) -> Result<(), &'static str> {
        if Some(eplayerindex)!=self.which_player_can_do_something() {
            return Err("Wrong player index");
        }
        self.m_doublings.push(b_doubling);
        assert!(0<self.m_doublings.size());
        Ok(())
    }

    pub fn finish_dealing(self, ruleset: &SRuleSet) -> SGamePreparations {
        let eplayerindex_first = self.m_doublings.first_playerindex();
        SGamePreparations {
            m_ahand : self.m_ahand,
            m_doublings : self.m_doublings,
            m_ruleset : ruleset,
            m_gameannouncements : SGameAnnouncements::new(eplayerindex_first),
        }
    }


}

pub type SGameAnnouncements<'rules> = SPlayersInRound<Option<&'rules TActivelyPlayableRules>>;

pub struct SGamePreparations<'rules> {
    pub m_ahand : SPlayerIndexMap<SHand>,
    m_doublings : SDoublings,
    pub m_ruleset : &'rules SRuleSet,
    pub m_gameannouncements : SGameAnnouncements<'rules>,
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

pub fn random_hands() -> SPlayerIndexMap<SHand> {
    let mut veccard : Vec<_> = SCard::values().into_iter().collect();
    assert!(veccard.len()==32);
    create_playerindexmap(move |_eplayerindex|
        random_hand(8, &mut veccard)
    )
}

impl<'rules> SGamePreparations<'rules> {
    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        self.m_gameannouncements.current_playerindex()
    }

    pub fn announce_game(&mut self, eplayerindex: EPlayerIndex, orules: Option<&'rules TActivelyPlayableRules>) -> Result<(), &'static str> {
        if Some(eplayerindex)!=self.which_player_can_do_something() {
            return Err("Wrong player index");
        }
        if orules.map_or(false, |rules| Some(eplayerindex)!=rules.playerindex()) {
            return Err("Only actively playable rules can be announced");
        }
        self.m_gameannouncements.push(orules);
        assert!(0<self.m_gameannouncements.size());
        Ok(())
    }

    pub fn determine_rules(self) -> VStockOrT<SPreGame<'rules>> {
        // TODO: find sensible way to deal with multiple game announcements (currently, we choose highest priority)
        let create_game = move |ahand, doublings, rules| {
            VStockOrT::OrT(SPreGame {
                m_ahand : ahand,
                m_doublings : doublings,
                m_rules : rules,
                m_vecstoss : vec![],
            })
        };
        let vecrules_announced : Vec<&TActivelyPlayableRules> = self.m_gameannouncements.iter()
            .filter_map(|(_eplayerindex, orules)| orules.clone())
            .collect();
        if 0<vecrules_announced.len() {
            let prio_best = vecrules_announced.iter()
                .map(|rules| rules.priority())
                .max()
                .unwrap();
            let rules_actively_played = vecrules_announced.into_iter()
                .find(|rules| rules.priority()==prio_best)
                .unwrap();
            create_game(self.m_ahand, self.m_doublings, rules_actively_played.as_rules())
        } else {
            match self.m_ruleset.m_stockorramsch {
                VStockOrT::OrT(ref rulesramsch) => {
                    create_game(self.m_ahand, self.m_doublings, rulesramsch.as_ref())
                },
                VStockOrT::Stock(n_stock) => {
                    VStockOrT::Stock(n_stock)
                }
            }
        }
    }
}

pub struct SPreGame<'rules> {
    pub m_ahand : SPlayerIndexMap<SHand>,
    pub m_doublings : SDoublings,
    pub m_rules : &'rules TRules,
    pub m_vecstoss : Vec<SStoss>,
}

impl<'rules> SPreGame<'rules> {
    pub fn which_player_can_do_something(&self) -> Vec<EPlayerIndex> {
        if self.m_vecstoss.len() < 4 {
            eplayerindex_values()
                .map(|eplayerindex| (eplayerindex + self.m_doublings.first_playerindex()) % 4)
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

    pub fn finish(self) -> SGame<'rules> {
        let eplayerindex_first = self.m_doublings.first_playerindex();
        SGame {
            m_ahand : self.m_ahand,
            m_doublings : self.m_doublings,
            m_rules : self.m_rules,
            m_vecstoss : self.m_vecstoss,
            m_vecstich : vec![SStich::new(eplayerindex_first)],
        }
    }
}

pub struct SGame<'rules> {
    pub m_ahand : SPlayerIndexMap<SHand>,
    pub m_doublings : SDoublings,
    pub m_rules : &'rules TRules,
    pub m_vecstoss : Vec<SStoss>,
    pub m_vecstich : Vec<SStich>,
}

impl<'rules> SGame<'rules> {
    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        self.current_stich().current_playerindex()
    }

    pub fn current_stich(&self) -> &SStich {
        current_stich(&self.m_vecstich)
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
            assert!(!self.m_vecstich.is_empty());
            self.m_vecstich.last_mut().unwrap().push(card_played);
        }
        for eplayerindex in eplayerindex_values() {
            skui::logln(&format!("Hand {}: {}", eplayerindex, self.m_ahand[eplayerindex]));
        }
        if 4==self.current_stich().size() {
            if 8==self.m_vecstich.len() { // TODO kurze Karte?
                skui::logln("Game finished.");
                skui::print_vecstich(&self.m_vecstich);
                Ok(())
            } else {
                // TODO: all players should have to acknowledge the current stich in some way
                let eplayerindex_last_stich = {
                    let stich = self.current_stich();
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
                assert!(self.m_vecstich.is_empty() || 4==self.current_stich().size());
                self.m_vecstich.push(SStich::new(eplayerindex_last_stich));
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    pub fn payout(&self, n_stock: isize) -> SAccountBalance {
        assert!(self.which_player_can_do_something().is_none());
        self.m_rules.payout(
            &SGameFinishedStiche::new(&self.m_vecstich),
            /*n_stoss*/ self.m_vecstoss.len(),
            /*n_doubling*/ self.m_doublings.iter().filter(|&(_eplayerindex, &b_doubling)| b_doubling).count(),
            n_stock,
        )
    }

    pub fn completed_stichs(&self) -> &[SStich] {
        completed_stichs(&self.m_vecstich)
    }
}
