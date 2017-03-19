use primitives::*;
use rules::*;
use rules::ruleset::*;
use skui;
use util::*;
use errors::*;

use rand::{self, Rng};

pub type SDoublings = SPlayersInRound<bool>;

pub struct SDealCards<'rules> {
    m_ahand : EnumMap<EPlayerIndex, SHand>,
    m_doublings : SDoublings,
    m_ruleset : &'rules SRuleSet,
}

impl<'rules> SDealCards<'rules> {
    pub fn new(epi_first: EPlayerIndex, ruleset: &SRuleSet) -> SDealCards {
        SDealCards {
            m_ahand : {
                let mut veccard : Vec<_> = SCard::values().into_iter().collect();
                assert!(veccard.len()==32);
                EPlayerIndex::map_from_fn(move |_epi|
                    random_hand(8, &mut veccard)
                )
            },
            m_doublings: SDoublings::new(epi_first),
            m_ruleset: ruleset,
        }
    }

    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        self.m_ruleset.m_oedoublingscope.as_ref().and_then(|_edoublingscope|
            self.m_doublings.current_playerindex()
        )
    }

    pub fn first_hand_for(&self, epi: EPlayerIndex) -> &[SCard] {
        let veccard = self.m_ahand[epi].cards();
        assert_eq!(veccard.len(), 8);
        &veccard[0..veccard.len()/2]
    }

    pub fn announce_doubling(&mut self, epi: EPlayerIndex, b_doubling: bool) -> Result<()> {
        if Some(epi)!=self.which_player_can_do_something() {
            bail!("Wrong player index");
        }
        self.m_doublings.push(b_doubling);
        assert!(0<self.m_doublings.size());
        Ok(())
    }

    pub fn finish_dealing(self, ruleset: &SRuleSet, n_stock: isize) -> SGamePreparations {
        assert!(self.which_player_can_do_something().is_none());
        let epi_first = self.m_doublings.first_playerindex();
        SGamePreparations {
            m_ahand : self.m_ahand,
            m_doublings : self.m_doublings,
            m_ruleset : ruleset,
            m_gameannouncements : SGameAnnouncements::new(epi_first),
            m_n_stock: n_stock,
        }
    }


}

pub type SGameAnnouncements<'rules> = SPlayersInRound<Option<&'rules TActivelyPlayableRules>>;

pub struct SGamePreparations<'rules> {
    pub m_ahand : EnumMap<EPlayerIndex, SHand>,
    m_doublings : SDoublings,
    pub m_ruleset : &'rules SRuleSet,
    pub m_gameannouncements : SGameAnnouncements<'rules>,
    pub m_n_stock : isize,
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

impl<'rules> SGamePreparations<'rules> {
    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        self.m_gameannouncements.current_playerindex()
    }

    pub fn announce_game(&mut self, epi: EPlayerIndex, orules: Option<&'rules TActivelyPlayableRules>) -> Result<()> {
        if Some(epi)!=self.which_player_can_do_something() {
            bail!("Wrong player index");
        }
        if orules.map_or(false, |rules| Some(epi)!=rules.playerindex()) {
            bail!("Only actively playable rules can be announced");
        }
        self.m_gameannouncements.push(orules);
        assert!(0<self.m_gameannouncements.size());
        Ok(())
    }

    pub fn determine_rules(self) -> VStockOrT<SPreGame> {
        // TODO: find sensible way to deal with multiple game announcements (currently, we choose highest priority)
        let create_game = move |ahand, doublings, n_stock, rules| {
            VStockOrT::OrT(SPreGame {
                m_ahand : ahand,
                m_doublings : doublings,
                m_rules : rules,
                m_vecstoss : vec![],
                m_n_stock : n_stock,
            })
        };
        let vecrules_announced : Vec<&TActivelyPlayableRules> = self.m_gameannouncements.into_iter()
            .filter_map(|(_epi, orules)| orules)
            .collect();
        if 0<vecrules_announced.len() {
            let prio_best = vecrules_announced.iter()
                .map(|rules| rules.priority())
                .max()
                .unwrap();
            let rules_actively_played = vecrules_announced.into_iter()
                .find(|rules| rules.priority()==prio_best)
                .unwrap();
            create_game(self.m_ahand, self.m_doublings, self.m_n_stock, rules_actively_played.box_clone())
        } else {
            match self.m_ruleset.m_stockorramsch {
                VStockOrT::OrT(ref rulesramsch) => {
                    create_game(self.m_ahand, self.m_doublings, self.m_n_stock, rulesramsch.box_clone())
                },
                VStockOrT::Stock(n_stock) => {
                    VStockOrT::Stock(match self.m_ruleset.m_oedoublingscope {
                        None | Some(EDoublingScope::Games) => n_stock,
                        Some(EDoublingScope::GamesAndStock) => {
                            n_stock * 2isize.pow(
                                self.m_doublings.iter().filter(|&(_epi, &b_doubling)| b_doubling).count().as_num()
                            )
                        }
                    })
                }
            }
        }
    }
}

