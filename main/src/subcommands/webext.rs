//use as_num::AsNum;
use byteorder::ByteOrder;
use itertools::Itertools;
//use openschafkopf_logging::*;
//use openschafkopf_util::*;
use serde_json::json;
use std::io::{Read, Write};
use openschafkopf_lib::{
    primitives::{
        cardvector::parse_cards,
        eplayerindex::EPlayerIndex,
    },
    rules::{
        TRules,
        parser::parse_rule_description_simple,
    },
};
use openschafkopf_util::*;
use plain_enum::PlainEnum;
use as_num::*;

pub fn subcommand(str_subcommand: &'static str) -> clap::Command<'static> {
    clap::Command::new(str_subcommand)
        .about("Backend of a web-extension suggesting a card for a given game state")
}

pub fn run(_clapmatches: &clap::ArgMatches) -> Result<(), failure::Error> {
    use std::sync::{Arc, Mutex};
    let ocmd_openschafkopf: Arc<Mutex<Option<std::process::Child>>> = Arc::new(Mutex::new(None));
    let (sendstr, recvstr) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(str_openschafkopf_out) = recvstr.recv() {
            let str_json_out = json!({ "strOpenschafkopfOut": str_openschafkopf_out }).to_string();
            info!("Trying to send \"{}\"", str_json_out);
            unwrap!(std::io::stdout().write_all(
                &via_out_param(|abyte_buffer_msg_len: &mut [u8; 4]| {
                    byteorder::NativeEndian::write_u32(
                        abyte_buffer_msg_len,
                        str_json_out.len().as_num::<u32>(),
                    )
                })
                .0
            ));
            unwrap!(std::io::stdout().write_all(str_json_out.as_bytes()));
            unwrap!(std::io::stdout().flush());
        }
    });
    loop {
        let str_json_in = {
            const N_BYTES_FOR_MSG_LEN: usize = 4;
            let (abyte_buffer_msg_len, n_bytes_read) =
                via_out_param(|abyte_buffer_msg_len: &mut [u8; N_BYTES_FOR_MSG_LEN]| {
                    unwrap!(std::io::stdin().read(abyte_buffer_msg_len))
                });
            match n_bytes_read {
                0 => {
                    info!("Received 0 bytes. Exiting.");
                    if let Some(mut cmd_openschafkopf) = unwrap!(ocmd_openschafkopf.lock()).take() {
                        if let Ok(()) = cmd_openschafkopf.kill() {
                            info!("Could not kill openschafkopf upon exiting.");
                        }
                    }
                    return Ok(());
                }
                N_BYTES_FOR_MSG_LEN => {
                    let n_bytes_msg_len = byteorder::NativeEndian::read_u32(&abyte_buffer_msg_len);
                    let str_json_in = unwrap!(String::from_utf8(
                        unwrap!(via_out_param_init_result(
                            (0..n_bytes_msg_len).map(|_| 0).collect::<Vec<_>>(),
                            |vecbyte| std::io::stdin().read(vecbyte)
                        ))
                        .0
                    ));
                    info!("Received \"{}\"", str_json_in);
                    str_json_in
                }
                _ => panic!("Unexpected value for n_bytes_read: {}", n_bytes_read),
            }
        };
        fn internal_communicate_error(sendstr: &std::sync::mpsc::Sender<String>, str_error_msg: &str, str_json_in: &str) {
            warn!("Communicating error: {}", str_error_msg);
            unwrap!(sendstr.send(
                json!({
                    "Err": {
                        "strErrorMsg": str_error_msg,
                        "strInput": str_json_in
                    }
                })
                .to_string() /*TODO? better to avoid digression via json value?*/
            ));
        }
        let communicate_error = |str_error_msg: &str| {
            internal_communicate_error(&sendstr, str_error_msg, &str_json_in)
        };
        match serde_json::de::from_str::<serde_json::Value>(&str_json_in) {
            Ok(jsonval) => {
                macro_rules! json_get(($index: expr, $fn_extract: ident) => {
                    if let Some(val) = jsonval.get($index) {
                        if let Some(x) = val.$fn_extract() {
                            x
                        } else {
                            communicate_error(&format!("{} not extractable {}", val, stringify!($fn_extract)));
                            continue;
                        }
                    } else {
                        communicate_error(&format!("Missing field: {}", $index));
                        continue;
                    }
                });
                let str_cards_as_played = json_get!("strCardsAsPlayed", as_str);
                let str_hand = json_get!("strHand", as_str);
                let n_hand_cards = unwrap!(parse_cards::<Vec<_>>(str_hand)).len();
                let str_selected_game_name = json_get!("selectedGameName", as_str);
                let jsonarr_announcement = json_get!("announcements", as_array);
                let n_epi_first = json_get!("firstPosition", as_u64).as_num::<usize>();
                let n_epi_active = {
                    match jsonarr_announcement
                        .iter()
                        .cycle()
                        .skip(n_epi_first)
                        .take(verify_eq!(EPlayerIndex::SIZE, jsonarr_announcement.len()))
                        .enumerate()
                        .filter(|&(_n_epi, jsonval_announcement)| jsonval_announcement.is_string())
                        .exactly_one()
                    {
                        Ok((n_epi_active, _str_announcement)) => n_epi_active,
                        Err(e) => {
                            communicate_error(&format!("No single announcement: {:?}", e));
                            continue;
                        }
                    }
                };
                let sendstr = sendstr.clone();
                let str_rules = &format!("{} von {}",
                    {
                        macro_rules! extract_farbe(() => {
                            match json_get!("selectedGameSuit", as_str) {
                                "E" => "Eichel",
                                "G" => "Gras",
                                "H" => "Herz",
                                "S" => "Schellen",
                                str_selected_game_suit => {
                                    communicate_error(&format!("Bad farbe: {}", str_selected_game_suit));
                                    continue;
                                }
                            }
                        });
                        match str_selected_game_name {
                            "Sauspiel" => format!("Sauspiel auf die {}", extract_farbe!()),
                            "Solo"|"Farbwenz" => format!("{}-{}", extract_farbe!(), str_selected_game_name),
                            "Wenz"|"Geier" => str_selected_game_name.to_owned(),
                            _ => {
                                communicate_error(&format!("Unknown game type: {}", str_selected_game_name));
                                continue;
                            },
                        }
                    },
                    n_epi_active,
                );
                const N_AHAND_POOL : usize = 1000;
                let str_simulate_hands = if n_hand_cards<=3 {
                    "all".to_owned()
                } else if let Some(f_occurence_probability)=
                    unwrap!(parse_rule_description_simple(str_rules))
                        .heuristic_active_occurence_probability()
                {
                    assert!(0. <= f_occurence_probability);
                    assert!(f_occurence_probability <= 1.);
                    format!("{}/{}",
                        (N_AHAND_POOL.as_num::<f64>() * f_occurence_probability).ceil(),
                        N_AHAND_POOL
                    )
                } else {
                    format!("{}", N_AHAND_POOL)
                };
                let str_branching = if n_hand_cards<=4 {
                    ""
                } else {
                    "1,3"
                };
                let str_repeat_hands = if n_hand_cards<=3 {
                    "1"
                } else {
                    "10"
                };
                let mut cmd_openschafkopf = debug_verify!(
                    std::process::Command::new({
                        let path_self = unwrap!(std::env::current_exe());
                        assert!(!unwrap!(path_self.symlink_metadata()) // "Queries the metadata about a file without following symlinks" (https://doc.rust-lang.org/std/path/struct.PathBuf.html#method.symlink_metadata)
                            .file_type()
                            .is_symlink()
                        );
                        path_self
                    })
                        .args([
                            "suggest-card",
                            "--rules",
                            str_rules,
                            "--hand",
                            str_hand,
                            "--cards-on-table",
                            str_cards_as_played,
                            "--simulate-hands",
                            &str_simulate_hands,
                            "--branching",
                            str_branching,
                            "--repeat-hands",
                            str_repeat_hands,
                        ])
                        .stdout(std::process::Stdio::piped())
                        .spawn()
                ).expect("Could not spawn process");
                let stdout = unwrap!(cmd_openschafkopf.stdout.take());
                let ocmd_openschafkopf = ocmd_openschafkopf.clone();
                {
                    let mut ocmd_openschafkopf = unwrap!(ocmd_openschafkopf.lock());
                    if let Some(mut cmd_openschafkopf) = ocmd_openschafkopf.take() {
                        if let Ok(()) = cmd_openschafkopf.kill() {
                            communicate_error("Process did not finish early enough.");
                        }
                    }
                    *ocmd_openschafkopf = Some(cmd_openschafkopf);
                }
                std::thread::spawn(move || {
                    if let Ok(str_openschafkopf_out) = std::io::read_to_string(stdout) {
                        if str_openschafkopf_out.trim().is_empty() {
                            internal_communicate_error(&sendstr, "openschafkopf returned empty string", "N/A");
                        } else {
                            unwrap!(sendstr.send(str_openschafkopf_out));
                        }
                    }
                });
            }
            Err(e) => {
                communicate_error(&format!("{:?} (category {:?})", e, e.classify()));
            }
        };
    }
}
