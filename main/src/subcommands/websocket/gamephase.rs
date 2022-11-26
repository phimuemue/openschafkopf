use crate::{
    util::*,
    primitives::*,
    game::*,
    rules::*,
    rules::ruleset::{SRuleGroup, allowed_rules},
};
use serde::{Serialize, Deserialize};
use std::mem::discriminant;

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


impl<DealCards, GamePreparations, DetermineRules, Game, GameResult, Accepted> VGamePhaseGeneric<DealCards, GamePreparations, DetermineRules, Game, GameResult, Accepted> {
    pub fn matches_phase(&self, gamephase: &Self) -> bool {
        discriminant(self)==discriminant(gamephase)
    }
}

#[derive(Debug)]
pub struct SWebsocketGameResult {
    // TODO? should the members be private?
    pub gameresult: SGameResult,
    pub mapepib_confirmed: EnumMap<EPlayerIndex, bool>, // TODO? enumset
}

impl TGamePhase for SWebsocketGameResult {
    type ActivePlayerInfo = EnumMap<EPlayerIndex, bool>;
    type Finish = SAccepted;
    fn which_player_can_do_something(&self) -> Option<Self::ActivePlayerInfo> {
        let oinfallible : /*mention type to get compiler error upon change*/Option<std::convert::Infallible> = self.gameresult.which_player_can_do_something(); // TODO simplify
        verify!(oinfallible.is_none());
        if_then_some!(self.mapepib_confirmed.iter().any(|b_confirmed| !b_confirmed),
            self.mapepib_confirmed.explicit_clone()
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
    SGame,
    SWebsocketGameResult,
    SAccepted,
>;
type VGamePhaseActivePlayerInfo<'a> = VGamePhaseGeneric<
    (&'a SDealCards, <SDealCards as TGamePhase>::ActivePlayerInfo),
    (&'a SGamePreparations, <SGamePreparations as TGamePhase>::ActivePlayerInfo),
    (&'a SDetermineRules, <SDetermineRules as TGamePhase>::ActivePlayerInfo),
    (&'a SGame, <SGame as TGamePhase>::ActivePlayerInfo),
    (&'a SWebsocketGameResult, <SWebsocketGameResult as TGamePhase>::ActivePlayerInfo),
    (&'a SAccepted, <SAccepted as TGamePhase>::ActivePlayerInfo),
>;

type SActivelyPlayableRulesIdentifier = String;
fn find_rules_by_id(slcrulegroup: &[SRuleGroup], hand: SFullHand, orulesid: &Option<SActivelyPlayableRulesIdentifier>) -> Result<Option<Box<dyn TActivelyPlayableRules>>, ()> {
    allowed_rules(slcrulegroup, hand)
        .find(|orules|
            &orules.map(<dyn TActivelyPlayableRules>::to_string)==orulesid
        )
        .map(|orules| orules.map(TActivelyPlayableRulesBoxClone::box_clone)) // TODO box_clone needed?
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
                 fn_gamephaseaction(orules.map(<dyn TActivelyPlayableRules>::to_string)),
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

impl VGamePhase {
    pub fn which_player_can_do_something(&self) -> Option<VGamePhaseActivePlayerInfo> {
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

    #[allow(clippy::result_large_err)]
    pub fn action(mut self, epi: EPlayerIndex, gamephaseaction: VGamePhaseAction) -> Result<Self, /*Err contains original self*/Self> {
        let b_ok;
        match (&mut self, gamephaseaction) {
            (VGamePhase::DealCards(ref mut dealcards), VGamePhaseAction::DealCards(b_doubling)) => {
                b_ok = dealcards.announce_doubling(epi, b_doubling).is_ok();
            },
            (VGamePhase::GamePreparations(ref mut gamepreparations), VGamePhaseAction::GamePreparations(ref orulesid)) => {
                b_ok = find_rules_by_id(
                    &gamepreparations.ruleset.avecrulegroup[epi],
                    gamepreparations.fullhand(epi),
                    orulesid
                ).ok().and_then(|orules| gamepreparations.announce_game(epi, orules).ok()).is_some()
            },
            (VGamePhase::DetermineRules(ref mut determinerules), VGamePhaseAction::DetermineRules(ref orulesid)) => {
                b_ok = determinerules.which_player_can_do_something()
                    .filter(|(epi_active, _vecrulegroup)| epi==*epi_active) // TODO needed?
                    .and_then(|(epi_active, vecrulegroup)| {
                        find_rules_by_id(
                            &vecrulegroup,
                            determinerules.fullhand(verify_eq!(epi, epi_active)),
                            orulesid
                        ).ok().and_then(|orules| {
                            if let Some(rules) = orules {
                                determinerules.announce_game(epi, rules)
                            } else {
                                determinerules.resign(epi)
                            }.ok()
                        })
                    })
                    .is_some();
            },
            (VGamePhase::Game(ref mut game), VGamePhaseAction::Game(ref gameaction)) => {
                b_ok = match gameaction {
                    VGameAction::Stoss => game.stoss(epi),
                    VGameAction::Zugeben(card) => game.zugeben(*card, epi),
                }.is_ok();
            },
            (VGamePhase::GameResult(ref mut gameresult), VGamePhaseAction::GameResult(())) => {
                gameresult.mapepib_confirmed[epi] = true;
                b_ok = true;
            },
            (VGamePhase::Accepted(_), VGamePhaseAction::Accepted(_)) => {
                b_ok = true;
            },
            (_gamephase, _cmd) => {
                // TODO assert!(!self.matches_phase(&gamephaseaction));
                b_ok = false;
            },
        };
        let mut b_ok_2 = b_ok;
        if verify_eq!(b_ok, b_ok_2) {
            use VGamePhaseGeneric::*;
            while self.which_player_can_do_something().is_none() {
                fn simple_transition<R: From<VGamePhase>, GamePhase: TGamePhase>(
                    phase: GamePhase,
                    fn_ok: impl FnOnce(GamePhase::Finish) -> VGamePhase,
                    fn_err: impl FnOnce(GamePhase) -> VGamePhase,
                ) -> R {
                    phase.finish().map_or_else(fn_err, fn_ok).into()
                }
                {
                    self = match self {
                        DealCards(dealcards) => simple_transition(dealcards, GamePreparations, DealCards),
                        GamePreparations(gamepreparations) => match gamepreparations.finish() {
                            Ok(VGamePreparationsFinish::DetermineRules(determinerules)) => DetermineRules(determinerules),
                            Ok(VGamePreparationsFinish::DirectGame(game)) => Game(game),
                            Ok(VGamePreparationsFinish::Stock(gameresult)) => GameResult(SWebsocketGameResult{
                                gameresult,
                                mapepib_confirmed: EPlayerIndex::map_from_fn(|_epi| false),
                            }),
                            Err(gamepreparations) => GamePreparations(gamepreparations),
                        }
                        DetermineRules(determinerules) => simple_transition(determinerules, Game, DetermineRules),
                        Game(game) => simple_transition(
                            game,
                            |gameresult| GameResult(SWebsocketGameResult{
                                gameresult,
                                mapepib_confirmed: EPlayerIndex::map_from_fn(|_epi| false),
                            }),
                            Game
                        ),
                        GameResult(gameresult) => match gameresult.finish() {
                            Ok(accepted) => {
                                let oinfallible : Option</*mention type to get compiler error upon change*/Infallible> = accepted.which_player_can_do_something();
                                assert!(oinfallible.is_none());
                                Accepted(accepted)
                            },
                            Err(gameresult) => {
                                b_ok_2 = false;
                                GameResult(gameresult)
                            },
                        },
                        Accepted(accepted) => Accepted(accepted),
                    };
                    if let Accepted(_accepted) = &self {
                        break; // TODO can we get rid of this special case?
                    }
                }
            }
        }
        if b_ok_2 {
            Ok(self)
        } else {
            Err(self)
        }
    }
}