pub struct SPreGame {
    pub m_ahand : EnumMap<EPlayerIndex, SHand>,
    pub m_doublings : SDoublings,
    pub m_rules : Box<TRules>,
    pub m_vecstoss : Vec<SStoss>,
    pub m_n_stock : isize,
}

impl SPreGame {
    pub fn which_player_can_do_something(&self) -> Vec<EPlayerIndex> {
        if self.m_vecstoss.len() < 4 {
            EPlayerIndex::values()
                .map(|epi| epi.wrapping_add(self.m_doublings.first_playerindex().to_usize()))
                .filter(|epi| self.m_rules.stoss_allowed(*epi, &self.m_vecstoss, &self.m_ahand[*epi]))
                .collect()
        } else {
            vec![]
        }
    }

    pub fn stoss(&mut self, epi_stoss: EPlayerIndex) -> Result<()> {
        if !self.which_player_can_do_something().into_iter()
            .any(|epi| epi==epi_stoss)
        {
            bail!("Stoss not allowed for specified epi");
        }
        self.m_vecstoss.push(SStoss{m_epi : epi_stoss});
        Ok(())
    }

    pub fn finish(self) -> SGame {
        let epi_first = self.m_doublings.first_playerindex();
        SGame {
            m_ahand : self.m_ahand,
            m_doublings : self.m_doublings,
            m_rules : self.m_rules,
            m_vecstoss : self.m_vecstoss,
            m_n_stock : self.m_n_stock,
            m_vecstich : vec![SStich::new(epi_first)],
        }
    }
}

pub struct SGame {
    pub m_ahand : EnumMap<EPlayerIndex, SHand>,
    pub m_doublings : SDoublings,
    pub m_rules : Box<TRules>,
    pub m_vecstoss : Vec<SStoss>,
    pub m_n_stock : isize,
    pub m_vecstich : Vec<SStich>,
}

impl SGame {
    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        self.current_stich().current_playerindex()
    }

    pub fn current_stich(&self) -> &SStich {
        current_stich(&self.m_vecstich)
    }

    pub fn zugeben(&mut self, card_played: SCard, epi: EPlayerIndex) -> Result<()> {
        // returns the EPlayerIndex of the player who is the next in row to do something
        skui::logln(&format!("Player {} wants to play {}", epi, card_played));
        if Some(epi)!=self.which_player_can_do_something() {
            bail!("Wrong player index");
        }
        if !self.m_ahand[epi].contains(card_played) {
            bail!("card not contained in player's hand");
        }
        {
            let hand = &mut self.m_ahand[epi];
            assert!(self.m_rules.card_is_allowed(&self.m_vecstich, hand, card_played));
            hand.play_card(card_played);
            assert!(!self.m_vecstich.is_empty());
            self.m_vecstich.last_mut().unwrap().push(card_played);
        }
        for epi in EPlayerIndex::values() {
            skui::logln(&format!("Hand {}: {}", epi, self.m_ahand[epi]));
        }
        if 4==self.current_stich().size() {
            if 8==self.m_vecstich.len() { // TODO kurze Karte?
                skui::logln("Game finished.");
                skui::print_vecstich(&self.m_vecstich);
                Ok(())
            } else {
                // TODO: all players should have to acknowledge the current stich in some way
                let epi_last_stich = {
                    let stich = self.current_stich();
                    skui::logln(&format!("Stich: {}", stich));
                    self.m_rules.winner_index(stich)
                };
                skui::logln(&format!("Opening new stich starting at {}", epi_last_stich));
                assert!(self.m_vecstich.is_empty() || 4==self.current_stich().size());
                self.m_vecstich.push(SStich::new(epi_last_stich));
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    pub fn payout(&self) -> SAccountBalance {
        assert!(self.which_player_can_do_something().is_none());
        self.m_rules.payout(
            &SGameFinishedStiche::new(&self.m_vecstich),
            /*n_stoss*/ self.m_vecstoss.len(),
            /*n_doubling*/ self.m_doublings.iter().filter(|&(_epi, &b_doubling)| b_doubling).count(),
            self.m_n_stock,
        )
    }

    pub fn completed_stichs(&self) -> &[SStich] {
        completed_stichs(&self.m_vecstich)
    }
}
