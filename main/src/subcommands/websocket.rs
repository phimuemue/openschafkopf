// adapted from https://github.com/sdroege/async-tungstenite/blob/master/examples/server.rs

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use crate::util::*;
use crate::game::*;
use crate::rules::*;
use crate::rules::ruleset::{SRuleGroup, SRuleSet, VStockOrT, allowed_rules};

use futures::prelude::*;
use futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    future, pin_mut,
};
use serde::{Serialize, Deserialize};
use std::task::{Context, Poll, Waker};

use async_std::{
    net::{TcpListener, TcpStream},
    task,
};
use async_tungstenite::tungstenite::protocol::Message;
use crate::primitives::*;
use rand::prelude::*;
use itertools::Itertools;

#[derive(Debug, Serialize, Deserialize, Clone)]
enum VGamePhaseGeneric<DealCards, GamePreparations, DetermineRules, Game, GameResult> {
    DealCards(DealCards),
    GamePreparations(GamePreparations),
    DetermineRules(DetermineRules),
    Game(Game),
    GameResult(GameResult),
}

type VGamePhase = VGamePhaseGeneric<
    SDealCards,
    SGamePreparations,
    SDetermineRules,
    SGame,
    SGameResult,
>;
type VGamePhaseActivePlayerInfo<'a> = VGamePhaseGeneric<
    (&'a SDealCards, <SDealCards as TGamePhase>::ActivePlayerInfo),
    (&'a SGamePreparations, <SGamePreparations as TGamePhase>::ActivePlayerInfo),
    (&'a SDetermineRules, <SDetermineRules as TGamePhase>::ActivePlayerInfo),
    (&'a SGame, <SGame as TGamePhase>::ActivePlayerInfo),
    (&'a SGameResult, <SGameResult as TGamePhase>::ActivePlayerInfo),
>;
type SActivelyPlayableRulesIdentifier = String;
fn find_rules_by_id(slcrulegroup: &[SRuleGroup], hand: SFullHand, orulesid: &Option<SActivelyPlayableRulesIdentifier>) -> Result<Option<Box<dyn TActivelyPlayableRules>>, ()> {
    allowed_rules(slcrulegroup, hand)
        .find(|orules|
            &orules.map(TActivelyPlayableRules::to_string)==orulesid
        )
        .map(|orules| orules.map(TActivelyPlayableRulesBoxClone::box_clone)) // TODO box_clone needed?
        .ok_or(())
}

fn rules_to_gamephaseaction<'retval, 'rules : 'retval, 'hand : 'retval>(slcrulegroup: &'rules [SRuleGroup], hand: SFullHand<'hand>, fn_gamephaseaction: impl 'static + Clone + Fn(Option<SActivelyPlayableRulesIdentifier>)->VGamePhaseAction) -> impl Clone + Iterator<Item=(SActivelyPlayableRulesIdentifier, VGamePhaseAction)> + 'retval {
    allowed_rules(slcrulegroup, hand)
        .map(move |orules|
             (
                 if let Some(rules) = orules {
                     rules.to_string()
                 } else {
                     "Weiter".to_string()
                 },
                 fn_gamephaseaction(orules.map(TActivelyPlayableRules::to_string)),
             )
        )
}

#[derive(Debug, Serialize, Deserialize, Clone)]
enum VGameAction {
    Stoss,
    Zugeben(SCard),
}
type VGamePhaseAction = VGamePhaseGeneric<
    /*DealCards announce_doubling*/ /*b_doubling*/bool,
    /*GamePreparations announce_game*/Option<SActivelyPlayableRulesIdentifier>,
    /*DetermineRules*/Option<SActivelyPlayableRulesIdentifier>,
    /*Game*/VGameAction,
    /*GameResult*/(),
>;
#[derive(Serialize, Deserialize)]
enum VPlayerCmd {
    GamePhaseAction(VGamePhaseAction),
    PlayerLogin{str_player_name: String},
}

impl VGamePhase {
    fn which_player_can_do_something(&self) -> Option<VGamePhaseActivePlayerInfo> {
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

#[derive(Debug)]
struct STimeoutCmd {
    gamephaseaction: VGamePhaseAction,
    aborthandle: future::AbortHandle,
}

#[derive(Debug)]
struct SPeer {
    sockaddr: SocketAddr,
    txmsg: UnboundedSender<Message>,
    n_money: isize,
    str_name: String,
}

#[derive(Debug, Default)]
struct SActivePeer {
    opeer: Option<SPeer>,
    otimeoutcmd: Option<STimeoutCmd>,
}

#[derive(Default, Debug)]
struct SPlayers {
    mapepiopeer: EnumMap<EPlayerIndex, SActivePeer>, // active
    vecpeer: Vec<SPeer>, // inactive
}
#[derive(Debug)]
struct STable{
    players: SPlayers,
    ogamephase: Option<VGamePhase>,
    n_stock: isize, // TODO would that be better within VGamePhase?
    ruleset: SRuleSet,
}

impl STable {
    fn new(ruleset: SRuleSet) -> Self {
        Self {
            players: SPlayers::default(),
            ogamephase: None,
            n_stock: 0,
            ruleset,
        }
    }

    fn insert(&mut self, self_mutex: Arc<Mutex<Self>>, peer: SPeer) {
        match self.players.mapepiopeer
            .iter_mut()
            .find(|opeer| opeer.opeer.is_none())
        {
            Some(opeer) if self.ogamephase.is_none() => {
                assert!(opeer.opeer.is_none());
                *opeer = SActivePeer{
                    opeer: Some(peer),
                    otimeoutcmd: None,
                }
            },
            _ => {
                self.players.vecpeer.push(peer);
            }
        }
        if self.ogamephase.is_none()
            && self.players.mapepiopeer
                .iter()
                .all(|opeer| opeer.opeer.is_some())
        {
            self.ogamephase = Some(VGamePhase::DealCards(SDealCards::new(
                self.ruleset.clone(),
                self.n_stock,
            )));
            self.on_incoming_message(
                self_mutex,
                /*oepi*/None,
                /*ogamephaseaction*/None,
            ); // To trigger game logic. TODO beautify instead of dummy msg.
        }
    }

    fn remove(&mut self, sockaddr: &SocketAddr) {
        for epi in EPlayerIndex::values() {
            if self.players.mapepiopeer[epi].opeer.as_ref().map(|peer| peer.sockaddr)==Some(*sockaddr) {
                self.players.mapepiopeer[epi].opeer = None;
            }
        }
        self.players.vecpeer.retain(|peer| peer.sockaddr!=*sockaddr);
    }
}

impl SPlayers {
    fn for_each(
        &mut self,
        oslcstich: Option<&[SStich]>,
        orules: Option<&dyn TRules>,
        f_cards: impl Fn(EPlayerIndex) -> Vec<SCard>,
        mut f_active: impl FnMut(EPlayerIndex, &mut Option<STimeoutCmd>)->VMessage,
        mut f_inactive: impl FnMut(&mut SPeer)->VMessage,
        oepi_timeout: Option<EPlayerIndex>,
    ) {
        let mapepistr_name = self.mapepiopeer.map(|activepeer| // TODO can we avoid temporary storage?
            activepeer
                .opeer
                .as_ref()
                .map(|peer| peer.str_name.clone())
                .unwrap_or("<BOT>".to_string())
        );
        let communicate = |oepi: Option<EPlayerIndex>, veccard: Vec<SCard>, msg, peer: &mut SPeer| {
            let i_epi_relative = oepi.unwrap_or(EPlayerIndex::EPI0).to_usize();
            let playerindex_server_to_client = |epi: EPlayerIndex| {
                epi.wrapping_add(EPlayerIndex::SIZE - i_epi_relative)
            };
            let playerindex_client_to_server = |epi: EPlayerIndex| {
                epi.wrapping_add(i_epi_relative)
            };
            #[derive(Serialize)]
            struct SDisplayedStichPrev {
                mapepistr_card: [String; EPlayerIndex::SIZE],
            }
            #[derive(Serialize)]
            struct SDisplayedStichCurrent {
                epi_first: EPlayerIndex, // also denotes winner index of ostichprev
                vecstr_card: Vec<String>,
            }
            #[derive(Serialize)]
            struct SDisplayedStichs {
                stichcurrent: SDisplayedStichCurrent,
                ostichprev: Option<SDisplayedStichPrev>,
            }
            #[derive(new, Serialize)]
            struct SSiteState {
                vectplstrstr_caption_message_zugeben: Vec<(String, VGamePhaseAction)>,
                msg: VMessage,
                odisplayedstichs: Option<SDisplayedStichs>,
                mapepistr: [String; EPlayerIndex::SIZE],
                otplepistr_rules: Option<(EPlayerIndex, String)>,
                oepi_timeout: Option<EPlayerIndex>,
            }
            let card_in_stich = |stich: &SStich, epi| {
                stich.get(playerindex_client_to_server(epi)).map(SCard::to_string)
            };
            unwrap!(peer.txmsg.unbounded_send(
                unwrap!(serde_json::to_string(&SSiteState::new(
                    veccard.into_iter()
                        .map(|card| (card.to_string(), VGamePhaseAction::Game(VGameAction::Zugeben(card))))
                        .collect::<Vec<_>>(),
                    msg,
                    /*odisplayedstichs*/ oslcstich.and_then(|slcstich| {
                        slcstich
                            .split_last()
                            .map(|(stich_current, slcstich_up_to_last)| {
                                SDisplayedStichs{
                                    stichcurrent: SDisplayedStichCurrent {
                                        epi_first: playerindex_server_to_client(stich_current.first_playerindex()),
                                        vecstr_card: stich_current
                                            .iter()
                                            .map(|(_epi, card)| card.to_string())
                                            .collect(),
                                    },
                                    ostichprev: slcstich_up_to_last
                                        .last()
                                        .map(|stich_prev| SDisplayedStichPrev{
                                            mapepistr_card: EPlayerIndex
                                                ::map_from_fn(|epi| unwrap!(card_in_stich(stich_prev, epi)))
                                                .into_raw()
                                        })
                                }
                            })
                    }),
                    EPlayerIndex::map_from_fn(|epi| 
                        format!("{} ({})",
                            mapepistr_name[playerindex_client_to_server(epi)],
                            playerindex_client_to_server(epi).to_usize(),
                        )
                    ).into_raw(),
                    orules.map(|rules| (
                        playerindex_server_to_client(rules.playerindex().unwrap_or(EPlayerIndex::EPI3)), // geber designates rules if no active
                        format!("{}", rules),
                    )),
                    oepi_timeout.map(playerindex_server_to_client),
                ))).into()
            ));
        };
        for epi in EPlayerIndex::values() {
            let ref mut activepeer = self.mapepiopeer[epi];
            let msg = f_active(epi, &mut activepeer.otimeoutcmd);
            if let Some(ref mut peer) = activepeer.opeer.as_mut() {
                let mut veccard = f_cards(epi);
                if let Some(rules) = orules {
                    rules.sort_cards_first_trumpf_then_farbe(&mut veccard);
                } else {
                    rulesramsch::SRulesRamsch::new( // TODO rules dummy is ugly
                        /*n_price*/0, // irrelevant
                        rulesramsch::VDurchmarsch::None, // irrelevant
                    ).sort_cards_first_trumpf_then_farbe(&mut veccard);
                }
                communicate(Some(epi), veccard, msg, peer);
            }
        }
        for peer in self.vecpeer.iter_mut() {
            let msg = f_inactive(peer);
            communicate(None, vec![], msg, peer);
        }
    }
}

impl STable {
    fn on_incoming_message(&mut self, /*TODO avoid this parameter*/self_mutex: Arc<Mutex<Self>>, oepi: Option<EPlayerIndex>, ogamephaseaction: Option<VGamePhaseAction>) {
        println!("on_incoming_message({:?}, {:?})", oepi, ogamephaseaction);
        if self.ogamephase.is_some() {
            if let Some(epi) = oepi {
                fn handle_err<T, E: std::fmt::Display>(res: Result<T, E>) {
                    match res {
                        Ok(_) => {},
                        Err(e) => println!("Error {}", e),
                    };
                }
                if let Some(gamephaseaction) = ogamephaseaction {
                    use std::mem::discriminant;
                    match self.players.mapepiopeer[epi].otimeoutcmd.as_ref() {
                        None => (),
                        Some(timeoutcmd) => {
                            if discriminant(&gamephaseaction)==discriminant(&timeoutcmd.gamephaseaction) {
                                timeoutcmd.aborthandle.abort();
                                self.players.mapepiopeer[epi].otimeoutcmd = None;
                            }
                        },
                    }
                    if let Some(ref mut gamephase) = debug_verify!(self.ogamephase.as_mut()) {
                        match (gamephase, gamephaseaction) {
                            (VGamePhase::DealCards(ref mut dealcards), VGamePhaseAction::DealCards(b_doubling)) => {
                                handle_err(dealcards.announce_doubling(epi, b_doubling));
                            },
                            (VGamePhase::GamePreparations(ref mut gamepreparations), VGamePhaseAction::GamePreparations(ref orulesid)) => {
                                if let Ok(orules) = find_rules_by_id(
                                    &gamepreparations.ruleset.avecrulegroup[epi],
                                    gamepreparations.fullhand(epi),
                                    orulesid
                                ) {
                                    handle_err(gamepreparations.announce_game(epi, orules));
                                }
                            },
                            (VGamePhase::DetermineRules(ref mut determinerules), VGamePhaseAction::DetermineRules(ref orulesid)) => {
                                if let Some((_epi_active, vecrulegroup)) = determinerules.which_player_can_do_something() {
                                    if let Ok(orules) = find_rules_by_id(
                                        &vecrulegroup,
                                        determinerules.fullhand(epi),
                                        orulesid
                                    ) {
                                        handle_err(if let Some(rules) = orules {
                                            determinerules.announce_game(epi, rules)
                                        } else {
                                            determinerules.resign(epi)
                                        });
                                    }
                                }
                            },
                            (VGamePhase::Game(ref mut game), VGamePhaseAction::Game(ref gameaction)) => {
                                handle_err(match gameaction {
                                    VGameAction::Stoss => game.stoss(epi),
                                    VGameAction::Zugeben(card) => game.zugeben(*card, epi),
                                });
                            },
                            (VGamePhase::GameResult(gameresult), VGamePhaseAction::GameResult(())) => {
                                gameresult.confirm(epi);
                            },
                            (_gamephase, _cmd) => {
                            },
                        };
                    }
                }
            }
            while self.ogamephase.as_ref().map_or(false, |gamephase| gamephase.which_player_can_do_something().is_none()) {
                use VGamePhaseGeneric::*;
                fn next_game(table: &mut STable) -> Option<VGamePhase> {
                    /*           E2
                     * E1                      E3
                     *    E0 SN SN-1 ... S1 S0
                     *
                     * E0 E1 E2 E3 [S0 S1 S2 ... SN]
                     * E1 E2 E3 S0 [S1 S2 ... SN E0]
                     * E2 E3 S0 S1 [S2 ... SN E0 E1]
                     */
                    // Players: E0 E1 E2 E3 [S0 S1 S2 ... SN] (S0 is longest waiting inactive player)
                    table.players.mapepiopeer.as_raw_mut().rotate_left(1);
                    // Players: E1 E2 E3 E0 [S0 S1 S2 ... SN]
                    if let Some(peer_epi3) = table.players.mapepiopeer[EPlayerIndex::EPI3].opeer.take() {
                        table.players.vecpeer.push(peer_epi3);
                    }
                    // Players: E1 E2 E3 -- [S0 S1 S2 ... SN E0] (E1, E2, E3 may be None)
                    // Fill up players one after another
                    assert!(table.players.mapepiopeer[EPlayerIndex::EPI3].opeer.is_none());
                    for epi in EPlayerIndex::values() {
                        if table.players.mapepiopeer[epi].opeer.is_none() && !table.players.vecpeer.is_empty() {
                            table.players.mapepiopeer[epi].opeer = Some(table.players.vecpeer.remove(0));
                        }
                    }
                    // Players: E1 E2 E3 S0 [S1 S2 ... SN E0] (E1, E2, E3 may be None)
                    if_then_some!(table.players.mapepiopeer.iter().all(|activepeer| activepeer.opeer.is_some()),
                        VGamePhase::DealCards(SDealCards::new(table.ruleset.clone(), table.n_stock))
                    )
                    // TODO should we clear timeouts?
                };
                fn simple_transition<GamePhase: TGamePhase>(
                    phase: GamePhase,
                    fn_ok: impl FnOnce(GamePhase::Finish) -> VGamePhase,
                    fn_err: impl FnOnce(GamePhase) -> VGamePhase,
                ) -> Option<VGamePhase> {
                    Some(phase.finish().map_or_else(fn_err, fn_ok))
                }
                if let Some(gamephase) = self.ogamephase.take() {
                    self.ogamephase = match gamephase {
                        DealCards(dealcards) => simple_transition(dealcards, GamePreparations, DealCards),
                        GamePreparations(gamepreparations) => match gamepreparations.finish() {
                            Ok(VGamePreparationsFinish::DetermineRules(determinerules)) => Some(DetermineRules(determinerules)),
                            Ok(VGamePreparationsFinish::DirectGame(game)) => Some(Game(game)),
                            Ok(VGamePreparationsFinish::Stock(gameresult)) => {
                                let mapepiopeer = &mut self.players.mapepiopeer;
                                gameresult.apply_payout(&mut self.n_stock, |epi, n_payout| {
                                    if let Some(ref mut peer) = mapepiopeer[epi].opeer {
                                        peer.n_money += n_payout;
                                    }
                                });
                                next_game(self)
                            },
                            Err(gamepreparations) => Some(GamePreparations(gamepreparations)),
                        }
                        DetermineRules(determinerules) => simple_transition(determinerules, Game, DetermineRules),
                        Game(game) => simple_transition(game, GameResult, Game),
                        GameResult(gameresult) => match gameresult.finish() {
                            Ok(gameresult) => {
                                let mapepiopeer = &mut self.players.mapepiopeer;
                                gameresult.apply_payout(&mut self.n_stock, |epi, n_payout| {
                                    if let Some(ref mut peer) = mapepiopeer[epi].opeer {
                                        peer.n_money += n_payout;
                                    }
                                });
                                next_game(self)
                            },
                            Err(gameresult) => Some(GameResult(gameresult)),
                        },
                    };
                }
            }
            if let Some(ref gamephase) = self.ogamephase {
                if let Some(whichplayercandosomething) = verify!(gamephase.which_player_can_do_something()) {
                    fn ask_with_timeout(
                        otimeoutcmd: &mut Option<STimeoutCmd>,
                        epi: EPlayerIndex,
                        str_question: String,
                        itgamephaseaction: impl Iterator<Item=(String, VGamePhaseAction)>,
                        table_mutex: Arc<Mutex<STable>>,
                        gamephaseaction_timeout: VGamePhaseAction,
                    ) -> VMessage {
                        let (timerfuture, aborthandle) = future::abortable(STimerFuture::new(
                            /*n_secs*/2,
                            table_mutex,
                            epi,
                        ));
                        assert!({
                            use std::mem::discriminant;
                            otimeoutcmd.as_ref().map_or(true, |timeoutcmd|
                                discriminant(&timeoutcmd.gamephaseaction)==discriminant(&gamephaseaction_timeout)
                            )
                        }); // only one active timeout cmd
                        *otimeoutcmd = Some(STimeoutCmd{
                            gamephaseaction: gamephaseaction_timeout,
                            aborthandle,
                        });
                        task::spawn(timerfuture);
                        VMessage::Ask{
                            str_question,
                            vecstrgamephaseaction: itgamephaseaction.collect(),
                        }
                    }
                    use VGamePhaseGeneric::*;
                    match whichplayercandosomething {
                        DealCards((dealcards, epi_doubling)) => {
                            self.players.for_each(
                                /*oslcstich*/None,
                                None,
                                |epi| dealcards.first_hand_for(epi).into(),
                                |epi, otimeoutcmd| {
                                    if epi_doubling==epi {
                                        ask_with_timeout(
                                            otimeoutcmd,
                                            epi_doubling,
                                            "Doppeln?".into(),
                                            [(true, "Doppeln"), (false, "Nicht doppeln")]
                                                .iter()
                                                .map(|(b_doubling, str_doubling)| 
                                                    (str_doubling.to_string(), VGamePhaseAction::DealCards(*b_doubling))
                                                ),
                                            self_mutex.clone(),
                                            VGamePhaseAction::DealCards(/*b_doubling*/false),
                                        )
                                    } else {
                                        VMessage::Info(format!("Asking {:?} for doubling", epi_doubling))
                                    }
                                },
                                |_peer| VMessage::Info(format!("Asking {:?} for doubling", epi_doubling)),
                                Some(epi_doubling),
                            );
                        },
                        GamePreparations((gamepreparations, epi_announce_game)) => {
                            self.players.for_each(
                                /*oslcstich*/None,
                                None,
                                |epi| gamepreparations.fullhand(epi).get().cards().to_vec(),
                                |epi, otimeoutcmd| {
                                    if epi_announce_game==epi {
                                        let itgamephaseaction_rules = rules_to_gamephaseaction(
                                            &gamepreparations.ruleset.avecrulegroup[epi_announce_game],
                                            gamepreparations.fullhand(epi_announce_game),
                                            VGamePhaseAction::GamePreparations,
                                        );
                                        let gamephaseaction_rules_default = unwrap!(itgamephaseaction_rules.clone().next()).1.clone();
                                        ask_with_timeout(
                                            otimeoutcmd,
                                            epi_announce_game,
                                            format!("Du bist an {}. Stelle. {}",
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
                                            itgamephaseaction_rules,
                                            self_mutex.clone(),
                                            gamephaseaction_rules_default,
                                        )
                                    } else {
                                        VMessage::Info(format!("Asking {:?} for game", epi_announce_game))
                                    }
                                },
                                |_peer| VMessage::Info(format!("Asking {:?} for game", epi_announce_game)),
                                Some(epi_announce_game),
                            );
                        },
                        DetermineRules((determinerules, (epi_determine, vecrulegroup))) => {
                            self.players.for_each(
                                /*oslcstich*/None,
                                None,
                                |epi| determinerules.fullhand(epi).get().cards().to_vec(),
                                |epi, otimeoutcmd| {
                                    if epi_determine==epi {
                                        let itgamephaseaction_rules = rules_to_gamephaseaction(
                                            &vecrulegroup,
                                            determinerules.fullhand(epi_determine),
                                            VGamePhaseAction::DetermineRules,
                                        );
                                        let gamephaseaction_rules_default = unwrap!(itgamephaseaction_rules.clone().next()).1.clone();
                                        ask_with_timeout(
                                            otimeoutcmd,
                                            epi_determine,
                                            format!(
                                                "Du bist an {}. Stelle. Von {}. Stelle wird {} geboten. Spielst Du etwas staerkeres?", // TODO umlaut-tactics?
                                                epi.to_usize() + 1, // EPlayerIndex is 0-based
                                                determinerules.pairepirules_current_bid.0.to_usize() + 1, // EPlayerIndex is 0-based
                                                determinerules.pairepirules_current_bid.1.to_string(),
                                            ),
                                            itgamephaseaction_rules,
                                            self_mutex.clone(),
                                            gamephaseaction_rules_default,
                                        )
                                    } else {
                                        VMessage::Info(format!("Re-Asking {:?} for game", epi_determine))
                                    }
                                },
                                |_peer| VMessage::Info(format!("Re-Asking {:?} for game", epi_determine)),
                                Some(epi_determine),
                            );
                        },
                        Game((game, (epi_card, vecepi_stoss))) => {
                            self.players.for_each(
                                Some(game.stichseq.visible_stichs()),
                                Some(game.rules.as_ref()),
                                |epi| game.ahand[epi].cards().to_vec(),
                                |epi, otimeoutcmd| {
                                    let ostrgamephaseaction = if_then_some!(vecepi_stoss.contains(&epi),
                                        ("Stoss".into(), VGamePhaseAction::Game(VGameAction::Stoss))
                                    );
                                    if epi_card==epi {
                                        ask_with_timeout(
                                            otimeoutcmd,
                                            epi_card,
                                            "".into(),
                                            ostrgamephaseaction.into_iter(),
                                            self_mutex.clone(),
                                            VGamePhaseAction::Game(VGameAction::Zugeben(
                                                *unwrap!(game.rules.all_allowed_cards(
                                                    &game.stichseq,
                                                    &game.ahand[epi_card],
                                                ).choose(&mut rand::thread_rng()))
                                            )),
                                        )
                                    } else if let Some(strgamephaseaction) = ostrgamephaseaction {
                                        VMessage::Ask{
                                            str_question: "".into(),
                                            vecstrgamephaseaction: vec![strgamephaseaction],
                                        }
                                    } else {
                                        VMessage::Info(format!("Asking {:?} for card", epi_card))
                                    }
                                },
                                |_peer| VMessage::Info(format!("Asking {:?} for card", epi_card)),
                                Some(epi_card),
                            );
                        },
                        GameResult((gameresult, mapepib_confirmed)) => {
                            self.players.for_each(
                                /*oslcstich*/if_then_some!(let VStockOrT::OrT(ref game) = gameresult.stockorgame,
                                    game.stichseq.completed_stichs()
                                ),
                                if_then_some!(let VStockOrT::OrT(ref game) = gameresult.stockorgame,
                                    game.rules.as_ref()
                                ),
                                |_epi| vec![],
                                |epi, otimeoutcmd| {
                                    if !mapepib_confirmed[epi] {
                                        ask_with_timeout(
                                            otimeoutcmd,
                                            epi,
                                            format!("Spiel beendet. {}", if gameresult.an_payout[epi] < 0 {
                                                format!("Verlust: {}", -gameresult.an_payout[epi])
                                            } else {
                                                format!("Gewinn: {}", gameresult.an_payout[epi])
                                            }),
                                            std::iter::once(("Ok".into(), VGamePhaseAction::GameResult(()))),
                                            self_mutex.clone(),
                                            VGamePhaseAction::GameResult(()),
                                        )
                                    } else {
                                        VMessage::Info("Game finished".into())
                                    }
                                },
                                |_peer| VMessage::Info("Game finished".into()),
                                None,
                            );
                        },
                    }
                }
            }
        } else {
            self.players.for_each(
                /*oslcstich*/None,
                None,
                |_epi| vec![],
                |_oepi, _otimeoutcmd| VMessage::Info("Waiting for more players.".into()),
                |_peer| VMessage::Info("Waiting for more players.".into()),
                None,
            );
        }
    }
}

#[derive(Serialize)]
enum VMessage {
    Info(String),
    Ask{
        str_question: String,
        vecstrgamephaseaction: Vec<(String, VGamePhaseAction)>,
    },
}

// timer adapted from https://rust-lang.github.io/async-book/02_execution/03_wakeups.html
struct STimerFuture {
    state: Arc<Mutex<STimerFutureState>>,
    table: Arc<Mutex<STable>>,
    epi: EPlayerIndex,
}

#[derive(Debug)]
struct STimerFutureState {
    b_completed: bool,
    owaker: Option<Waker>,
}

impl Future for STimerFuture {
    type Output = ();
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = unwrap!(self.state.lock());
        if state.b_completed {
            let table_mutex = self.table.clone();
            let mut table = unwrap!(self.table.lock());
            if let Some(timeoutcmd) = table.players.mapepiopeer[self.epi].otimeoutcmd.take() {
                table.on_incoming_message(table_mutex, Some(self.epi), Some(timeoutcmd.gamephaseaction));
            }
            Poll::Ready(())
        } else {
            state.owaker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl STimerFuture {
    fn new(n_secs: u64, table: Arc<Mutex<STable>>, epi: EPlayerIndex) -> Self {
        let state = Arc::new(Mutex::new(STimerFutureState {
            b_completed: false,
            owaker: None,
        }));
        let thread_shared_state = state.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::new(n_secs, /*nanos*/0));
            let mut state = unwrap!(thread_shared_state.lock());
            state.b_completed = true;
            if let Some(waker) = state.owaker.take() {
                waker.wake()
            }
        });
        Self {state, table, epi}
    }
}

async fn handle_connection(table: Arc<Mutex<STable>>, tcpstream: TcpStream, sockaddr: SocketAddr) {
    println!("Incoming TCP connection from: {}", sockaddr);
    let wsstream = unwrap!(async_tungstenite::accept_async(tcpstream).await);
    println!("WebSocket connection established: {}", sockaddr);
    // Insert the write part of this peer to the peer map.
    let (txmsg, rxmsg) = unbounded();
    let table_mutex = table.clone();
    unwrap!(table.lock()).insert(table_mutex.clone(), SPeer{
        sockaddr,
        txmsg,
        n_money: 0,
        str_name: "<Name>".into(), // TODO can we initialize it right away?
    });
    let (sink_ws_out, stream_ws_in) = wsstream.split();
    let broadcast_incoming = stream_ws_in
        .try_filter(|msg| {
            // Broadcasting a Close message from one client
            // will close the other clients.
            future::ready(!msg.is_close())
        })
        .try_for_each(|msg| {
            let str_msg = unwrap!(msg.to_text());
            let mut table = unwrap!(table.lock());
            let oepi = EPlayerIndex::values()
                .find(|epi| table.players.mapepiopeer[*epi].opeer.as_ref().map(|peer| peer.sockaddr)==Some(sockaddr));
            println!(
                "Received a message from {} ({:?}): {}",
                sockaddr,
                oepi,
                str_msg,
            );
            use VPlayerCmd::*;
            match serde_json::from_str(str_msg) {
                Ok(GamePhaseAction(gamephaseaction)) => table.on_incoming_message(table_mutex.clone(), oepi, Some(gamephaseaction)),
                Ok(PlayerLogin{str_player_name}) => {
                    if let Some(ref epi)=oepi {
                        if let Some(ref mut peer) = table.players.mapepiopeer[*epi].opeer {
                            peer.str_name = str_player_name;
                        }
                    } else if let Some(ref mut peer) = table.players.vecpeer.iter_mut().find(|peer| peer.sockaddr==sockaddr) {
                        peer.str_name = str_player_name;
                    }
                },
                Err(e) => println!("Error: {}", e),
            }
            future::ok(())
        });
    let receive_from_others = rxmsg.map(Ok).forward(sink_ws_out);
    pin_mut!(broadcast_incoming, receive_from_others); // TODO Is this really needed?
    future::select(broadcast_incoming, receive_from_others).await;
    println!("{} disconnected", &sockaddr);
    unwrap!(table.lock()).remove(&sockaddr);
}

async fn internal_run(ruleset: SRuleSet) -> Result<(), Error> {
    let str_addr = "127.0.0.1:8080";
    let table = Arc::new(Mutex::new(STable::new(ruleset)));
    // Create the event loop and TCP listener we'll accept connections on.
    let listener = unwrap!(TcpListener::bind(&str_addr).await);
    println!("Listening on: {}", str_addr);
    // Let's spawn the handling of each connection in a separate task.
    while let Ok((tcpstream, sockaddr)) = listener.accept().await {
        task::spawn(handle_connection(table.clone(), tcpstream, sockaddr));
    }
    Ok(())
}

pub fn run(ruleset: SRuleSet) -> Result<(), Error> {
    task::block_on(internal_run(ruleset))
}

