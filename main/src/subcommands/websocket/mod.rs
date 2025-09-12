// adapted from https://github.com/sdroege/async-tungstenite/blob/master/examples/server.rs

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
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
        .arg(clap::Arg::new("with-bots")
            .long("with-bots")
            .help("Allow playing against bots")
        )
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
    epi: EPlayerIndex,
}

#[derive(Debug)]
struct SPeer {
    sockaddr: SocketAddr,
    txmsg: UnboundedSender<Message>,
    n_money: isize,
    str_name: String,
}

#[derive(Default, Debug)]
struct SPlayers {
    mapepiopeer_active: EnumMap<EPlayerIndex, Option<SPeer>>,
    vecpeer_inactive: Vec<SPeer>,
}
#[derive(Debug)]
struct STable{
    players: SPlayers,
    ogamephase: Option<VGamePhase>,
    otimeoutcmd: Option<STimeoutCmd>, // TODO? tie to ogamephase?
    n_stock: isize, // TODO would that be better within VGamePhase?
    ruleset: SRuleSet,
    b_with_bots: bool,
}

impl STable {
    fn new(ruleset: SRuleSet, b_with_bots: bool) -> Self {
        Self {
            players: SPlayers::default(),
            ogamephase: None,
            otimeoutcmd: None,
            n_stock: 0,
            ruleset,
            b_with_bots,
        }
    }

    fn start_new_game(&self) -> Option<SDealCards> {
        let mut itopeer = self.players.mapepiopeer_active.iter();
        if_then_some!(
            (self.b_with_bots && itopeer.any(Option::is_some))
                || itopeer.all(Option::is_some),
            SDealCards::new(
                self.ruleset.clone(),
                self.n_stock,
            )
        )
    }

    fn insert(&mut self, self_mutex: Arc<Mutex<Self>>, peer: SPeer) {
        match self.players.mapepiopeer_active
            .iter_mut()
            .find(|opeer| opeer.is_none())
        {
            Some(opeer) if self.ogamephase.is_none() => {
                verify!(opeer.replace(peer).is_none());
            },
            _ => {
                self.players.vecpeer_inactive.push(peer);
            }
        }
        if self.ogamephase.is_none() && let Some(dealcards) = self.start_new_game() {
            self.ogamephase = Some(VGamePhase::DealCards(dealcards));
            self.on_incoming_message(
                self_mutex,
                /*oepi*/None,
                /*ogamephaseaction*/None,
            ); // To trigger game logic. TODO beautify instead of dummy msg.
        }
    }

    fn remove(&mut self, sockaddr: &SocketAddr) {
        for epi in EPlayerIndex::values() {
            if self.players.mapepiopeer_active[epi].as_ref().map(|peer| peer.sockaddr)==Some(*sockaddr) {
                self.players.mapepiopeer_active[epi] = None;
            }
        }
        self.players.vecpeer_inactive.retain(|peer| peer.sockaddr!=*sockaddr);
    }
}

impl SPlayers {
    fn communicate_to_players(&self, sendtoplayers: &SSendToPlayers) {
        let mapepistr_name = self.mapepiopeer_active.map(|opeer_active| // TODO can we avoid temporary storage?
            opeer_active
                .as_ref()
                .map(|peer| peer.str_name.clone())
                .unwrap_or_else(||"<BOT>".to_string())
        );
        let communicate = |oepi: Option<EPlayerIndex>, veccard: Vec<ECard>, msg, peer: &SPeer| {
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
                        format!("{rules}"),
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
            if let Some(peer_active) = self.mapepiopeer_active[epi].as_ref() {
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
                    peer_active
                );
            }
        }
        for peer in self.vecpeer_inactive.iter() {
            communicate(None, vec![], /*msg*/sendtoplayers.msg_inactive.clone(/*TODO? needed?*/), peer);
        }
    }
}

