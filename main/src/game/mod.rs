use crate::primitives::*;
use crate::rules::{ruleset::*, *};
use crate::util::*;
use arrayvec::ArrayVec;
use rand::{self, Rng};
use std::mem;
use itertools::Itertools;

pub mod run;

pub trait TGamePhase : Sized {
    type ActivePlayerInfo;
    type Finish;

    fn which_player_can_do_something(&self) -> Option<Self::ActivePlayerInfo>;
    fn finish_success(self) -> Self::Finish;

    fn finish(self) -> Result<Self::Finish, Self> {
        if self.which_player_can_do_something().is_some() {
            Err(self)
        } else {
            Ok(self.finish_success())
        }
    }
}

pub type SDoublings = SPlayersInRound<bool, SStaticEPI0>;

#[derive(Debug)]
pub struct SDealCards {
    aveccard : EnumMap<EPlayerIndex, /*not yet a "hand"*/SHandVector>,
    doublings : SDoublings,
    ruleset : SRuleSet,
    n_stock : isize,
}

impl TGamePhase for SDealCards {
    type ActivePlayerInfo = EPlayerIndex;
    type Finish = SGamePreparations;

    fn which_player_can_do_something(&self) -> Option<Self::ActivePlayerInfo> {
        self.ruleset.oedoublingscope.as_ref().and_then(|_edoublingscope|
            self.doublings.current_playerindex()
        )
    }

    fn finish_success(self) -> Self::Finish {
        assert_eq!(self.doublings.first_playerindex(), EPlayerIndex::EPI0);
        SGamePreparations {
            aveccard : self.aveccard,
            doublings : self.doublings,
            ruleset: self.ruleset,
            gameannouncements : SGameAnnouncements::new(SStaticEPI0{}),
            n_stock: self.n_stock,
        }
    }
}

impl SDealCards {
    pub fn new(ruleset: SRuleSet, n_stock: isize) -> SDealCards {
        let ekurzlang = ruleset.ekurzlang;
        SDealCards {
            aveccard : {
                let mut veccard = SCard::values(ekurzlang).collect::<Vec<_>>();
                assert_eq!(veccard.len(), EPlayerIndex::SIZE*ekurzlang.cards_per_player());
                EPlayerIndex::map_from_fn(move |_epi|
                    random_hand(ekurzlang.cards_per_player(), &mut veccard)
                )
            },
            doublings: SDoublings::new(SStaticEPI0{}),
            ruleset,
            n_stock,
        }
    }

    pub fn first_hand_for(&self, epi: EPlayerIndex) -> &[SCard] {
        let veccard = &self.aveccard[epi];
        assert_eq!(veccard.len(), self.ruleset.ekurzlang.cards_per_player());
        &veccard[0..veccard.len()/2]
    }

    pub fn announce_doubling(&mut self, epi: EPlayerIndex, b_doubling: bool) -> Result<(), Error> {
        if Some(epi)!=self.which_player_can_do_something() {
            bail!("Wrong player index");
        }
        self.doublings.push(b_doubling);
        assert!(!self.doublings.is_empty());
        Ok(())
    }
}

pub type SGameAnnouncementsGeneric<GameAnnouncement> = SPlayersInRound<Option<GameAnnouncement>, SStaticEPI0>;
pub type SGameAnnouncements = SGameAnnouncementsGeneric<Box<dyn TActivelyPlayableRules>>;

#[derive(Debug)]
pub struct SGamePreparations {
    pub aveccard : EnumMap<EPlayerIndex, SHandVector>,
    pub doublings : SDoublings,
    pub ruleset : SRuleSet,
    pub gameannouncements : SGameAnnouncements,
    pub n_stock : isize,
}

pub fn random_hand(n_size: usize, veccard : &mut Vec<SCard>) -> SHandVector {
    assert!(veccard.len()>=n_size);
    let mut veccard_hand = SHandVector::new();
    for _i in 0..n_size {
        let i_card = rand::thread_rng().gen_range(0..veccard.len());
        veccard_hand.push(veccard.swap_remove(i_card));
    }
    assert_eq!(veccard_hand.len(), n_size);
    veccard_hand
}

