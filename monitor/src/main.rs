use as_num::AsNum;
use byteorder::ByteOrder;
use itertools::Itertools;
use openschafkopf_logging::*;
use openschafkopf_util::*;
use serde_json::json;
use std::io::{Read, Write};

fn main() -> Result<(), failure::Error> {
    openschafkopf_logging::init_logging()?;
    use std::sync::{Arc, Mutex};
    let ocmd_openschafkopf: Arc<Mutex<Option<std::process::Child>>> = Arc::new(Mutex::new(None));
    let (sendstr, recvstr) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(str_openschafkopf_out) = recvstr.recv() {
            let str_json_out = json!({ "strOpenschafkopfOut": str_openschafkopf_out }).to_string();
            info!("Trying to send \"{}\"", str_json_out);
            debug_verify!(std::io::stdout().write(
                &via_out_param(|abyte_buffer_msg_len: &mut [u8; 4]| {
                    byteorder::NativeEndian::write_u32(
                        abyte_buffer_msg_len,
                        str_json_out.len().as_num::<u32>(),
                    )
                })
                .0
            ))
            .unwrap();
            debug_verify!(std::io::stdout().write(str_json_out.as_bytes())).unwrap();
            debug_verify!(std::io::stdout().flush()).unwrap();
        }
    });
    loop {
        let str_json_in = {
            const N_BYTES_FOR_MSG_LEN: usize = 4;
            let (abyte_buffer_msg_len, n_bytes_read) =
                via_out_param(|abyte_buffer_msg_len: &mut [u8; N_BYTES_FOR_MSG_LEN]| {
                    debug_verify!(std::io::stdin().read(abyte_buffer_msg_len)).unwrap()
                });
            match n_bytes_read {
                0 => {
                    info!("Received 0 bytes. Exiting.");
                    return Ok(());
                }
                N_BYTES_FOR_MSG_LEN => {
                    let n_bytes_msg_len = byteorder::NativeEndian::read_u32(&abyte_buffer_msg_len);
                    let str_json_in = debug_verify!(String::from_utf8(
                        debug_verify!(via_out_param_init_result(
                            (0..n_bytes_msg_len).map(|_| 0).collect::<Vec<_>>(),
                            |vecbyte| std::io::stdin().read(vecbyte)
                        ))
                        .unwrap()
                        .0
                    ))
                    .unwrap();
                    info!("Received \"{}\"", str_json_in);
                    str_json_in
                }
                _ => panic!("Unexpected value for n_bytes_read: {}", n_bytes_read),
            }
        };
        let communicate_error = |str_error_msg| {
            warn!("Communicating error: {}", str_error_msg);
            debug_verify!(sendstr.send(
                json!({
                    "Err": {
                        "strErrorMsg": str_error_msg,
                        "strInput": str_json_in
                    }
                })
                .to_string() /*TODO? better to avoid digression via json value?*/
            ))
            .unwrap();
        };
        match serde_json::de::from_str::<serde_json::Value>(&str_json_in) {
            Ok(jsonval) => {
                if let Some(mut cmd_openschafkopf) =
                    debug_verify!(ocmd_openschafkopf.lock()).unwrap().take()
                {
                    if let Ok(()) = cmd_openschafkopf.kill() {
                        communicate_error("Process did not finish early enough.");
                    }
                }
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
                let str_selected_game_name = json_get!("selectedGameName", as_str);
                let jsonarr_announcement = json_get!("announcements", as_array);
                let n_epi_active = {
                    match jsonarr_announcement
                        .iter()
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
                let ocmd_openschafkopf = ocmd_openschafkopf.clone();
                let sendstr = sendstr.clone();
                let mut cmd_openschafkopf = debug_verify!(
                    std::process::Command::new({
                        let path_self = debug_verify!(std::env::current_exe()).unwrap();
                        assert!(!debug_verify!(path_self.symlink_metadata()).unwrap() // "Queries the metadata about a file without following symlinks" (https://doc.rust-lang.org/std/path/struct.PathBuf.html#method.symlink_metadata)
                            .file_type()
                            .is_symlink()
                        );
                        debug_verify!(path_self.parent()).unwrap().join("openschafkopf")
                    })
                        .args(&[
                            "suggest-card".to_owned(),
                            "--rules".to_owned(),
                            format!("{} von {}",
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
                            ),
                            "--hand".to_owned(),
                            str_hand.to_owned(),
                            "--cards-on-table".to_owned(),
                            str_cards_as_played.to_owned(),
                        ])
                        .stdout(std::process::Stdio::piped())
                        .spawn()
                ).expect("Could not spawn process");
                let stdout = debug_verify!(cmd_openschafkopf.stdout.take()).unwrap();
                *debug_verify!(ocmd_openschafkopf.lock()).unwrap() = Some(cmd_openschafkopf);
                std::thread::spawn(move || {
                    if let Ok((str_openschafkopf_out, _n_bytes)) =
                        via_out_param_result(|str_openschafkopf_out| {
                            std::io::BufReader::new(stdout).read_to_string(str_openschafkopf_out)
                        })
                    {
                        debug_verify!(sendstr.send(str_openschafkopf_out)).unwrap();
                        debug_verify!(ocmd_openschafkopf.lock()).unwrap().take();
                    }
                });
            }
            Err(e) => {
                communicate_error(&format!("{:?} (category {:?})", e, e.classify()));
            }
        };
    }
}