impl STable {
    fn on_incoming_message(&mut self, /*TODO avoid this parameter*/self_mutex: Arc<Mutex<Self>>, oepi: Option<EPlayerIndex>, ogamephaseaction: Option<VGamePhaseAction>) {
        println!("on_incoming_message({oepi:?}, {ogamephaseaction:?})");
        if self.ogamephase.is_some() {
            if let (Some(epi), Some(gamephaseaction)) = (oepi, ogamephaseaction) {
                self.ogamephase = match verify_or_println!(unwrap!(self.ogamephase.take()).action(epi, gamephaseaction.clone())) {
                    Ok(gamephase) => {
                        if let Some(timeoutcmd) = &self.otimeoutcmd
                            && timeoutcmd.epi==epi && gamephaseaction.matches_phase(&timeoutcmd.gamephaseaction)
                        {
                            timeoutcmd.aborthandle.abort();
                            assert_eq!(epi, timeoutcmd.epi);
                            self.otimeoutcmd = None;
                        }
                        match gamephase {
                            VGamePhase::GameResult(gameresult) => {
                                gameresult.gameresult.clone(/*TODO avoid clone (and force apply_payout on construction?)*/).apply_payout(&mut self.n_stock, |epi, n_payout| {
                                    if let Some(peer) = &mut self.players.mapepiopeer_active[epi] {
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
                                let mapepiopeer_active = &mut self.players.mapepiopeer_active;
                                mapepiopeer_active.as_raw_mut().rotate_left(1);
                                // Players: E1 E2 E3 E0 [S0 S1 S2 ... SN]
                                if let Some(peer_epi3) = mapepiopeer_active[EPlayerIndex::EPI3].take() {
                                    self.players.vecpeer_inactive.push(peer_epi3);
                                }
                                // Players: E1 E2 E3 -- [S0 S1 S2 ... SN E0] (E1, E2, E3 may be None)
                                // Fill up players one after another
                                assert!(mapepiopeer_active[EPlayerIndex::EPI3].is_none());
                                for epi in EPlayerIndex::values() {
                                    if mapepiopeer_active[epi].is_none() && !self.players.vecpeer_inactive.is_empty() {
                                        mapepiopeer_active[epi] = Some(self.players.vecpeer_inactive.remove(0));
                                    }
                                }
                                // Players: E1 E2 E3 S0 [S1 S2 ... SN E0] (E1, E2, E3 may be None)
                                // TODO should we clear timeouts?
                                self.start_new_game()
                                    .map(|dealcards| VGamePhase::DealCards(dealcards))
                            },
                            gamephase => {
                                Some(gamephase)
                            },
                        }
                    },
                    Err(gamephase) => Some(gamephase),
                }
            }
            if let Some(ref gamephase) = self.ogamephase
                && let Some(sendtoplayers) = verify!(gamephase.which_player_can_do_something())
            {
                self.players.communicate_to_players(&sendtoplayers);
                if let Some(timeoutaction) = &sendtoplayers.otimeoutaction {
                    let epi_timeoutaction = timeoutaction.epi;
                    let gamephaseaction_timeout = timeoutaction.gamephaseaction_timeout.clone(); // TODO clone needed?
                    let (timerfuture, aborthandle) = future::abortable(async move {
                        task::sleep(Duration::new(/*secs*/2, /*nanos*/0)).await;
                        let table_mutex = self_mutex.clone();
                        let mut table = unwrap!(table_mutex.lock());
                        if let Some(timeoutcmd) = table.otimeoutcmd.take_if(|timeoutcmd| timeoutcmd.epi==epi_timeoutaction) {
                            table.on_incoming_message(table_mutex.clone(), Some(verify_eq!(timeoutcmd.epi, epi_timeoutaction)), Some(timeoutcmd.gamephaseaction));
                        }
                    });
                    assert!(self.otimeoutcmd.as_ref().is_none_or(|timeoutcmd|
                        timeoutcmd.gamephaseaction.matches_phase(&gamephaseaction_timeout)
                    ));
                    self.otimeoutcmd = Some(STimeoutCmd{
                        gamephaseaction: timeoutaction.gamephaseaction_timeout.clone(/*TODO needed?*/),
                        aborthandle,
                        epi: epi_timeoutaction,
                    });
                    task::spawn(timerfuture);
                }
            }
        } else {
            self.players.communicate_to_players(
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

async fn handle_connection(table: Arc<Mutex<STable>>, tcpstream: TcpStream, sockaddr: SocketAddr) {
    println!("Incoming TCP connection from: {sockaddr}");
    let Ok(wsstream) = verify_or_println!(accept_async(tcpstream).await) else {
        return;
    };
    println!("WebSocket connection established: {sockaddr}");
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
                .find(|epi| table.players.mapepiopeer_active[*epi].as_ref().map(|peer| peer.sockaddr)==Some(sockaddr));
            println!("Received a message from {sockaddr} ({oepi:?}): {str_msg}");
            if let Ok(playercmd) = verify_or_println!(serde_json::from_str(str_msg)) {
                match playercmd {
                    VPlayerCmd::GamePhaseAction(gamephaseaction) => table.on_incoming_message(table_mutex.clone(), oepi, Some(gamephaseaction)),
                    VPlayerCmd::PlayerLogin{str_player_name} => {
                        if let Some(ref epi)=oepi {
                            if let Some(ref mut peer) = table.players.mapepiopeer_active[*epi] {
                                peer.str_name = str_player_name;
                            }
                        } else if let Some(ref mut peer) = table.players.vecpeer_inactive.iter_mut().find(|peer| peer.sockaddr==sockaddr) {
                            peer.str_name = str_player_name;
                        }
                    },
                }
            }
            future::ok(())
        });
    let receive_from_others = rxmsg.map(Ok).forward(sink_ws_out);
    future::select(broadcast_incoming, receive_from_others).await;
    println!("{} disconnected", &sockaddr);
    unwrap!(table.lock()).remove(&sockaddr);
}

async fn internal_run(ruleset: SRuleSet, b_with_bots: bool) -> Result<(), Error> {
    let str_addr = "127.0.0.1:8080";
    let table = Arc::new(Mutex::new(STable::new(ruleset, b_with_bots)));
    // Create the event loop and TCP listener we'll accept connections on.
    let listener = unwrap!(TcpListener::bind(&str_addr).await);
    println!("Listening on: {str_addr}");
    // Let's spawn the handling of each connection in a separate task.
    while let Ok((tcpstream, sockaddr)) = listener.accept().await {
        task::spawn(handle_connection(table.clone(), tcpstream, sockaddr));
    }
    Ok(())
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    task::block_on(internal_run(
        super::get_ruleset(clapmatches)?,
        /*b_with_bots*/clapmatches.is_present("with-bots"),
    ))
}