#[allow(clippy::large_enum_variant)] // It is ok for DirectGame to be so large
#[derive(Debug)]
pub enum VGamePreparationsFinish {
    DetermineRules(SDetermineRules),
    DirectGame(SGame),
    Stock(SGameResult),
}

impl TGamePhase for SGamePreparations {
    type ActivePlayerInfo = EPlayerIndex;
    type Finish = VGamePreparationsFinish;

    fn which_player_can_do_something(&self) -> Option<Self::ActivePlayerInfo> {
        self.gameannouncements.current_playerindex()
    }

    fn finish_success(self) -> Self::Finish {
        let mut vectplepirules : Vec<(_, Box<dyn TActivelyPlayableRules>)> = self.gameannouncements.into_iter()
            .filter_map(|(epi, orules)| orules.map(|rules| (epi, rules)))
            .collect();
        if let Some(tplepirules_current_bid) = vectplepirules.pop() {
            VGamePreparationsFinish::DetermineRules(SDetermineRules::new(
                self.aveccard,
                self.doublings,
                self.ruleset,
                vectplepirules,
                self.n_stock,
                tplepirules_current_bid,
            ))
        } else {
            match self.ruleset.stockorramsch {
                VStockOrT::OrT(ref rulesramsch) => {
                    VGamePreparationsFinish::DirectGame(SGame::new(
                        self.aveccard,
                        self.doublings,
                        self.ruleset.ostossparams.clone(),
                        rulesramsch.clone(),
                        self.n_stock,
                    ))
                },
                VStockOrT::Stock(n_stock) => {
                    let n_stock = match self.ruleset.oedoublingscope {
                        None | Some(EDoublingScope::Games) => n_stock,
                        Some(EDoublingScope::GamesAndStock) => {
                            n_stock * 2isize.pow(
                                self.doublings.iter().filter(|&(_epi, &b_doubling)| b_doubling).count().as_num()
                            )
                        }
                    };
                    VGamePreparationsFinish::Stock(SGameResult{
                        an_payout: EPlayerIndex::map_from_fn(|_epi| -n_stock),
                        stockorgame: VStockOrT::Stock(()),
                    })
                }
            }
        }
    }
}

macro_rules! impl_fullhand { () => {
    pub fn fullhand(&self, epi: EPlayerIndex) -> SFullHand {
        SFullHand::new(&self.aveccard[epi], self.ruleset.ekurzlang)
    }
}}

impl SGamePreparations {
    impl_fullhand!();

    pub fn announce_game(&mut self, epi: EPlayerIndex, orules: Option<Box<dyn TActivelyPlayableRules>>) -> Result<(), Error> {
        if Some(epi)!=self.which_player_can_do_something() {
            bail!("Wrong player index");
        }
        if orules.as_ref().map_or(false, |rules| Some(epi)!=rules.playerindex()) {
            bail!("Only actively playable rules can be announced");
        }
        if !orules.as_ref().map_or(true, |rules| rules.can_be_played(self.fullhand(epi))) {
            bail!("Rules cannot be played. {}", SDisplayCardSlice(&self.aveccard[epi]));
        }
        self.gameannouncements.push(orules);
        assert!(!self.gameannouncements.is_empty());
        Ok(())
    }
}

#[derive(new, Debug)]
pub struct SDetermineRules {
    pub aveccard : EnumMap<EPlayerIndex, SHandVector>,
    pub doublings : SDoublings,
    pub ruleset : SRuleSet,
    pub vectplepirules_queued : Vec<(EPlayerIndex, Box<dyn TActivelyPlayableRules>)>,
    pub n_stock : isize,
    pub tplepirules_current_bid : (EPlayerIndex, Box<dyn TActivelyPlayableRules>),
}

impl TGamePhase for SDetermineRules {
    type ActivePlayerInfo = (EPlayerIndex, Vec<SRuleGroup>);
    type Finish = SGame;

