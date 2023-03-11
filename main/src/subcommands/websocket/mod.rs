// adapted from https://github.com/sdroege/async-tungstenite/blob/master/examples/server.rs

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    pin::Pin,
    thread::{sleep, spawn},
    time::Duration,
};
use crate::{
    util::*,
    game::*,
    rules::{*, trumpfdecider::TTrumpfDecider},
    rules::ruleset::{SRuleSet, VStockOrT},
    primitives::*,
};
use futures::{
    prelude::*,
    channel::mpsc::{unbounded, UnboundedSender},
    future,
};
use serde::{Serialize, Deserialize};
use async_std::{
    net::{TcpListener, TcpStream},
    task,
};
use async_tungstenite::{
    accept_async,
    tungstenite::protocol::Message,
};
use rand::{
    thread_rng,
    prelude::*,
};
use itertools::Itertools;

mod gamephase;
use gamephase::{
    Infallible,
    VGamePhase,
    VGamePhaseAction,
    VGameAction,
    VGamePhaseGeneric,
    rules_to_gamephaseaction,
};

pub fn subcommand(str_subcommand: &'static str) -> clap::Command {
    use super::shared_args::*;
    clap::Command::new(str_subcommand)
        .about("Play in the browser")
        .arg(ruleset_arg())
}

#[derive(Serialize, Deserialize)]
enum VPlayerCmd {
    GamePhaseAction(VGamePhaseAction),
    PlayerLogin{str_player_name: String},
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


#[derive(new)]
struct STimeoutAction {
    epi: EPlayerIndex,
    gamephaseaction_timeout: VGamePhaseAction,
}

struct SSendToPlayers<'game> {
    slcstich: &'game [SStich],
    orules: Option<&'game dyn TRules>,
    mapepiveccard: EnumMap<EPlayerIndex, Vec<ECard>>,
    msg_inactive: VMessage,
}

impl<'game> SSendToPlayers<'game> {
    fn new<Card: TMoveOrClone<ECard>, ItCard: IntoIterator<Item=Card>> (
        slcstich: &'game [SStich],
        orules: Option<&'game dyn TRules>,
        fn_cards: impl Fn(EPlayerIndex)->ItCard,
        msg_inactive: VMessage,
    ) -> Self {
        Self {
            slcstich,
            orules,
            mapepiveccard: EPlayerIndex::map_from_fn(|epi| fn_cards(epi).into_iter().map(TMoveOrClone::move_or_clone).collect()),
            msg_inactive,
        }
    }
}

impl SPlayers {
    fn for_each(
        &mut self,
        sendtoplayers: &SSendToPlayers,
        mut f_active: impl FnMut(EPlayerIndex, &mut Option<STimeoutCmd>)->VMessage,
        oepi_timeout: Option<EPlayerIndex>,
    ) {
        let mapepistr_name = self.mapepiopeer.map(|activepeer| // TODO can we avoid temporary storage?
            activepeer
                .opeer
                .as_ref()
                .map(|peer| peer.str_name.clone())
                .unwrap_or_else(||"<BOT>".to_string())
        );
        let communicate = |oepi: Option<EPlayerIndex>, veccard: Vec<ECard>, msg, peer: &mut SPeer| {
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
                stich.get(playerindex_client_to_server(epi)).map(ECard::to_string)
            };
            unwrap!(peer.txmsg.unbounded_send(
                unwrap!(serde_json::to_string(&SSiteState::new(
                    veccard.into_iter()
                        .map(|card| (card.to_string(), VGamePhaseAction::Game(VGameAction::Zugeben(card))))
                        .collect::<Vec<_>>(),
                    msg,
                    /*odisplayedstichs*/sendtoplayers.slcstich
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
                        }),
                    EPlayerIndex::map_from_fn(|epi| 
                        format!("{} ({})",
                            mapepistr_name[playerindex_client_to_server(epi)],
                            playerindex_client_to_server(epi).to_usize(),
                        )
                    ).into_raw(),
                    sendtoplayers.orules.map(|rules| (
                        playerindex_server_to_client(rules.playerindex().unwrap_or(EPlayerIndex::EPI3)), // geber designates rules if no active
                        format!("{}", rules),
                    )),
                    oepi_timeout.map(playerindex_server_to_client),
                ))).into()
            ));
        };
        for epi in EPlayerIndex::values() {
            let activepeer = &mut self.mapepiopeer[epi];
            let msg = f_active(epi, &mut activepeer.otimeoutcmd);
            if let Some(ref mut peer) = activepeer.opeer.as_mut() {
                let mut veccard = sendtoplayers.mapepiveccard[epi].clone(); // TODO? avoid clone
                if let Some(rules) = sendtoplayers.orules {
                    rules.sort_cards_first_trumpf_then_farbe(&mut veccard);
                } else {
                    rulesrufspiel::STrumpfDeciderRufspiel::default()
                        .sort_cards_first_trumpf_then_farbe(&mut veccard)
                }
                communicate(Some(epi), veccard, msg, peer);
            }
        }
        for peer in self.vecpeer.iter_mut() {
            communicate(None, vec![], /*msg*/sendtoplayers.msg_inactive.clone(/*TODO? needed?*/), peer);
        }
    }
}

