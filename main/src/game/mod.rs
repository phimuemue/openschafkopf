use crate::primitives::*;
use crate::rules::{ruleset::*, *};
use crate::util::*;
use rand::{self, Rng};
use std::mem;

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

#[derive(Debug, Clone)]
pub struct SExpensifiersNoStoss {
    n_stock : isize,
    doublings : SDoublings,
}

impl SExpensifiersNoStoss {
    pub fn new(n_stock: isize) -> Self {
        Self::new_with_doublings(n_stock, SDoublings::new(SStaticEPI0{}))
    }

    pub fn new_with_doublings(n_stock: isize, doublings: SDoublings) -> Self {
        Self {
            n_stock,
            doublings,
        }
    }

    fn into_with_stoss(self) -> SExpensifiers {
        SExpensifiers::new(
            self.n_stock,
            self.doublings,
            /*vecstoss*/vec!(),
        )
    }
}

#[derive(Debug)]
pub struct SDealCards {
    aveccard : EnumMap<EPlayerIndex, /*not yet a "hand"*/SHandVector>,
    expensifiers: SExpensifiersNoStoss,
    ruleset : SRuleSet,
}

impl TGamePhase for SDealCards {
    type ActivePlayerInfo = EPlayerIndex;
    type Finish = SGamePreparations;

    fn which_player_can_do_something(&self) -> Option<Self::ActivePlayerInfo> {
        self.ruleset.oedoublingscope.as_ref().and_then(|_edoublingscope|
            self.expensifiers.doublings.current_playerindex()
        )
    }

    fn finish_success(self) -> Self::Finish {
        assert_eq!(self.expensifiers.doublings.first_playerindex(), EPlayerIndex::EPI0);
        SGamePreparations {
            aveccard : self.aveccard,
            expensifiers: self.expensifiers,
            ruleset: self.ruleset,
            gameannouncements : SGameAnnouncements::new(SStaticEPI0{}),
        }
    }
}

impl SDealCards {
    pub fn new(ruleset: SRuleSet, n_stock: isize) -> SDealCards {
        let ekurzlang = ruleset.ekurzlang;
        SDealCards {
            aveccard : {
                let mut veccard = ECard::values(ekurzlang).collect::<Vec<_>>();
                assert_eq!(veccard.len(), EPlayerIndex::SIZE*ekurzlang.cards_per_player());
                EPlayerIndex::map_from_fn(move |_epi|
                    random_hand(ekurzlang.cards_per_player(), &mut veccard)
                )
            },
            expensifiers: SExpensifiersNoStoss::new(n_stock),
            ruleset,
        }
    }

    pub fn first_hand_for(&self, epi: EPlayerIndex) -> &[ECard] {
        let veccard = &self.aveccard[epi];
        assert_eq!(veccard.len(), self.ruleset.ekurzlang.cards_per_player());
        &veccard[0..veccard.len()/2]
    }

    pub fn announce_doubling(&mut self, epi: EPlayerIndex, b_doubling: bool) -> Result<(), &'static str> {
        if Some(epi)!=self.which_player_can_do_something() {
            Err("Wrong player index")
        } else {
            self.expensifiers.doublings.push(b_doubling);
            assert!(!self.expensifiers.doublings.is_empty());
            Ok(())
        }
    }
}

pub type SGameAnnouncementsGeneric<GameAnnouncement> = SPlayersInRound<Option<GameAnnouncement>, SStaticEPI0>;
pub type SGameAnnouncements = SGameAnnouncementsGeneric<Box<dyn TActivelyPlayableRules>>;

#[derive(Debug)]
pub struct SGamePreparations {
    pub aveccard : EnumMap<EPlayerIndex, SHandVector>,
    pub expensifiers: SExpensifiersNoStoss,
    pub ruleset : SRuleSet,
    pub gameannouncements : SGameAnnouncements,
}