    /*
        Example:
        0: Rufspiel, 1: Wenz, 2: Farbwenz, 3: Rufspiel
        self.vectplepirules_queued | self.tplepirules_current_bid
        0r 1w 2fw                | 3r EBid::AtLeast (indicating that 2fw needs a prio of at least the one offered by 3)
        => ask 2, and tell him that 3 offers r
        => if 2 announces game, we get 0r 1w 3r | 2fw EBid::Higher (indicating that 3 has to offer a strictly better prio)
           otherwise we get 0r 1w | 3r EBid::AtLeast
        => continue until self.vectplepirules_queued is empty
    */
    fn which_player_can_do_something(&self) -> Option<Self::ActivePlayerInfo> {
        self.vectplepirules_queued.last().as_ref().map(|&&(epi, ref _rules)| (
            epi,
            self.ruleset.avecrulegroup[epi].iter()
                .filter_map(|rulegroup| rulegroup.with_higher_prio_than(
                    &self.currently_offered_prio().1,
                    {
                        assert_ne!(epi, self.tplepirules_current_bid.0);
                        if epi < self.tplepirules_current_bid.0 {
                            EBid::AtLeast
                        } else {
                            EBid::Higher
                        }
                    }
                ))
                .collect()
        ))
    }

    fn finish_success(self) -> Self::Finish {
        assert!(self.vectplepirules_queued.is_empty());
        assert_eq!(self.ruleset.ekurzlang, EKurzLang::from_cards_per_player(self.aveccard[EPlayerIndex::EPI0].len()));
        SGame::new(
            self.aveccard,
            self.doublings,
            self.ruleset.ostossparams.clone(),
            self.tplepirules_current_bid.1.upcast().box_clone(),
            self.n_stock,
        )
    }
}

impl SDetermineRules {
    impl_fullhand!();

    pub fn currently_offered_prio(&self) -> (EPlayerIndex, VGameAnnouncementPriority) {
        (self.tplepirules_current_bid.0, self.tplepirules_current_bid.1.priority())
    }

    pub fn announce_game(&mut self, epi: EPlayerIndex, rules: Box<dyn TActivelyPlayableRules>) -> Result<(), Error> {
        if Some(epi)!=self.which_player_can_do_something().map(|(epi, ref _vecrulegroup)| epi) {
            bail!("announce_game not allowed for specified EPlayerIndex");
        }
        if rules.priority()<self.currently_offered_prio().1 {
            bail!("announced rules' priority must be at least as large as the latest announced priority");
        }
        if !rules.can_be_played(self.fullhand(epi)) {
            bail!("Rules cannot be played. {}", SDisplayCardSlice(&self.aveccard[epi]));
        }
        assert_ne!(epi, self.tplepirules_current_bid.0);
        assert!(!self.vectplepirules_queued.is_empty());
        let epi_check = unwrap!(self.vectplepirules_queued.pop()).0;
        assert_eq!(epi, epi_check);
        let mut tplepirules_current_bid = (epi, rules);
        mem::swap(&mut self.tplepirules_current_bid, &mut tplepirules_current_bid);
        self.vectplepirules_queued.push(tplepirules_current_bid);
        assert_eq!(epi, self.tplepirules_current_bid.0);
        Ok(())
    }

    pub fn resign(&mut self, epi: EPlayerIndex) -> Result<(), Error> {
        if Some(epi)!=self.which_player_can_do_something().map(|(epi, ref _vecrulegroup)| epi) {
            bail!("announce_game not allowed for specified EPlayerIndex");
        }
        assert!(!self.vectplepirules_queued.is_empty());
        let tpleplayerindexorules = unwrap!(self.vectplepirules_queued.pop());
        assert_eq!(epi, tpleplayerindexorules.0);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)] // TODO? custom impl Debug
pub struct SStichSequence {
    vecstich: ArrayVec<SStich, 9>,
    ekurzlang: EKurzLang,
}

use std::fmt;

impl std::fmt::Display for SStichSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for stich in self.completed_stichs() {
            write!(f, "{} | ", stich)?;
        }
        write!(f, "{}", self.current_stich())?;
        Ok(())
    }
}

