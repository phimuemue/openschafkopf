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
    ahand : EnumMap<EPlayerIndex, SHand>,
    doublings : SDoublings,
    ruleset : &'rules SRuleSet,
}

impl<'rules> SDealCards<'rules> {
    pub fn new(epi_first: EPlayerIndex, ruleset: &SRuleSet) -> SDealCards {
        SDealCards {
            ahand : {
                let mut veccard : Vec<_> = SCard::values(ruleset.ekurzlang).into_iter().collect();
                assert_eq!(veccard.len(), EPlayerIndex::ubound_usize()*ruleset.ekurzlang.cards_per_player());
                EPlayerIndex::map_from_fn(move |_epi|
                    random_hand(ruleset.ekurzlang.cards_per_player(), &mut veccard)
                )
            },
            doublings: SDoublings::new(epi_first),
            ruleset,
        }
    }

    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        self.ruleset.oedoublingscope.as_ref().and_then(|_edoublingscope|
            self.doublings.current_playerindex()
        )
    }

    pub fn first_hand_for(&self, epi: EPlayerIndex) -> &[SCard] {
        let veccard = self.ahand[epi].cards();
        assert_eq!(veccard.len(), self.ruleset.ekurzlang.cards_per_player());
        &veccard[0..veccard.len()/2]
    }

    pub fn announce_doubling(&mut self, epi: EPlayerIndex, b_doubling: bool) -> Result<()> {
        if Some(epi)!=self.which_player_can_do_something() {
            bail!("Wrong player index");
        }
        self.doublings.push(b_doubling);
        assert!(0<self.doublings.size());
        Ok(())
    }

    pub fn finish_dealing(self, ruleset: &SRuleSet, n_stock: isize) -> Result<SGamePreparations> {
        if let Some(epi) = self.which_player_can_do_something() {
            bail!(format!("{} still could announce doubling", epi));
        }
        let epi_first = self.doublings.first_playerindex();
        Ok(SGamePreparations {
            ahand : self.ahand,
            doublings : self.doublings,
            ruleset,
            gameannouncements : SGameAnnouncements::new(epi_first),
            n_stock,
        })
    }


}

pub type SGameAnnouncements = SPlayersInRound<Option<Box<TActivelyPlayableRules>>>;

pub struct SGamePreparations<'rules> {
    pub ahand : EnumMap<EPlayerIndex, SHand>,
    doublings : SDoublings,
    pub ruleset : &'rules SRuleSet,
    pub gameannouncements : SGameAnnouncements,
    pub n_stock : isize,
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
        self.gameannouncements.current_playerindex()
    }

    pub fn announce_game(&mut self, epi: EPlayerIndex, orules: Option<Box<TActivelyPlayableRules>>) -> Result<()> {
        if Some(epi)!=self.which_player_can_do_something() {
            bail!("Wrong player index");
        }
        if orules.as_ref().map_or(false, |rules| Some(epi)!=rules.playerindex()) {
            bail!("Only actively playable rules can be announced");
        }
        self.gameannouncements.push(orules);
        assert!(0<self.gameannouncements.size());
        Ok(())
    }

    pub fn finish(self) -> VGamePreparationsFinish<'rules> {
        let vecpairepirules : Vec<(_, Box<TActivelyPlayableRules>)> = self.gameannouncements.into_iter()
            .filter_map(|(epi, orules)| orules.map(|rules| (epi, rules)))
            .collect();
        if !vecpairepirules.is_empty() {
            VGamePreparationsFinish::DetermineRules(SDetermineRules::new(
                self.ahand,
                self.doublings,
                self.ruleset,
                vecpairepirules,
                self.n_stock,
            ))
        } else {
            match self.ruleset.stockorramsch {
                VStockOrT::OrT(ref rulesramsch) => {
                    VGamePreparationsFinish::DirectGame(SPreGame::new(
                        self.ahand,
                        self.doublings,
                        rulesramsch.clone(),
                        self.n_stock,
                    ))
                },
                VStockOrT::Stock(n_stock) => {
                    VGamePreparationsFinish::Stock(match self.ruleset.oedoublingscope {
                        None | Some(EDoublingScope::Games) => n_stock,
                        Some(EDoublingScope::GamesAndStock) => {
                            n_stock * 2isize.pow(
                                self.doublings.iter().filter(|&(_epi, &b_doubling)| b_doubling).count().as_num()
                            )
                        }
                    })
                }
            }
        }
    }
}

