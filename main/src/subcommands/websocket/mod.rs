// adapted from https://github.com/sdroege/async-tungstenite/blob/master/examples/server.rs

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    pin::Pin,
    thread::{sleep, spawn},
    time::Duration,
};
use openschafkopf_lib::{
    game::*,
    rules::trumpfdecider::STrumpfDecider,
    rules::ruleset::{SRuleSet},
    primitives::*,
};
use openschafkopf_util::*;
use derive_new::new;
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
use plain_enum::{EnumMap, PlainEnum};
use failure::*;

mod gamephase;
use gamephase::{
    SSendToPlayers,
    VGamePhase,
    VGamePhaseAction,
    VGameAction,
    VMessage,
};

pub fn subcommand(str_subcommand: &'static str) -> clap::Command<'static> {
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

impl SPlayers {
    fn for_each(
        &mut self,
        table_mutex: Arc<Mutex<STable>>,
        sendtoplayers: &SSendToPlayers,
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
            if let Err(err) = peer.txmsg.unbounded_send(
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
                    sendtoplayers.otimeoutaction
                        .as_ref()
                        .map(|timeoutaction| playerindex_server_to_client(timeoutaction.epi)),
                ))).into()
            ) {
                assert!(err.is_disconnected());
            }
        };
        for epi in EPlayerIndex::values() {
            let activepeer = &mut self.mapepiopeer[epi];
            if let Some(timeoutaction) = &sendtoplayers.otimeoutaction {
                if timeoutaction.epi==epi {
                    let (timerfuture, aborthandle) = future::abortable(STimerFuture::new(
                        /*n_secs*/2,
                        table_mutex.clone(),
                        verify_eq!(timeoutaction.epi, epi),
                    ));
                    assert!(activepeer.otimeoutcmd.as_ref().map_or(true, |timeoutcmd|
                        timeoutcmd.gamephaseaction.matches_phase(&timeoutaction.gamephaseaction_timeout)
                    )); // only one active timeout cmd
                    activepeer.otimeoutcmd = Some(STimeoutCmd{
                        gamephaseaction: timeoutaction.gamephaseaction_timeout.clone(/*TODO needed?*/),
                        aborthandle,
                    });
                    task::spawn(timerfuture);
                }
            }
            if let Some(ref mut peer) = activepeer.opeer.as_mut() {
                let mut veccard = sendtoplayers.mapepiveccard[epi].clone(); // TODO? avoid clone
                if let Some(rules) = sendtoplayers.orules {
                    rules.sort_cards(&mut veccard);
                } else {
                    STrumpfDecider::new(&[ESchlag::Ober, ESchlag::Unter], Some(EFarbe::Herz))
                        .sort_cards(&mut veccard)
                }
                communicate(
                    Some(epi),
                    veccard,
                    sendtoplayers.mapepiomsg_active[epi]
                        .as_ref()
                        .unwrap_or(&sendtoplayers.msg_inactive)
                        .clone(),
                    peer
                );
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
                if let Some(sendtoplayers) = verify!(gamephase.which_player_can_do_something()) {
                    self.players.for_each(self_mutex, &sendtoplayers);
                }
            }
        } else {
            self.players.for_each(
                self_mutex,
                &SSendToPlayers::new(
                    /*slcstich*/&[],
                    /*orules*/None,
                    /*fn_cards*/|_epi| std::iter::empty::<ECard>(),
                    /*fn_msg_active*/|_epi| None,
                    /*msg_inactive*/VMessage::Info("Waiting for more players.".into()),
                    /*otimeoutaction*/None,
                ),
            );
        }
    }
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