impl SStichSequence {
    #[cfg(debug_assertions)]
    fn assert_invariant(&self) {
        assert!(!self.vecstich.is_empty());
        assert_eq!(self.vecstich[0].first_playerindex(), EPlayerIndex::EPI0);
        assert!(!self.current_stich_no_invariant().is_full());
        assert_eq!(self.vecstich[0..self.vecstich.len()-1].len(), self.vecstich.len()-1);
        assert!(self.vecstich[0..self.vecstich.len()-1].iter().all(SStich::is_full));
        assert!(self.completed_stichs_no_invariant().len()<=self.ekurzlang.cards_per_player());
        if self.completed_stichs_no_invariant().len()==self.ekurzlang.cards_per_player() {
            assert!(self.current_stich_no_invariant().is_empty());
        }
    }

    pub fn new(ekurzlang: EKurzLang) -> Self {
        let stichseq = SStichSequence {
            vecstich: {
                let mut vecstich = ArrayVec::new();
                vecstich.push(SStich::new(EPlayerIndex::EPI0));
                vecstich
            },
            ekurzlang,
        };
        #[cfg(debug_assertions)]stichseq.assert_invariant();
        stichseq
    }

    pub fn new_from_cards(ekurzlang: EKurzLang, itcard: impl Iterator<Item=SCard>, rules: &dyn TRules) -> Self {
        itcard.fold(Self::new(ekurzlang), mutate_return!(|stichseq, card| {
            stichseq.zugeben(card, rules);
        }))
    }

    pub fn game_finished(&self) -> bool {
        #[cfg(debug_assertions)]self.assert_invariant();
        assert!(self.completed_stichs().len()<=self.ekurzlang.cards_per_player());
        self.completed_stichs().len()==self.ekurzlang.cards_per_player()
    }

    pub fn no_card_played(&self) -> bool {
        #[cfg(debug_assertions)]self.assert_invariant();
        self.completed_stichs().is_empty() && self.current_stich().is_empty()
    }

    fn completed_stichs_no_invariant(&self) -> &[SStich] {
        &self.vecstich[0..self.vecstich.len()-1]
    }

    pub fn completed_stichs(&self) -> &[SStich] {
        #[cfg(debug_assertions)]self.assert_invariant();
        self.completed_stichs_no_invariant()
    }

    fn current_stich_no_invariant(&self) -> &SStich {
        unwrap!(self.vecstich.last())
    }

    pub fn current_stich(&self) -> &SStich {
        #[cfg(debug_assertions)]self.assert_invariant();
        self.current_stich_no_invariant()
    }

    pub fn zugeben_custom_winner_index(&mut self, card: SCard, fn_winner_index: impl FnOnce(&SStich)->EPlayerIndex) {
        #[cfg(debug_assertions)]self.assert_invariant();
        unwrap!(self.vecstich.last_mut()).push(card);
        if self.current_stich_no_invariant().is_full() {
            self.vecstich.push(SStich::new(fn_winner_index(self.current_stich_no_invariant())));
        }
        #[cfg(debug_assertions)]self.assert_invariant();
    }

    pub fn completed_stichs_custom_winner_index(&self, if_dbg_else!({fn_winner_index}{_fn_winner_index}): impl Fn(&SStich)->EPlayerIndex) -> impl Iterator<Item=(&SStich, EPlayerIndex)> {
        #[cfg(debug_assertions)]self.assert_invariant();
        self.vecstich[0..self.vecstich.len()]
            .iter()
            .tuple_windows()
            .map(move |(stich_0, stich_1)| {
                (stich_0, debug_verify_eq!(stich_1.first_playerindex(), fn_winner_index(stich_0)))
            })
    }

