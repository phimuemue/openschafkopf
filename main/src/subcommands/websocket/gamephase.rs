use openschafkopf_lib::{
    primitives::*,
    game::*,
    rules::*,
    rules::ruleset::{SRuleSet, SRuleGroup, allowed_rules, VStockOrT},
};
use openschafkopf_util::*;
use serde::{Serialize, Deserialize};
use rand::{
    rng,
    prelude::*,
};
use itertools::Itertools;
use derive_new::new;
use plain_enum::{EnumMap, PlainEnum};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Infallible {} // TODO use std::convert::Infallible

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VGamePhaseGeneric<DealCards, GamePreparations, DetermineRules, Game, GameResult, Accepted> {
    DealCards(DealCards),
    GamePreparations(GamePreparations),
    DetermineRules(DetermineRules),
    Game(Game),
    GameResult(GameResult),
    Accepted(Accepted),
}

macro_rules! impl_try_zip{($fn_name:ident ($($refmut_lhs:tt)*) ($($refmut_rhs:tt)*)) => {
    fn $fn_name<R, DealCardsOther, GamePreparationsOther, DetermineRulesOther, GameOther, GameResultOther, AcceptedOther>(
        $($refmut_lhs)* self,
        other: $($refmut_rhs)* VGamePhaseGeneric<DealCardsOther, GamePreparationsOther, DetermineRulesOther, GameOther, GameResultOther, AcceptedOther>,
        value_on_failing_match: R,
        fn_deal_cards: impl FnOnce($($refmut_lhs)* DealCards, $($refmut_rhs)* DealCardsOther) -> R,
        fn_game_preparations: impl FnOnce($($refmut_lhs)* GamePreparations, $($refmut_rhs)* GamePreparationsOther) -> R,
        fn_determine_rules: impl FnOnce($($refmut_lhs)* DetermineRules, $($refmut_rhs)* DetermineRulesOther) -> R,
        fn_game: impl FnOnce($($refmut_lhs)* Game, $($refmut_rhs)* GameOther) -> R,
        fn_game_result: impl FnOnce($($refmut_lhs)* GameResult, $($refmut_rhs)* GameResultOther) -> R,
        fn_accepted: impl FnOnce($($refmut_lhs)* Accepted, $($refmut_rhs)* AcceptedOther) -> R,
    ) -> R {
        use VGamePhaseGeneric::*;
        match (self, other) {
            (DealCards(lhs), DealCards(rhs)) => fn_deal_cards(lhs, rhs),
            (DealCards(_), _) => value_on_failing_match,
            (GamePreparations(lhs), GamePreparations(rhs)) => fn_game_preparations(lhs, rhs),
            (GamePreparations(_), _) => value_on_failing_match,
            (DetermineRules(lhs), DetermineRules(rhs)) => fn_determine_rules(lhs, rhs),
            (DetermineRules(_), _) => value_on_failing_match,
            (Game(lhs), Game(rhs)) => fn_game(lhs, rhs),
            (Game(_), _) => value_on_failing_match,
            (GameResult(lhs), GameResult(rhs)) => fn_game_result(lhs, rhs),
            (GameResult(_), _) => value_on_failing_match,
            (Accepted(lhs), Accepted(rhs)) => fn_accepted(lhs, rhs),
            (Accepted(_), _) => value_on_failing_match,
        }
    }
}}

impl<DealCards, GamePreparations, DetermineRules, Game, GameResult, Accepted> VGamePhaseGeneric<DealCards, GamePreparations, DetermineRules, Game, GameResult, Accepted> {
    impl_try_zip!(try_zip (&) (&));
    impl_try_zip!(try_zip_mutref_move (&mut) ());

    pub fn matches_phase(&self, gamephase: &Self) -> bool {
        self.try_zip(
            gamephase,
            /*value_on_failing_match*/false,
            |_,_| true,
            |_,_| true,
            |_,_| true,
            |_,_| true,
            |_,_| true,
            |_,_| true,
        )
    }
}

#[derive(Debug)]
pub struct SWebsocketGameResult {
    // TODO? should the members be private?
    pub gameresult: SGameResult<SRuleSet>,
    pub setepi_confirmed: EnumSet<EPlayerIndex>,
}

impl SWebsocketGameResult {
    fn new(gameresult: SGameResult<SRuleSet>) -> Self {
        Self {
            gameresult,
            setepi_confirmed: EnumSet::new_empty(),
        }
    }
}

