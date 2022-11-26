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
pub enum VGamePhaseGeneric<DealCards, GamePreparations, DetermineRules, Game, GameResult> {
    DealCards(DealCards),
    GamePreparations(GamePreparations),
    DetermineRules(DetermineRules),
    Game(Game),
    GameResult(GameResult),
}


impl<DealCards, GamePreparations, DetermineRules, Game, GameResult> VGamePhaseGeneric<DealCards, GamePreparations, DetermineRules, Game, GameResult> {
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
    type Finish = SWebsocketGameResult;
    fn which_player_can_do_something(&self) -> Option<Self::ActivePlayerInfo> {
        let oinfallible : /*mention type to get compiler error upon change*/Option<std::convert::Infallible> = self.gameresult.which_player_can_do_something(); // TODO simplify
        verify!(oinfallible.is_none());
        if_then_some!(self.mapepib_confirmed.iter().any(|b_confirmed| !b_confirmed),
            self.mapepib_confirmed.explicit_clone()
        )
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
>;
type VGamePhaseActivePlayerInfo<'a> = VGamePhaseGeneric<
    (&'a SDealCards, <SDealCards as TGamePhase>::ActivePlayerInfo),
    (&'a SGamePreparations, <SGamePreparations as TGamePhase>::ActivePlayerInfo),
    (&'a SDetermineRules, <SDetermineRules as TGamePhase>::ActivePlayerInfo),
    (&'a SGame, <SGame as TGamePhase>::ActivePlayerInfo),
    (&'a SWebsocketGameResult, <SWebsocketGameResult as TGamePhase>::ActivePlayerInfo),
>;

type SActivelyPlayableRulesIdentifier = String;
pub fn find_rules_by_id(slcrulegroup: &[SRuleGroup], hand: SFullHand, orulesid: &Option<SActivelyPlayableRulesIdentifier>) -> Result<Option<Box<dyn TActivelyPlayableRules>>, ()> {
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
        }
    }
}