    pub fn completed_stichs_winner_index<'lifetime>(&'lifetime self, rules: &'lifetime impl TRules) -> impl Iterator<Item=(&'lifetime SStich, EPlayerIndex)> + 'lifetime {
        self.completed_stichs_custom_winner_index(move |stich| rules.winner_index(stich))
    }

    pub fn zugeben(&mut self, card: SCard, rules: &dyn TRules) {
        self.zugeben_custom_winner_index(card, |stich| rules.winner_index(stich));
    }

    pub fn zugeben_and_restore<R>(&mut self, card: SCard, rules: &dyn TRules, func: impl FnOnce(&mut Self)->R) -> R {
        #[cfg(debug_assertions)]self.assert_invariant();
        let n_len = self.vecstich.len();
        assert!(!self.current_stich().is_full());
        self.zugeben(card, rules);
        let r = func(self);
        if self.current_stich().is_empty() {
            unwrap!(self.vecstich.pop());
            assert!(self.current_stich_no_invariant().is_full());
        }
        unwrap!(self.vecstich.last_mut()).undo_most_recent();
        debug_assert_eq!(n_len, self.vecstich.len());
        #[cfg(debug_assertions)]self.assert_invariant();
        r
    }

    pub fn visible_stichs(&self) -> &[SStich] {
        &self.vecstich[0..self.vecstich.len().min(self.ekurzlang.cards_per_player())]
    }
    
    pub fn visible_cards(&self) -> impl Iterator<Item=(EPlayerIndex, &SCard)> {
        self.visible_stichs().iter().flat_map(SStich::iter)
    }

    pub fn kurzlang(&self) -> EKurzLang {
        #[cfg(debug_assertions)]self.assert_invariant();
        self.ekurzlang
    }

    pub fn count_played_cards(&self) -> usize {
        #[cfg(debug_assertions)]self.assert_invariant();
        self.completed_stichs().len() * EPlayerIndex::SIZE
            + self.current_stich().size()
    }
}

#[derive(Debug, Clone)]
pub struct SGameGeneric<Ruleset, GameAnnouncements, DetermineRules> {
    pub aveccard: EnumMap<EPlayerIndex, SHandVector>, // remembers order of dealt cards
    pub ahand : EnumMap<EPlayerIndex, SHand>,
    pub doublings : SDoublings,
    gameannouncements : GameAnnouncements,
    determinerules: DetermineRules,
    pub rules : Box<dyn TRules>,
    pub vecstoss : Vec<SStoss>,
    pub ostossparams : Option<SStossParams>,
    pub n_stock : isize,
    pub stichseq: SStichSequence,
    ruleset: Ruleset,
}
pub type SGame = SGameGeneric<(), (), ()>; // forgets ruleset and gameannouncements

pub type SGameAction = (EPlayerIndex, Vec<EPlayerIndex>);

impl<Ruleset, GameAnnouncements, DetermineRules> TGamePhase for SGameGeneric<Ruleset, GameAnnouncements, DetermineRules> {
    type ActivePlayerInfo = SGameAction;
    type Finish = SGameResultGeneric<Ruleset, GameAnnouncements, DetermineRules>;

    fn which_player_can_do_something(&self) -> Option<Self::ActivePlayerInfo> {
        if self.stichseq.completed_stichs().len() < self.kurzlang().cards_per_player() {
            self.current_playable_stich().current_playerindex().map(|epi_current| (
                epi_current,
                if let Some(ref stossparams) = self.ostossparams {
                    if self.stichseq.no_card_played() // TODORULES Adjustable latest time of stoss
                        && self.vecstoss.len() < stossparams.n_stoss_max
                    {
                        EPlayerIndex::values()
                            .filter(|epi| {
                                self.rules.stoss_allowed(*epi, &self.vecstoss, &self.ahand[*epi])
                            })
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                },
            ))
        } else {
            None
        }
    }

    fn finish_success(self) -> Self::Finish {
        assert!(self.kurzlang().cards_per_player()==self.completed_stichs().len());
        SGameResultGeneric {
            an_payout : self.rules.payout(
                SStichSequenceGameFinished::new(&self.stichseq),
                stoss_and_doublings(&self.vecstoss, &self.doublings),
                self.n_stock,
            ),
            stockorgame: VStockOrT::OrT(self),
        }
    }
}

impl SGame {
    pub fn new(
        aveccard : EnumMap<EPlayerIndex, SHandVector>,
        doublings : SDoublings,
        ostossparams : Option<SStossParams>,
        rules : Box<dyn TRules>,
        n_stock : isize,
    ) -> SGame {
        SGame::new_with(
            aveccard,
            doublings,
            ostossparams,
            rules,
            n_stock,
            /*ruleset*/(),
            /*gameannouncements*/(),
            /*determinerules*/(),
        )
    }
}

impl<Ruleset, GameAnnouncements, DetermineRules> SGameGeneric<Ruleset, GameAnnouncements, DetermineRules> {
    pub fn new_with(
        aveccard : EnumMap<EPlayerIndex, SHandVector>,
        doublings : SDoublings,
        ostossparams : Option<SStossParams>,
        rules : Box<dyn TRules>,
        n_stock : isize,
        ruleset: Ruleset,
        gameannouncements: GameAnnouncements,
        determinerules: DetermineRules,
    ) -> SGameGeneric<Ruleset, GameAnnouncements, DetermineRules> {
        let ahand = aveccard.map(|veccard| SHand::new_from_iter(veccard.iter().copied()));
        let n_cards_per_player = ahand[EPlayerIndex::EPI0].cards().len();
        assert!(ahand.iter().all(|hand| hand.cards().len()==n_cards_per_player));
        SGameGeneric {
            aveccard,
            ahand,
            doublings,
            gameannouncements,
            determinerules,
            rules,
            vecstoss: Vec::new(),
            ostossparams,
            n_stock,
            stichseq: SStichSequence::new(EKurzLang::from_cards_per_player(n_cards_per_player)),
            ruleset,
        }
    }