impl STable {
    fn on_incoming_message(&mut self, /*TODO avoid this parameter*/self_mutex: Arc<Mutex<Self>>, oepi: Option<EPlayerIndex>, ogamephaseaction: Option<VGamePhaseAction>) {
        println!("on_incoming_message({:?}, {:?})", oepi, ogamephaseaction);
        if self.ogamephase.is_some() {
            if let (Some(epi), Some(gamephaseaction)) = (oepi, ogamephaseaction) {
                self.ogamephase = match unwrap!(self.ogamephase.take()).action(epi, gamephaseaction.clone()) {
                    Ok(gamephase) => {
                        if let Some(timeoutcmd) = &self.players.mapepiopeer[epi].otimeoutcmd {
                            if gamephaseaction.matches_phase(&timeoutcmd.gamephaseaction) {
                                timeoutcmd.aborthandle.abort();
                                self.players.mapepiopeer[epi].otimeoutcmd = None;
                            }
                        }
                        match gamephase {
                            VGamePhase::GameResult(gameresult) => {
                                gameresult.gameresult.clone(/*TODO avoid clone (and force apply_payout on construction?)*/).apply_payout(&mut self.n_stock, |epi, n_payout| {
                                    if let Some(ref mut peer) = &mut self.players.mapepiopeer[epi].opeer {
                                        peer.n_money += n_payout;
                                    }
                                });
                                Some(VGamePhase::GameResult(gameresult))
                            },
                            VGamePhase::Accepted(_accepted) => {
                                // advance to next game
                                /*           E2
                                 * E1                      E3
                                 *    E0 SN SN-1 ... S1 S0
                                 *
                                 * E0 E1 E2 E3 [S0 S1 S2 ... SN]
                                 * E1 E2 E3 S0 [S1 S2 ... SN E0]
                                 * E2 E3 S0 S1 [S2 ... SN E0 E1]
                                 */
                                // Players: E0 E1 E2 E3 [S0 S1 S2 ... SN] (S0 is longest waiting inactive player)
                                let mapepiopeer = &mut self.players.mapepiopeer;
                                mapepiopeer.as_raw_mut().rotate_left(1);
                                // Players: E1 E2 E3 E0 [S0 S1 S2 ... SN]
                                if let Some(peer_epi3) = mapepiopeer[EPlayerIndex::EPI3].opeer.take() {
                                    self.players.vecpeer.push(peer_epi3);
                                }
                                // Players: E1 E2 E3 -- [S0 S1 S2 ... SN E0] (E1, E2, E3 may be None)
                                // Fill up players one after another
                                assert!(mapepiopeer[EPlayerIndex::EPI3].opeer.is_none());
                                for epi in EPlayerIndex::values() {
                                    if mapepiopeer[epi].opeer.is_none() && !self.players.vecpeer.is_empty() {
                                        mapepiopeer[epi].opeer = Some(self.players.vecpeer.remove(0));
                                    }
                                }
                                // Players: E1 E2 E3 S0 [S1 S2 ... SN E0] (E1, E2, E3 may be None)
                                // TODO should we clear timeouts?
                                if_then_some!(mapepiopeer.iter().all(|activepeer| activepeer.opeer.is_some()),
                                    VGamePhase::DealCards(SDealCards::new(self.ruleset.clone(), self.n_stock))
                                )
                            },
                            gamephase => {
                                Some(gamephase)
                            },
                        }
                    },
                    Err(gamephase) => {
                        println!("Error:\n{:?}\n{:?}", gamephase, gamephaseaction);
                        Some(gamephase)
                    },
                }
            }
            if let Some(ref gamephase) = self.ogamephase {
                if let Some(whichplayercandosomething) = verify!(gamephase.which_player_can_do_something()) {
                    fn register_timeout(
                        timeoutaction: STimeoutAction,
                        otimeoutcmd: &mut Option<STimeoutCmd>,
                        table_mutex: Arc<Mutex<STable>>,
                    ) {
                        let (timerfuture, aborthandle) = future::abortable(STimerFuture::new(
                            /*n_secs*/2,
                            table_mutex,
                            timeoutaction.epi,
                        ));
                        assert!(otimeoutcmd.as_ref().map_or(true, |timeoutcmd|
                            timeoutcmd.gamephaseaction.matches_phase(&timeoutaction.gamephaseaction_timeout)
                        )); // only one active timeout cmd
                        *otimeoutcmd = Some(STimeoutCmd{
                            gamephaseaction: timeoutaction.gamephaseaction_timeout,
                            aborthandle,
                        });
                        task::spawn(timerfuture);
                    }
                    use VGamePhaseGeneric::*;
                    match whichplayercandosomething {
                        DealCards((dealcards, epi_doubling)) => {
                            self.players.for_each(
                                &SSendToPlayers::new(
                                    /*slcstich*/&[],
                                    /*orules*/None,
                                    |epi| dealcards.first_hand_for(epi),
                                    /*msg_inactive*/VMessage::Info(format!("Asking {:?} for doubling", epi_doubling)),
                                ),
                                |epi, otimeoutcmd| {
                                    if epi_doubling==epi {
                                        register_timeout(
                                            STimeoutAction::new(
                                                epi_doubling,
                                                VGamePhaseAction::DealCards(/*b_doubling*/false),
                                            ),
                                            otimeoutcmd,
                                            self_mutex.clone(),
                                        );
                                        VMessage::Ask{
                                            str_question: "Doppeln".into(),
                                            vecstrgamephaseaction: [(true, "Doppeln"), (false, "Nicht doppeln")]
                                                .into_iter()
                                                .map(|(b_doubling, str_doubling)| 
                                                    (str_doubling.to_string(), VGamePhaseAction::DealCards(b_doubling))
                                                )
                                                .collect(),
                                        }
                                    } else {
                                        VMessage::Info(format!("Asking {:?} for doubling", epi_doubling))
                                    }
                                },
                                Some(epi_doubling),
                            );
                        },
                        GamePreparations((gamepreparations, epi_announce_game)) => {
                            self.players.for_each(
                                &SSendToPlayers::new(
                                    /*slcstich*/&[],
                                    /*orules*/None,
                                    |epi| gamepreparations.fullhand(epi).get(),
                                    /*msg_inactive*/VMessage::Info(format!("Asking {:?} for game", epi_announce_game)),
                                ),
                                |epi, otimeoutcmd| {
                                    if epi_announce_game==epi {
                                        let itgamephaseaction_rules = rules_to_gamephaseaction(
                                            &gamepreparations.ruleset.avecrulegroup[epi_announce_game],
                                            gamepreparations.fullhand(epi_announce_game),
                                            VGamePhaseAction::GamePreparations,
                                        );
                                        let gamephaseaction_rules_default = unwrap!(itgamephaseaction_rules.clone().next()).1;
                                        register_timeout(
                                            STimeoutAction::new(
                                                epi_announce_game,
                                                gamephaseaction_rules_default,
                                            ),
                                            otimeoutcmd,
                                            self_mutex.clone(),
                                        );
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
                                            vecstrgamephaseaction: itgamephaseaction_rules.collect(),
                                        }
                                    } else {
                                        VMessage::Info(format!("Asking {:?} for game", epi_announce_game))
                                    }
                                },
                                Some(epi_announce_game),
                            );
                        },
                        DetermineRules((determinerules, (epi_determine, vecrulegroup))) => {
                            self.players.for_each(
                                &SSendToPlayers::new(
                                    /*slcstich*/&[],
                                    /*orules*/None,
                                    |epi| determinerules.fullhand(epi).get(),
                                    /*msg_inactive*/VMessage::Info(format!("Re-Asking {:?} for game", epi_determine)),
                                ),
                                |epi, otimeoutcmd| {
                                    if epi_determine==epi {
                                        let itgamephaseaction_rules = rules_to_gamephaseaction(
                                            &vecrulegroup,
                                            determinerules.fullhand(epi_determine),
                                            VGamePhaseAction::DetermineRules,
                                        );
                                        let gamephaseaction_rules_default = unwrap!(itgamephaseaction_rules.clone().next()).1;
                                        register_timeout(
                                            STimeoutAction::new(
                                                epi_determine,
                                                gamephaseaction_rules_default,
                                            ),
                                            otimeoutcmd,
                                            self_mutex.clone(),
                                        );
                                        VMessage::Ask{
                                            str_question: format!(
                                                "Du bist an {}. Stelle. Von {}. Stelle wird {} geboten. Spielst Du etwas staerkeres?", // TODO umlaut-tactics?
                                                epi.to_usize() + 1, // EPlayerIndex is 0-based
                                                determinerules.tplepirules_current_bid.0.to_usize() + 1, // EPlayerIndex is 0-based
                                                determinerules.tplepirules_current_bid.1,
                                            ),
                                            vecstrgamephaseaction: itgamephaseaction_rules.collect(),
                                        }
                                    } else {
                                        VMessage::Info(format!("Re-Asking {:?} for game", epi_determine))
                                    }
                                },
                                Some(epi_determine),
                            );
                        },
                        Game((game, (epi_card, vecepi_stoss))) => {
                            self.players.for_each(
                                &SSendToPlayers::new(
                                    game.stichseq.visible_stichs(),
                                    Some(game.rules.as_ref()),
                                    |epi| game.ahand[epi].cards(),
                                    /*msg_inactive*/VMessage::Info(format!("Asking {:?} for card", epi_card)),
                                ),
                                |epi, otimeoutcmd| {
                                    let ostrgamephaseaction = if_then_some!(vecepi_stoss.contains(&epi),
                                        ("Stoss".into(), VGamePhaseAction::Game(VGameAction::Stoss))
                                    );
                                    if epi_card==epi {
                                        register_timeout(
                                            STimeoutAction::new(
                                                epi_card,
                                                VGamePhaseAction::Game(VGameAction::Zugeben(
                                                    *unwrap!(game.rules.all_allowed_cards(
                                                        &game.stichseq,
                                                        &game.ahand[epi_card],
                                                    ).choose(&mut thread_rng()))
                                                )),
                                            ),
                                            otimeoutcmd,
                                            self_mutex.clone(),
                                        );
                                        VMessage::Ask{
                                            str_question: "".into(),
                                            vecstrgamephaseaction: ostrgamephaseaction.into_iter().collect(),
                                        }
                                    } else if let Some(strgamephaseaction) = ostrgamephaseaction {
                                        VMessage::Ask{
                                            str_question: "".into(),
                                            vecstrgamephaseaction: vec![strgamephaseaction],
                                        }
                                    } else {
                                        VMessage::Info(format!("Asking {:?} for card", epi_card))
                                    }
                                },
                                Some(epi_card),
                            );
                        },
                        GameResult((gameresult, mapepib_confirmed)) => {
                            self.players.for_each(
                                &SSendToPlayers::new(
                                    /*slcstich*/if let VStockOrT::OrT(ref game) = gameresult.gameresult.stockorgame {
                                        game.stichseq.completed_stichs()
                                    } else {
                                        &[]
                                    },
                                    if_then_some!(let VStockOrT::OrT(ref game) = gameresult.gameresult.stockorgame,
                                        game.rules.as_ref()
                                    ),
                                    /*fn_cards*/|_epi| std::iter::empty::<ECard>(),
                                    /*msg_inactive*/VMessage::Info("Game finished".into()),
                                ),
                                |epi, otimeoutcmd| {
                                    if !mapepib_confirmed[epi] {
                                        register_timeout(
                                            STimeoutAction::new(
                                                epi,
                                                VGamePhaseAction::GameResult(()),
                                            ),
                                            otimeoutcmd,
                                            self_mutex.clone(),
                                        );
                                        VMessage::Ask{
                                            str_question: format!("Spiel beendet. {}", if gameresult.gameresult.an_payout[epi] < 0 {
                                                format!("Verlust: {}", -gameresult.gameresult.an_payout[epi])
                                            } else {
                                                format!("Gewinn: {}", gameresult.gameresult.an_payout[epi])
                                            }),
                                            vecstrgamephaseaction: Some(("Ok".into(), VGamePhaseAction::GameResult(()))).into_iter().collect(),
                                        }
                                    } else {
                                        VMessage::Info("Game finished".into())
                                    }
                                },
                                None,
                            );
                        },
                        Accepted((_accepted, infallible)) => {
                            let _infallible : /*mention type to get compiler error upon change*/Infallible = infallible;
                        },
                    }
                }
            }
        } else {
            self.players.for_each(
                &SSendToPlayers::new(
                    /*slcstich*/&[],
                    /*orules*/None,
                    /*fn_cards*/|_epi| std::iter::empty::<ECard>(),
                    /*msg_inactive*/VMessage::Info("Waiting for more players.".into()),
                ),
                |_oepi, _otimeoutcmd| VMessage::Info("Waiting for more players.".into()),
                None,
            );
        }
    }
}

#[derive(Serialize, Clone)]
enum VMessage {
    Info(String),
    Ask{
        str_question: String,
        vecstrgamephaseaction: Vec<(String, VGamePhaseAction)>,
    },
}

// timer adapted from https://rust-lang.github.io/async-book/02_execution/03_wakeups.html
// TODO should possibly be replaced standard facility
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
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
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
        spawn(move || {
            sleep(Duration::new(n_secs, /*nanos*/0));
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
    let wsstream = match accept_async(tcpstream).await {
        Ok(wsstream) => wsstream,
        Err(e) => {
            println!("Error in accepting tcpstream: {}", e);
            return;
        }
    };
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

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    task::block_on(internal_run(super::get_ruleset(clapmatches)?))
}