pub fn random_hand(n_size: usize, veccard : &mut Vec<ECard>) -> SHandVector {
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
                self.expensifiers,
                self.ruleset,
                vectplepirules,
                tplepirules_current_bid,
            ))
        } else {
            match self.ruleset.stockorramsch {
                VStockOrT::OrT(ref rulesramsch) => {
                    VGamePreparationsFinish::DirectGame(SGame::new(
                        self.aveccard,
                        self.expensifiers,
                        self.ruleset.ostossparams.clone(),
                        rulesramsch.clone(),
                    ))
                },
                VStockOrT::Stock(n_stock) => {
                    let n_stock = match self.ruleset.oedoublingscope {
                        None | Some(EDoublingScope::Games) => n_stock,
                        Some(EDoublingScope::GamesAndStock) => {
                            n_stock * 2isize.pow(
                                self.expensifiers.doublings.iter().filter(|&(_epi, &b_doubling)| b_doubling).count().as_num()
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
            bail!("Rules cannot be played. {}", SDisplayCardSlice::new(self.aveccard[epi].clone(), &orules));
        }
        self.gameannouncements.push(orules);
        assert!(!self.gameannouncements.is_empty());
        Ok(())
    }
}

#[derive(new, Debug)]
pub struct SDetermineRules {
    pub aveccard : EnumMap<EPlayerIndex, SHandVector>,
    pub expensifiers : SExpensifiersNoStoss,
    pub ruleset : SRuleSet,
    pub vectplepirules_queued : Vec<(EPlayerIndex, Box<dyn TActivelyPlayableRules>)>,
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
        assert_eq!(self.ruleset.ekurzlang, unwrap!(EKurzLang::from_cards_per_player(self.aveccard[EPlayerIndex::EPI0].len())));
        SGame::new(
            self.aveccard,
            self.expensifiers,
            self.ruleset.ostossparams.clone(),
            self.tplepirules_current_bid.1.upcast().box_clone(),
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
            bail!("Rules cannot be played. {}", SDisplayCardSlice::new(self.aveccard[epi].clone(), &rules));
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

#[derive(Debug, Clone)]
pub struct SGameGeneric<Ruleset, GameAnnouncements, DetermineRules> {
    pub aveccard: EnumMap<EPlayerIndex, SHandVector>, // remembers order of dealt cards
    pub ahand : EnumMap<EPlayerIndex, SHand>,
    gameannouncements : GameAnnouncements,
    determinerules: DetermineRules,
    pub rules : Box<dyn TRules>,
    pub ostossparams : Option<SStossParams>,
    pub expensifiers: SExpensifiers,
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
                        && self.expensifiers.vecstoss.len() < stossparams.n_stoss_max
                    {
                        EPlayerIndex::values()
                            .filter(|epi| {
                                self.rules.stoss_allowed(*epi, &self.expensifiers.vecstoss, &self.ahand[*epi])
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
        let stichseq = SStichSequenceGameFinished::new(&self.stichseq);
        SGameResultGeneric {
            an_payout : self.rules.payout(
                stichseq,
                &self.expensifiers,
                &SRuleStateCache::new_from_gamefinishedstiche(
                    stichseq,
                    self.rules.as_ref(),
                ),
                /*b_test_points_as_payout*/if_dbg_else!({true}{()}),
            ),
            stockorgame: VStockOrT::OrT(self),
        }
    }
}

impl SGame {
    pub fn new(
        aveccard : EnumMap<EPlayerIndex, SHandVector>,
        expensifiers : SExpensifiersNoStoss,
        ostossparams : Option<SStossParams>,
        rules : Box<dyn TRules>,
    ) -> SGame {
        SGame::new_with(
            aveccard,
            expensifiers,
            ostossparams,
            rules,
            /*ruleset*/(),
            /*gameannouncements*/(),
            /*determinerules*/(),
        )
    }
}

impl<Ruleset, GameAnnouncements, DetermineRules> SGameGeneric<Ruleset, GameAnnouncements, DetermineRules> {
    pub fn new_with(
        aveccard : EnumMap<EPlayerIndex, SHandVector>,
        expensifiers : SExpensifiersNoStoss,
        ostossparams : Option<SStossParams>,
        rules : Box<dyn TRules>,
        ruleset: Ruleset,
        gameannouncements: GameAnnouncements,
        determinerules: DetermineRules,
    ) -> SGameGeneric<Ruleset, GameAnnouncements, DetermineRules> {
        let ahand = aveccard.map(|veccard| SHand::new_from_iter(veccard));
        let n_cards_per_player = ahand[EPlayerIndex::EPI0].cards().len();
        assert!(ahand.iter().all(|hand| hand.cards().len()==n_cards_per_player));
        SGameGeneric {
            aveccard,
            ahand,
            gameannouncements,
            determinerules,
            rules,
            ostossparams,
            expensifiers: SExpensifiers::new(expensifiers.n_stock, expensifiers.doublings, /*vecstoss*/vec!()),
            stichseq: SStichSequence::new(unwrap!(EKurzLang::from_cards_per_player(n_cards_per_player))),
            ruleset,
        }
    }

    pub fn new_finished(
        rules : Box<dyn TRules>,
        ostossparams : Option<SStossParams>,
        expensifiers: SExpensifiers,
        stichseq: SStichSequenceGameFinished, // TODO take value instead of wrapper
        mut fn_before_zugeben: impl FnMut(&SGame, /*i_stich*/usize, EPlayerIndex, ECard),
    ) -> Result<SGame, Error> {
        let aveccard = EPlayerIndex::map_from_fn(|epi|
            stichseq.get()
                .completed_cards_by(epi)
                .collect()
        );
        let SExpensifiers{n_stock, doublings, vecstoss} = expensifiers;
        let mut game = SGame::new(aveccard, SExpensifiersNoStoss::new_with_doublings(n_stock, doublings), ostossparams, rules);
        for stoss in vecstoss.into_iter() {
            game.stoss(stoss.epi)?;
        }
        for (i_stich, stich) in stichseq.get().completed_stichs().iter().enumerate() {
            for (epi, card) in stich.iter() {
                fn_before_zugeben(&game, i_stich, epi, *card);
                game.zugeben(*card, epi)?;
            }
        }
        assert!(game.which_player_can_do_something().is_none());
        Ok(game)
    }

    pub fn map<Ruleset2, GameAnnouncements2, DetermineRules2>(self, fn_announcements: impl FnOnce(GameAnnouncements)->GameAnnouncements2, fn_determinerules: impl FnOnce(DetermineRules)->DetermineRules2, fn_ruleset: impl FnOnce(Ruleset)->Ruleset2) -> SGameGeneric<Ruleset2, GameAnnouncements2, DetermineRules2> {
        let SGameGeneric {
            aveccard,
            ahand,
            gameannouncements,
            determinerules,
            rules,
            ostossparams,
            expensifiers,
            stichseq,
            ruleset,
        } = self;
        SGameGeneric {
            aveccard,
            ahand,
            gameannouncements: fn_announcements(gameannouncements),
            determinerules: fn_determinerules(determinerules),
            rules,
            ostossparams,
            expensifiers,
            stichseq,
            ruleset: fn_ruleset(ruleset),
        }
    }

    pub fn current_playable_stich(&self) -> &SStich {
        assert!(self.stichseq.completed_stichs().len()<self.kurzlang().cards_per_player());
        self.stichseq.current_stich()
    }

    pub fn kurzlang(&self) -> EKurzLang {
        debug_assert_eq!(self.stichseq.remaining_cards_per_hand(), self.ahand.map(|hand| hand.cards().len()));
        self.stichseq.kurzlang()
    }

    pub fn stoss(&mut self, epi_stoss: EPlayerIndex) -> Result<(), Error> {
        match self.which_player_can_do_something() {
            None => bail!("Game already ended."),
            Some(gameaction) => {
                if !gameaction.1.iter().any(|&epi| epi==epi_stoss) {
                    bail!(format!("Stoss not allowed for specified epi {:?}", gameaction.1));
                }
                self.expensifiers.vecstoss.push(SStoss{epi : epi_stoss});
                Ok(())
            }
        }
    }

    pub fn zugeben(&mut self, card: ECard, epi: EPlayerIndex) -> Result<(), Error> {
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

#[derive(Debug, Clone)]
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
    pub fn apply_payout(self, n_stock: &mut isize, mut fn_payout_to_epi: impl FnMut(EPlayerIndex, isize)) { // TODO should n_stock be member of SGameResult? // TODO should apply_payout be forced upon construction?
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