    pub fn new_finished(
        rules : Box<dyn TRules>,
        doublings : SDoublings,
        ostossparams : Option<SStossParams>,
        vecstoss: Vec<SStoss>,
        n_stock : isize,
        stichseq: SStichSequenceGameFinished, // TODO take value instead of wrapper
        mut fn_before_zugeben: impl FnMut(&SGame, /*i_stich*/usize, EPlayerIndex, SCard),
    ) -> Result<SGame, Error> {
        let aveccard = EPlayerIndex::map_from_fn(|epi|
            stichseq.get()
                .completed_stichs()
                .iter()
                .map(|stich| stich[epi])
                .collect()
        );
        let mut game = SGame::new(aveccard, doublings, ostossparams, rules, n_stock);
        for stoss in vecstoss.into_iter() {
            if game.stoss(stoss.epi).is_err() {
                bail!("Error in stoss.")
            }
        }
        for (i_stich, stich) in stichseq.get().completed_stichs().iter().enumerate() {
            for (epi, card) in stich.iter() {
                fn_before_zugeben(&game, i_stich, epi, *card);
                if game.zugeben(*card, epi).is_err() {
                    bail!("Error in zugeben.")
                }
            }
        }
        assert!(game.which_player_can_do_something().is_none());
        Ok(game)
    }

    pub fn map<Ruleset2, GameAnnouncements2, DetermineRules2>(self, fn_announcements: impl FnOnce(GameAnnouncements)->GameAnnouncements2, fn_determinerules: impl FnOnce(DetermineRules)->DetermineRules2, fn_ruleset: impl FnOnce(Ruleset)->Ruleset2) -> SGameGeneric<Ruleset2, GameAnnouncements2, DetermineRules2> {
        let SGameGeneric {
            aveccard,
            ahand,
            doublings,
            gameannouncements,
            determinerules,
            rules,
            vecstoss,
            ostossparams,
            n_stock,
            stichseq,
            ruleset,
        } = self;
        SGameGeneric {
            aveccard,
            ahand,
            doublings,
            gameannouncements: fn_announcements(gameannouncements),
            determinerules: fn_determinerules(determinerules),
            rules,
            vecstoss,
            ostossparams,
            n_stock,
            stichseq,
            ruleset: fn_ruleset(ruleset),
        }
    }

    pub fn current_playable_stich(&self) -> &SStich {
        assert!(self.stichseq.completed_stichs().len()<self.kurzlang().cards_per_player());
        self.stichseq.current_stich()
    }

    pub fn kurzlang(&self) -> EKurzLang {
        #[cfg(debug_assertions)] {
            let cards_per_player = |epi| {
                self.ahand[epi].cards().len()
                    + self.stichseq.completed_stichs().len()
                    + match self.stichseq.current_stich().get(epi) {
                        None => 0,
                        Some(_card) => 1,
                    }
            };
            assert!(EPlayerIndex::values().all(|epi| cards_per_player(epi)==cards_per_player(EPlayerIndex::EPI0)));
            assert_eq!(EKurzLang::from_cards_per_player(cards_per_player(EPlayerIndex::EPI0)), self.stichseq.ekurzlang);
        }
        self.stichseq.ekurzlang
    }