impl TGamePhase for SWebsocketGameResult {
    type ActivePlayerInfo = EnumSet<EPlayerIndex>;
    type Finish = SAccepted;
    fn which_player_can_do_something(&self) -> Option<Self::ActivePlayerInfo> {
        let None /*Option<Infallible>*/ = self.gameresult.which_player_can_do_something(); // TODO simplify
        if_then_some!(!self.setepi_confirmed.is_full(),
            self.setepi_confirmed.clone()
        )
    }
    fn finish_success(self) -> Self::Finish {
        SAccepted{}
    }
}

#[derive(Debug)]
pub struct SAccepted {
}

impl TGamePhase for SAccepted {
    type ActivePlayerInfo = Infallible; // TODO good idea to use Infallible here?
    type Finish = Self; // TODO? use SDealCards
    fn which_player_can_do_something(&self) -> Option<Self::ActivePlayerInfo> {
        None
    }
    fn finish_success(self) -> Self::Finish {
        self
    }
}

pub type VGamePhase = VGamePhaseGeneric<
    SDealCards,
    SGamePreparations,
    SDetermineRules,
    SGameGeneric<SRuleSet, (), ()>,
    SWebsocketGameResult,
    SAccepted,
>;
type VGamePhaseActivePlayerInfo<'a> = VGamePhaseGeneric<
    (&'a SDealCards, <SDealCards as TGamePhase>::ActivePlayerInfo),
    (&'a SGamePreparations, <SGamePreparations as TGamePhase>::ActivePlayerInfo),
    (&'a SDetermineRules, <SDetermineRules as TGamePhase>::ActivePlayerInfo),
    (&'a SGameGeneric<SRuleSet, (), ()>, <SGameGeneric<SRuleSet, (), ()> as TGamePhase>::ActivePlayerInfo),
    (&'a SWebsocketGameResult, <SWebsocketGameResult as TGamePhase>::ActivePlayerInfo),
    (&'a SAccepted, <SAccepted as TGamePhase>::ActivePlayerInfo),
>;

type SActivelyPlayableRulesIdentifier = String;
fn find_rules_by_id(slcrulegroup: &[SRuleGroup], hand: SFullHand, orulesid: &Option<SActivelyPlayableRulesIdentifier>) -> Result<Option<SActivelyPlayableRules>, ()> {
    allowed_rules(slcrulegroup, hand)
        .find(|orules|
            &orules.map(<SActivelyPlayableRules>::to_string)==orulesid
        )
        .map(|orules| orules.cloned()) // TODO clone needed?
        .ok_or(())
}

pub fn rules_to_gamephaseaction<'retval, 'rules : 'retval, 'hand : 'retval>(slcrulegroup: &'rules [SRuleGroup], hand: SFullHand<'hand>, fn_gamephaseaction: impl 'static + Clone + Fn(Option<SActivelyPlayableRulesIdentifier>)->VGamePhaseAction) -> impl Clone + Iterator<Item=(SActivelyPlayableRulesIdentifier, VGamePhaseAction)> + 'retval {
    allowed_rules(slcrulegroup, hand)
        .map(move |orules|
             (
                 if let Some(rules) = orules {
                     rules.to_string()
                 } else {
                     "Weiter".to_string()
                 },
                 fn_gamephaseaction(orules.map(<SActivelyPlayableRules>::to_string)),
             )
        )
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VGameAction {
    Stoss,
    Zugeben(ECard),
}

pub type VGamePhaseAction = VGamePhaseGeneric<
    /*DealCards announce_doubling*/ /*b_doubling*/bool,
    /*GamePreparations announce_game*/Option<SActivelyPlayableRulesIdentifier>,
    /*DetermineRules*/Option<SActivelyPlayableRulesIdentifier>,
    /*Game*/VGameAction,
    /*GameResult*/(),
    /*Accepted*/Infallible,
>;

#[derive(Serialize, Clone, Debug)]
pub enum VMessage {
    Info(String),
    Ask{
        str_question: String,
        vecstrgamephaseaction: Vec<(String, VGamePhaseAction)>,
    },
}

#[derive(new, Debug)]
pub struct STimeoutAction {
    pub epi: EPlayerIndex,
    pub gamephaseaction_timeout: VGamePhaseAction,
}

#[derive(Debug)]
pub struct SSendToPlayers {
    pub vecstich: Vec<SStich>, // TODO? Could borrow?
    pub orules: Option<SRules>, // TODO? Could borrow?
    pub mapepiveccard: EnumMap<EPlayerIndex, Vec<ECard>>,
    pub mapepiomsg_active: EnumMap<EPlayerIndex, Option<VMessage>>,
    pub msg_inactive: VMessage,
    pub otimeoutaction: Option<STimeoutAction>, // TODO can we avoid Option here?
}

impl SSendToPlayers {
    pub fn new<Card: TMoveOrClone<ECard>, ItCard: IntoIterator<Item=Card>> (
        vecstich: Vec<SStich>,
        orules: Option<SRules>,
        fn_cards: impl Fn(EPlayerIndex)->ItCard,
        fn_msg_active: impl Fn(EPlayerIndex)->Option<VMessage>,
        msg_inactive: VMessage,
        otimeoutaction: impl Into<Option<STimeoutAction>>,
    ) -> Self {
        Self {
            vecstich,
            orules,
            mapepiveccard: EPlayerIndex::map_from_fn(|epi| fn_cards(epi).into_iter().map(TMoveOrClone::move_or_clone).collect()),
            mapepiomsg_active: EPlayerIndex::map_from_fn(fn_msg_active),
            msg_inactive,
            otimeoutaction: otimeoutaction.into(),
        }
    }
}

impl VGamePhase {
    fn internal_which_player_can_do_something(&self) -> Option<VGamePhaseActivePlayerInfo<'_>> {
        use VGamePhaseGeneric::*;
        fn internal<GamePhase: TGamePhase>(gamephase: &GamePhase) -> Option<(&GamePhase, GamePhase::ActivePlayerInfo)> {
            gamephase.which_player_can_do_something()
                .map(|activeplayerinfo| (gamephase, activeplayerinfo))
        }
        match self {
            DealCards(dealcards) => internal(dealcards).map(DealCards),
            GamePreparations(gamepreparations) => internal(gamepreparations).map(GamePreparations),
            DetermineRules(determinerules) => internal(determinerules).map(DetermineRules),
            Game(game) => internal(game).map(Game),
            GameResult(gameresult) => internal(gameresult).map(GameResult),
            Accepted(accepted) => internal(accepted).map(Accepted),
        }
    }

    pub fn which_player_can_do_something(&self) -> Option<SSendToPlayers> {
        self.internal_which_player_can_do_something().map(|whichplayercandosomething| {
            use VGamePhaseGeneric::*;
            match whichplayercandosomething {
                DealCards((dealcards, epi_doubling)) => {
                    SSendToPlayers::new(
                        /*vecstich*/Vec::new(),
                        /*orules*/None,
                        |epi| dealcards.first_hand_for(epi),
                        /*fn_msg_active*/ |epi| {
                            if_then_some!(epi_doubling==epi,
                                VMessage::Ask{
                                    str_question: "Doppeln".into(),
                                    vecstrgamephaseaction: [(true, "Doppeln"), (false, "Nicht doppeln")]
                                        .into_iter()
                                        .map(|(b_doubling, str_doubling)| 
                                            (str_doubling.to_string(), VGamePhaseAction::DealCards(b_doubling))
                                        )
                                        .collect(),
                                }
                            )
                        },
                        /*msg_inactive*/VMessage::Info(format!("Asking {epi_doubling:?} for doubling")),
                        STimeoutAction::new(
                            epi_doubling,
                            VGamePhaseAction::DealCards(/*b_doubling*/false),
                        ),
                    )
                },
                GamePreparations((gamepreparations, epi_announce_game)) => {
                    let itgamephaseaction_rules = rules_to_gamephaseaction(
                        &gamepreparations.ruleset.avecrulegroup[epi_announce_game],
                        gamepreparations.fullhand(epi_announce_game),
                        VGamePhaseAction::GamePreparations,
                    );
                    let gamephaseaction_rules_default = unwrap!(itgamephaseaction_rules.clone().next()).1;
                    let vecstrgamephaseaction = itgamephaseaction_rules.collect::<Vec<_>>();
                    SSendToPlayers::new(
                        /*vecstich*/Vec::new(),
                        /*orules*/None,
                        |epi| gamepreparations.fullhand(epi).get(),
                        /*fn_msg_active*/ |epi| {
                            if_then_some!(epi_announce_game==epi,
                                VMessage::Ask{
                                    str_question: format!("Du bist an {}. Stelle. {}",
                                        epi_announce_game.to_usize() + 1, // EPlayerIndex is 0-based
                                        {
                                            // TODO inform about player names
                                            let vectplepirules = gamepreparations.gameannouncements
                                                .iter()
                                                .filter_map(|(epi, orules)| orules.as_ref().map(|rules| (epi, rules)))
                                                .collect::<Vec<_>>();
                                            if epi==EPlayerIndex::EPI0 {
                                                assert!(vectplepirules.is_empty());
                                                "".to_string()
                                            } else if vectplepirules.is_empty() {
                                                "Bisher will niemand spielen. Spielst Du?".to_string()
                                            } else {
                                                match vectplepirules.iter().exactly_one() {
                                                    Ok((epi_announced, _rules)) => {
                                                        format!(
                                                            "Vor Dir spielt an {}. Stelle. Spielst Du auch?",
                                                            epi_announced.to_usize() + 1, // EPlayerIndex is 0-based
                                                        )
                                                    },
                                                    Err(ittplepirules) => {
                                                        format!(
                                                            "Vor Dir spielen: An {}. Spielst Du auch?",
                                                            ittplepirules
                                                                .map(|(epi_announced, _rules)| {
                                                                    format!(
                                                                        "{}. Stelle",
                                                                        epi_announced.to_usize() + 1, // EPlayerIndex is 0-based
                                                                    )
                                                                })
                                                                .join(", ")
                                                        )
                                                    },
                                                }
                                            }
                                        }
                                    ),
                                    vecstrgamephaseaction: vecstrgamephaseaction.clone(),
                                }
                            )
                        },
                        /*msg_inactive*/VMessage::Info(format!("Asking {epi_announce_game:?} for game")),
                        STimeoutAction::new(
                            epi_announce_game,
                            gamephaseaction_rules_default,
                        ),
                    )
                },
                DetermineRules((determinerules, (epi_determine, vecrulegroup))) => {
                    let itgamephaseaction_rules = rules_to_gamephaseaction(
                        &vecrulegroup,
                        determinerules.fullhand(epi_determine),
                        VGamePhaseAction::DetermineRules,
                    );
                    let gamephaseaction_rules_default = unwrap!(itgamephaseaction_rules.clone().next()).1;
                    let vecstrgamephaseaction = itgamephaseaction_rules.collect::<Vec<_>>();
                    SSendToPlayers::new(
                        /*vecstich*/Vec::new(),
                        /*orules*/None,
                        |epi| determinerules.fullhand(epi).get(),
                        /*fn_msg_active*/ |epi| {
                            if_then_some!(epi_determine==epi,
                                VMessage::Ask{
                                    str_question: format!(
                                        "Du bist an {}. Stelle. Von {}. Stelle wird {} geboten. Spielst Du etwas staerkeres?", // TODO umlaut-tactics?
                                        epi.to_usize() + 1, // EPlayerIndex is 0-based
                                        determinerules.tplepirules_current_bid.0.to_usize() + 1, // EPlayerIndex is 0-based
                                        determinerules.tplepirules_current_bid.1,
                                    ),
                                    vecstrgamephaseaction: vecstrgamephaseaction.clone()
                                }
                            )
                        },
                        /*msg_inactive*/VMessage::Info(format!("Re-Asking {epi_determine:?} for game")),
                        STimeoutAction::new(
                            epi_determine,
                            gamephaseaction_rules_default,
                        ),
                    )
                },
                Game((game, (epi_card, vecepi_stoss))) => {
                    SSendToPlayers::new(
                        game.stichseq.visible_stichs().to_vec(),
                        Some(game.rules.clone()),
                        |epi| game.ahand[epi].cards(),
                        /*fn_msg_active*/ |epi| {
                            if_then_some!(vecepi_stoss.contains(&epi),
                                VMessage::Ask {
                                    str_question: "".into(),
                                    vecstrgamephaseaction: [("Stoss".into(), VGamePhaseAction::Game(VGameAction::Stoss))].to_vec(),
                                }
                            )
                        },
                        /*msg_inactive*/VMessage::Info(format!("Asking {epi_card:?} for card")),
                        STimeoutAction::new(
                            epi_card,
                            VGamePhaseAction::Game(VGameAction::Zugeben({
                                let itrules =game.rules.all_allowed_cards(
                                    &game.stichseq,
                                    &game.ahand[epi_card],
                                );
                                *unwrap!(itrules.choose(&mut rng()))
                            })),
                        ),
                    )
                },
                GameResult((gameresult, setepi_confirmed)) => {
                    SSendToPlayers::new(
                        /*vecstich*/if let VStockOrT::OrT(ref game) = gameresult.gameresult.stockorgame {
                            game.stichseq.completed_stichs().to_vec()
                        } else {
                            Vec::new()
                        },
                        if_then_some!(let VStockOrT::OrT(ref game) = gameresult.gameresult.stockorgame,
                            game.rules.clone()
                        ),
                        /*fn_cards*/|_epi| std::iter::empty::<ECard>(),
                        /*fn_msg_active*/ |epi| {
                            if_then_some!(!setepi_confirmed.contains(epi),
                                VMessage::Ask{
                                    str_question: format!("Spiel beendet. {}", if gameresult.gameresult.an_payout[epi] < 0 {
                                        format!("Verlust: {}", -gameresult.gameresult.an_payout[epi])
                                    } else {
                                        format!("Gewinn: {}", gameresult.gameresult.an_payout[epi])
                                    }),
                                    vecstrgamephaseaction: Some(("Ok".into(), VGamePhaseAction::GameResult(()))).into_iter().collect(),
                                }
                            )
                        },
                        /*msg_inactive*/VMessage::Info("Game finished".into()),
                        EPlayerIndex::values()
                            .find(|epi| !setepi_confirmed.contains(*epi))
                            .map(|epi_confirm|
                                STimeoutAction::new(
                                    epi_confirm,
                                    VGamePhaseAction::GameResult(()),
                                )
                            ),
                    )
                },
                // Accepted(_, Infallible) unreachable
            }
        })
    }

    #[allow(clippy::result_large_err)]
    pub fn action(mut self, epi: EPlayerIndex, gamephaseaction: VGamePhaseAction) -> Result<Self, /*Err contains original self*/Self> {
        let b_change = self.try_zip_mutref_move(gamephaseaction,
            /*value_on_failing_match*/false,
            |dealcards, b_doubling| {
                dealcards.announce_doubling(epi, b_doubling).is_ok()
            },
            |gamepreparations, orulesid| {
                find_rules_by_id(
                    &gamepreparations.ruleset.avecrulegroup[epi],
                    gamepreparations.fullhand(epi),
                    &orulesid
                ).ok().and_then(|orules| gamepreparations.announce_game(epi, orules).ok()).is_some()
            },
            |determinerules, orulesid| {
                determinerules.which_player_can_do_something()
                    .filter(|(epi_active, _vecrulegroup)| epi==*epi_active) // TODO needed?
                    .and_then(|(epi_active, vecrulegroup)| {
                        find_rules_by_id(
                            &vecrulegroup,
                            determinerules.fullhand(verify_eq!(epi, epi_active)),
                            &orulesid
                        ).ok().and_then(|orules| {
                            if let Some(rules) = orules {
                                determinerules.announce_game(epi, rules)
                            } else {
                                determinerules.resign(epi)
                            }.ok()
                        })
                    })
                    .is_some()
            },
            |game, gameaction| {
                match gameaction {
                    VGameAction::Stoss => game.stoss(epi),
                    VGameAction::Zugeben(card) => game.zugeben(card, epi),
                }.is_ok()
            },
            |gameresult, ()| {
                gameresult.setepi_confirmed.insert(epi)
            },
            |_accepted, _| {
                false
            },
        );
        if b_change {
            use VGamePhaseGeneric::*;
            loop {
                match self {
                    DealCards(dealcards) => match dealcards.finish() {
                        Ok(gamepreparations) => self = GamePreparations(gamepreparations),
                        Err((dealcards, _activeplayerinfo)) => return Ok(DealCards(dealcards)),
                    },
                    GamePreparations(gamepreparations) => match gamepreparations.finish() {
                        Ok(VGamePreparationsFinish::DetermineRules(determinerules)) => self = DetermineRules(determinerules),
                        Ok(VGamePreparationsFinish::DirectGame(game)) => self = Game(game),
                        Ok(VGamePreparationsFinish::Stock(gameresult)) => self = GameResult(SWebsocketGameResult::new(gameresult)),
                        Err((gamepreparations, _activeplayerinfo)) => return Ok(GamePreparations(gamepreparations)),
                    },
                    DetermineRules(determinerules) => match determinerules.finish() {
                        Ok(game) => self = Game(game),
                        Err((determinerules, _activeplayerinfo)) => return Ok(DetermineRules(determinerules)),
                    },
                    Game(game) => match game.finish() {
                        Ok(gameresult) => self = GameResult(SWebsocketGameResult::new(gameresult)),
                        Err((game, _activeplayerinfo)) => return Ok(Game(game)),
                    },
                    GameResult(gameresult) => match gameresult.finish() {
                        Ok(accepted) => {
                            let None /*Option<Infallible>*/ = accepted.which_player_can_do_something();
                            self = Accepted(accepted);
                        },
                        Err((gameresult, _activeplayerinfo)) => return Ok(GameResult(gameresult)),
                    },
                    Accepted(accepted) => return Ok(Accepted(accepted)),
                };
            };
        } else {
            Err(self)
        }
    }
}

