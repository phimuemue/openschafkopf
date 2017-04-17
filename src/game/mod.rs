use primitives::*;
use rules::*;
use rules::ruleset::*;
use skui;
use util::*;
use errors::*;

use rand::{self, Rng};
use std::mem;

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
                assert_eq!(veccard.len(), 32);
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

pub type SGameAnnouncements = SPlayersInRound<Option<Box<TActivelyPlayableRules>>>;

pub struct SGamePreparations<'rules> {
    pub m_ahand : EnumMap<EPlayerIndex, SHand>,
    m_doublings : SDoublings,
    pub m_ruleset : &'rules SRuleSet,
    pub m_gameannouncements : SGameAnnouncements,
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

pub enum VGamePreparationsFinish<'rules> {
    DetermineRules(SDetermineRules<'rules>),
    DirectGame(SPreGame),
    Stock(/*n_stock*/isize),

}

impl<'rules> SGamePreparations<'rules> {
    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        self.m_gameannouncements.current_playerindex()
    }

    pub fn announce_game(&mut self, epi: EPlayerIndex, orules: Option<Box<TActivelyPlayableRules>>) -> Result<()> {
        if Some(epi)!=self.which_player_can_do_something() {
            bail!("Wrong player index");
        }
        if orules.as_ref().map_or(false, |rules| Some(epi)!=rules.playerindex()) {
            bail!("Only actively playable rules can be announced");
        }
        self.m_gameannouncements.push(orules);
        assert!(0<self.m_gameannouncements.size());
        Ok(())
    }

    pub fn finish(self) -> VGamePreparationsFinish<'rules> {
        let vecpairepirules : Vec<(_, Box<TActivelyPlayableRules>)> = self.m_gameannouncements.into_iter()
            .filter_map(|(epi, orules)| orules.map(|rules| (epi, rules)))
            .collect();
        if !vecpairepirules.is_empty() {
            VGamePreparationsFinish::DetermineRules(SDetermineRules::new(
                self.m_ahand,
                self.m_doublings,
                self.m_ruleset,
                vecpairepirules,
                self.m_n_stock,
            ))
        } else {
            match self.m_ruleset.m_stockorramsch {
                VStockOrT::OrT(ref rulesramsch) => {
                    VGamePreparationsFinish::DirectGame(SPreGame {
                        m_ahand: self.m_ahand,
                        m_doublings: self.m_doublings,
                        m_rules: rulesramsch.clone(),
                        m_vecstoss: Vec::new(),
                        m_n_stock: self.m_n_stock,
                    })
                },
                VStockOrT::Stock(n_stock) => {
                    VGamePreparationsFinish::Stock(match self.m_ruleset.m_oedoublingscope {
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

pub struct SDetermineRules<'rules> {
    pub m_ahand : EnumMap<EPlayerIndex, SHand>,
    pub m_doublings : SDoublings,
    pub m_ruleset : &'rules SRuleSet,
    pub m_vecpairepirules_queued : Vec<(EPlayerIndex, Box<TActivelyPlayableRules>)>,
    pub m_n_stock : isize,
    m_pairepirules_current_bid : (EPlayerIndex, Box<TActivelyPlayableRules>),
}

impl<'rules> SDetermineRules<'rules> {
    /*
        Example:
        0: Rufspiel, 1: Wenz, 2: Farbwenz, 3: Rufspiel
        m_vecpairepirules_queued | m_pairepirules_current_bid
        0r 1w 2fw                | 3r EBid::AtLeast (indicating that 2fw needs a prio of at least the one offered by 3)
        => ask 2, and tell him that 3 offers r
        => if 2 announces game, we get 0r 1w 3r | 2fw EBid::Higher (indicating that 3 has to offer a strictly better prio)
           otherwise we get 0r 1w | 3r EBid::AtLeast
        => continue until m_vecpairepirules_queued is empty
    */

    pub fn new(
        ahand : EnumMap<EPlayerIndex, SHand>,
        doublings : SDoublings,
        ruleset: &SRuleSet,
        mut vecpaireplayerindexrules_queued : Vec<(EPlayerIndex, Box<TActivelyPlayableRules>)>,
        n_stock : isize,
    ) -> SDetermineRules {
        assert!(!vecpaireplayerindexrules_queued.is_empty());
        let pairepirules_current_bid = vecpaireplayerindexrules_queued.pop().unwrap();
        SDetermineRules {
            m_ahand : ahand,
            m_doublings : doublings,
            m_ruleset : ruleset,
            m_n_stock : n_stock,
            m_vecpairepirules_queued : vecpaireplayerindexrules_queued,
            m_pairepirules_current_bid : pairepirules_current_bid,
        }
    }

    pub fn which_player_can_do_something(&self) -> Option<(EPlayerIndex, Vec<SRuleGroup>)> {
        self.m_vecpairepirules_queued.last().as_ref().map(|&&(epi, ref _rules)| (
            epi,
            self.m_ruleset.m_avecrulegroup[epi].iter()
                .filter_map(|rulegroup| rulegroup.with_higher_prio_than(
                    &self.currently_offered_prio().1,
                    {
                        assert_ne!(epi, self.m_pairepirules_current_bid.0);
                        let doublings = &self.m_doublings;
                        if doublings.position(epi) < doublings.position(self.m_pairepirules_current_bid.0) {
                            EBid::AtLeast
                        } else {
                            EBid::Higher
                        }
                    }
                ))
                .collect()
        ))
    }

    pub fn currently_offered_prio(&self) -> (EPlayerIndex, VGameAnnouncementPriority) {
        (self.m_pairepirules_current_bid.0, self.m_pairepirules_current_bid.1.priority())
    }

    pub fn announce_game(&mut self, epi: EPlayerIndex, rules: Box<TActivelyPlayableRules>) -> Result<()> {
        if Some(epi)!=self.which_player_can_do_something().map(|(epi, ref _vecrulegroup)| epi) {
            bail!("announce_game not allowed for specified EPlayerIndex");
        }
        if rules.priority()<self.currently_offered_prio().1 {
            bail!("announced rules' priority must be at least as large as the latest announced priority");
        }
        assert_ne!(epi, self.m_pairepirules_current_bid.0);
        assert!(!self.m_vecpairepirules_queued.is_empty());
        let epi_check = self.m_vecpairepirules_queued.pop().unwrap().0;
        assert_eq!(epi, epi_check);
        let mut pairepirules_current_bid = (epi, rules);
        mem::swap(&mut self.m_pairepirules_current_bid, &mut pairepirules_current_bid);
        self.m_vecpairepirules_queued.push(pairepirules_current_bid);
        assert_eq!(epi, self.m_pairepirules_current_bid.0);
        Ok(())
    }

    pub fn resign(&mut self, epi: EPlayerIndex) -> Result<()> {
        if Some(epi)!=self.which_player_can_do_something().map(|(epi, ref _vecrulegroup)| epi) {
            bail!("announce_game not allowed for specified EPlayerIndex");
        }
        assert!(!self.m_vecpairepirules_queued.is_empty());
        let paireplayerindexorules = self.m_vecpairepirules_queued.pop().unwrap();
        assert_eq!(epi, paireplayerindexorules.0);
        Ok(())
    }

    pub fn finish(self) -> SPreGame {
        assert!(self.which_player_can_do_something().is_none());
        assert!(self.m_vecpairepirules_queued.is_empty());
        SPreGame {
            m_ahand: self.m_ahand,
            m_doublings: self.m_doublings,
            m_rules: self.m_pairepirules_current_bid.1.as_rules().box_clone(),
            m_vecstoss: Vec::new(),
            m_n_stock: self.m_n_stock,
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