    pub fn stoss(&mut self, epi_stoss: EPlayerIndex) -> Result<(), Error> {
        match self.which_player_can_do_something() {
            None => bail!("Game already ended."),
            Some(gameaction) => {
                if !gameaction.1.iter().any(|&epi| epi==epi_stoss) {
                    bail!(format!("Stoss not allowed for specified epi {:?}", gameaction.1));
                }
                self.vecstoss.push(SStoss{epi : epi_stoss});
                Ok(())
            }
        }
    }

    pub fn zugeben(&mut self, card: SCard, epi: EPlayerIndex) -> Result<(), Error> {
        if Some(epi)!=self.which_player_can_do_something().map(|gameaction| gameaction.0) {
            bail!("Wrong player index");
        }
        if !self.ahand[epi].contains(card) {
            bail!("card not contained in player's hand");
        }
        if !self.rules.card_is_allowed(&self.stichseq, &self.ahand[epi], card) {
            bail!("{} is not allowed", card);
        }
        self.ahand[epi].play_card(card);
        self.stichseq.zugeben(card, self.rules.as_ref());
        Ok(())
    }

    pub fn completed_stichs(&self) -> &[SStich] {
        self.stichseq.completed_stichs()
    }
}

#[derive(Debug)]
pub struct SGameResultGeneric<Ruleset, GameAnnouncements, DetermineRules> {
    // TODO store all information about finished game, even in case of stock
    pub an_payout : EnumMap<EPlayerIndex, isize>,
    pub stockorgame: VStockOrT<(), SGameGeneric<Ruleset, GameAnnouncements, DetermineRules>>,
}
pub type SGameResult = SGameResultGeneric<(), (), ()>;

impl TGamePhase for SGameResult { // "absorbing state"
    type ActivePlayerInfo = std::convert::Infallible;
    type Finish = SGameResult;

    fn which_player_can_do_something(&self) -> Option<Self::ActivePlayerInfo> {
        None
    }
    fn finish_success(self) -> Self::Finish {
        self
    }
}

impl<Ruleset, GameAnnouncements, DetermineRules> SGameResultGeneric<Ruleset, GameAnnouncements, DetermineRules> {
    pub fn apply_payout(self, n_stock: &mut isize, mut fn_payout_to_epi: impl FnMut(EPlayerIndex, isize)) { // TODO should n_stock be member of SGameResult?
        for epi in EPlayerIndex::values() {
            fn_payout_to_epi(epi, self.an_payout[epi]);
        }
        let n_pay_into_stock = -self.an_payout.iter().sum::<isize>();
        assert!(
            n_pay_into_stock >= 0 // either pay into stock...
            || n_pay_into_stock == -*n_stock // ... or exactly empty it (assume that this is always possible)
        );
        *n_stock += n_pay_into_stock;
        assert!(0 <= *n_stock);
    }

    pub fn map<Ruleset2, GameAnnouncements2, DetermineRules2>(self, fn_announcements: impl FnOnce(GameAnnouncements)->GameAnnouncements2, fn_determinerules: impl FnOnce(DetermineRules)->DetermineRules2, fn_ruleset: impl FnOnce(Ruleset)->Ruleset2) -> SGameResultGeneric<Ruleset2, GameAnnouncements2, DetermineRules2> {
        let SGameResultGeneric {
            an_payout,
            stockorgame,
        } = self;
        SGameResultGeneric {
            an_payout,
            stockorgame: match stockorgame {
                VStockOrT::Stock(stock) => VStockOrT::Stock(stock),
                VStockOrT::OrT(game) => VStockOrT::OrT(game.map(fn_announcements, fn_determinerules, fn_ruleset)),
            },
        }
    }
}

pub fn stoss_and_doublings(vecstoss: &[SStoss], doublings: &SDoublings) -> (usize, usize) {
    (
        vecstoss.len(),
        doublings.iter().filter(|&(_epi, &b_doubling)| b_doubling).count(),
    )
}