pub struct SDetermineRules<'rules> {
    pub ahand : EnumMap<EPlayerIndex, SHand>,
    pub doublings : SDoublings,
    pub ruleset : &'rules SRuleSet,
    pub vecpairepirules_queued : Vec<(EPlayerIndex, Box<TActivelyPlayableRules>)>,
    pub n_stock : isize,
    pairepirules_current_bid : (EPlayerIndex, Box<TActivelyPlayableRules>),
}

impl<'rules> SDetermineRules<'rules> {
    /*
        Example:
        0: Rufspiel, 1: Wenz, 2: Farbwenz, 3: Rufspiel
        self.vecpairepirules_queued | self.pairepirules_current_bid
        0r 1w 2fw                | 3r EBid::AtLeast (indicating that 2fw needs a prio of at least the one offered by 3)
        => ask 2, and tell him that 3 offers r
        => if 2 announces game, we get 0r 1w 3r | 2fw EBid::Higher (indicating that 3 has to offer a strictly better prio)
           otherwise we get 0r 1w | 3r EBid::AtLeast
        => continue until self.vecpairepirules_queued is empty
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
            ahand,
            doublings,
            ruleset,
            n_stock,
            vecpairepirules_queued : vecpaireplayerindexrules_queued,
            pairepirules_current_bid,
        }
    }

    pub fn which_player_can_do_something(&self) -> Option<(EPlayerIndex, Vec<SRuleGroup>)> {
        self.vecpairepirules_queued.last().as_ref().map(|&&(epi, ref _rules)| (
            epi,
            self.ruleset.avecrulegroup[epi].iter()
                .filter_map(|rulegroup| rulegroup.with_higher_prio_than(
                    &self.currently_offered_prio().1,
                    {
                        assert_ne!(epi, self.pairepirules_current_bid.0);
                        let doublings = &self.doublings;
                        if doublings.position(epi) < doublings.position(self.pairepirules_current_bid.0) {
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
        (self.pairepirules_current_bid.0, self.pairepirules_current_bid.1.priority())
    }

    pub fn announce_game(&mut self, epi: EPlayerIndex, rules: Box<TActivelyPlayableRules>) -> Result<()> {
        if Some(epi)!=self.which_player_can_do_something().map(|(epi, ref _vecrulegroup)| epi) {
            bail!("announce_game not allowed for specified EPlayerIndex");
        }
        if rules.priority()<self.currently_offered_prio().1 {
            bail!("announced rules' priority must be at least as large as the latest announced priority");
        }
        assert_ne!(epi, self.pairepirules_current_bid.0);
        assert!(!self.vecpairepirules_queued.is_empty());
        let epi_check = self.vecpairepirules_queued.pop().unwrap().0;
        assert_eq!(epi, epi_check);
        let mut pairepirules_current_bid = (epi, rules);
        mem::swap(&mut self.pairepirules_current_bid, &mut pairepirules_current_bid);
        self.vecpairepirules_queued.push(pairepirules_current_bid);
        assert_eq!(epi, self.pairepirules_current_bid.0);
        Ok(())
    }

    pub fn resign(&mut self, epi: EPlayerIndex) -> Result<()> {
        if Some(epi)!=self.which_player_can_do_something().map(|(epi, ref _vecrulegroup)| epi) {
            bail!("announce_game not allowed for specified EPlayerIndex");
        }
        assert!(!self.vecpairepirules_queued.is_empty());
        let paireplayerindexorules = self.vecpairepirules_queued.pop().unwrap();
        assert_eq!(epi, paireplayerindexorules.0);
        Ok(())
    }

    pub fn finish(self) -> Result<SPreGame> {
        if let Some((epi, _)) = self.which_player_can_do_something() {
            bail!(format!("{} still could do something", epi));
        }
        assert!(self.vecpairepirules_queued.is_empty());
        assert_eq!(self.ruleset.ekurzlang, EKurzLang::from_cards_per_player(self.ahand[EPlayerIndex::EPI0].cards().len()));
        Ok(SPreGame::new(
            self.ahand,
            self.doublings,
            self.pairepirules_current_bid.1.as_rules().box_clone(),
            self.n_stock,
        ))
    }
}

pub struct SPreGame {
    pub ahand : EnumMap<EPlayerIndex, SHand>,
    pub doublings : SDoublings,
    pub rules : Box<TRules>,
    pub vecstoss : Vec<SStoss>,
    pub n_stock : isize,
}

impl SPreGame {
    pub fn new(
        ahand : EnumMap<EPlayerIndex, SHand>,
        doublings : SDoublings,
        rules : Box<TRules>,
        n_stock : isize,
    ) -> SPreGame {
        SPreGame {ahand, doublings, rules, vecstoss: Vec::new(), n_stock}
    }
    pub fn which_player_can_do_something(&self) -> Vec<EPlayerIndex> {
        if self.vecstoss.len() < 4 {
            EPlayerIndex::values()
                .map(|epi| epi.wrapping_add(self.doublings.first_playerindex().to_usize()))
                .filter(|epi| self.rules.stoss_allowed(*epi, &self.vecstoss, &self.ahand[*epi]))
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
        self.vecstoss.push(SStoss{epi : epi_stoss});
        Ok(())
    }

    pub fn finish(self) -> SGame {
        let epi_first = self.doublings.first_playerindex();
        SGame {
            ahand : self.ahand,
            doublings : self.doublings,
            rules : self.rules,
            vecstoss : self.vecstoss,
            n_stock : self.n_stock,
            vecstich : vec![SStich::new(epi_first)],
        }
    }
}

pub struct SGame {
    pub ahand : EnumMap<EPlayerIndex, SHand>,
    pub doublings : SDoublings,
    pub rules : Box<TRules>,
    pub vecstoss : Vec<SStoss>,
    pub n_stock : isize,
    pub vecstich : Vec<SStich>,
}

impl SGame {
    pub fn which_player_can_do_something(&self) -> Option<EPlayerIndex> {
        self.current_stich().current_playerindex()
    }

    pub fn current_stich(&self) -> &SStich {
        current_stich(&self.vecstich)
    }

    fn kurzlang(&self) -> EKurzLang {
        let cards_per_player = |epi| {
            self.vecstich.iter().filter(|stich| stich.get(epi).is_some()).count() + self.ahand[epi].cards().len()
        };
        assert!(EPlayerIndex::values().all(|epi| cards_per_player(epi)==cards_per_player(EPlayerIndex::EPI0)));
        EKurzLang::from_cards_per_player(cards_per_player(EPlayerIndex::EPI0))
    }

    pub fn zugeben(&mut self, card_played: SCard, epi: EPlayerIndex) -> Result<()> {
        // returns the EPlayerIndex of the player who is the next in row to do something
        skui::logln(&format!("Player {} wants to play {}", epi, card_played));
        if Some(epi)!=self.which_player_can_do_something() {
            bail!("Wrong player index");
        }
        if !self.ahand[epi].contains(card_played) {
            bail!("card not contained in player's hand");
        }
        {
            let hand = &mut self.ahand[epi];
            assert!(self.rules.card_is_allowed(&self.vecstich, hand, card_played));
            hand.play_card(card_played);
            assert!(!self.vecstich.is_empty());
            self.vecstich.last_mut().unwrap().push(card_played);
        }
        for epi in EPlayerIndex::values() {
            skui::logln(&format!("Hand {}: {}", epi, self.ahand[epi]));
        }
        if 4==self.current_stich().size() {
            if self.kurzlang().cards_per_player()==self.vecstich.len() {
                skui::logln("Game finished.");
                skui::print_vecstich(&self.vecstich);
                Ok(())
            } else {
                let epi_last_stich = {
                    let stich = self.current_stich();
                    skui::logln(&format!("Stich: {}", stich));
                    self.rules.winner_index(stich)
                };
                skui::logln(&format!("Opening new stich starting at {}", epi_last_stich));
                assert!(self.vecstich.is_empty() || 4==self.current_stich().size());
                self.vecstich.push(SStich::new(epi_last_stich));
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    pub fn payout(&self) -> SAccountBalance {
        assert!(self.which_player_can_do_something().is_none());
        self.rules.payout(
            &SGameFinishedStiche::new(&self.vecstich, self.kurzlang()),
            stoss_and_doublings(&self.vecstoss, &self.doublings),
            self.n_stock,
        )
    }

    pub fn completed_stichs(&self) -> &[SStich] {
        completed_stichs(&self.vecstich)
    }
}

pub fn stoss_and_doublings(vecstoss: &[SStoss], doublings: &SDoublings) -> (usize, usize) {
    (
        vecstoss.len(),
        doublings.iter().filter(|&(_epi, &b_doubling)| b_doubling).count(),
    )
}
