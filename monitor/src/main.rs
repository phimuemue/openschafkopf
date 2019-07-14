extern crate serde_json;
extern crate byteorder;
extern crate as_num;
extern crate openschafkopf_util;
use openschafkopf_util::*;
use as_num::AsNum;
use byteorder::ByteOrder;
use serde_json::json;
use std::io::{Read, Write};

// TODO proper logging

fn main() {
    use std::sync::{Arc, Mutex};
    let ocmd_openschafkopf : Arc<Mutex<Option<std::process::Child>>> = Arc::new(Mutex::new(None));
    let (sendstr, recvstr) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(str_openschafkopf_out) = recvstr.recv() {
            let str_json_out = json!({
                "strOpenschafkopfOut": str_openschafkopf_out
            }).to_string();
            let mut abyte_buffer_msg_len = [0; 4];
            byteorder::NativeEndian::write_u32(&mut abyte_buffer_msg_len, str_json_out.len().as_num::<u32>());
            debug_verify!(std::io::stdout().write(&abyte_buffer_msg_len)).unwrap();
            debug_verify!(std::io::stdout().write(str_json_out.as_bytes())).unwrap();
            debug_verify!(std::io::stdout().flush()).unwrap();
        }
    });
    loop {
        let str_json_in = {
            let mut abyte_buffer_msg_len = [0; 4];
            let n_bytes_read = debug_verify!(std::io::stdin().read(&mut abyte_buffer_msg_len)).unwrap();
            assert_eq!(n_bytes_read, abyte_buffer_msg_len.len());
            let n_bytes_msg_len = byteorder::NativeEndian::read_u32(&abyte_buffer_msg_len);
            let mut vecbyte : Vec<u8> = (0..n_bytes_msg_len).map(|_| 0).collect();
            debug_verify!(std::io::stdin().read(vecbyte.as_mut_slice())).unwrap();
            debug_verify!(String::from_utf8(vecbyte)).unwrap()
        };
        let communicate_error = |str_error_msg| {
            debug_verify!(sendstr.send(json!({
                "Err": {
                    "strErrorMsg": str_error_msg,
                    "strInput": str_json_in
                }
            }).to_string()/*TODO? better to avoid digression via json value?*/)).unwrap();
        };
        match serde_json::de::from_str::<serde_json::Value>(&str_json_in) {
            Ok(jsonval) => {
                if let Some(mut cmd_openschafkopf) = debug_verify!(ocmd_openschafkopf.lock()).unwrap()
                    .take()
                {
                    if let Ok(()) = cmd_openschafkopf.kill() {
                        eprintln!("Process did not finish early enough.");
                    }
                }
                macro_rules! json_get(($index: expr, $fn_extract: ident) => {
                    if let Some(val) = jsonval.get($index) {
                        if let Some(x) = val.$fn_extract() {
                            x
                        } else {
                            communicate_error(format!("{} not extractable {}", val, stringify!($fn_extract)));
                            break;
                        }
                    } else {
                        communicate_error(format!("Missing field: {}", $index));
                        break;
                    }
                });
                let str_cards_as_played = json_get!("strCardsAsPlayed", as_str);
                let str_hand = json_get!("strHand", as_str);
                let str_selected_game_name = json_get!("selectedGameName", as_str);
                let str_selected_game_suit = json_get!("selectedGameSuit", as_str);
                let jsonarr_announcement = json_get!("announcements", as_array);
                let n_epi_active = {
                    match jsonarr_announcement.iter()
                        .enumerate()
                        .filter(|&(_n_epi, jsonval_announcement)| jsonval_announcement.is_string())
                        .single() {
                        Ok((n_epi_active, _str_announcement)) => {
                            n_epi_active
                        },
                        Err(e) => {
                            communicate_error(format!("No single announcement: {:?}", e));
                            break;
                        }
                    }
                };
                let n_epi_first = json_get!("firstPosition", as_u64);
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
                            format!("{}", n_epi_first), // first_player_index
                            format!("{} von {}",
                                {
                                    match str_selected_game_name {
                                        "Sauspiel" => format!("Sauspiel auf die {}", {
                                            match str_selected_game_suit {
                                                "E" => "Alte",
                                                "G" => "Blaue",
                                                "S" => "Hundsgfickte",
                                                _ => {
                                                    communicate_error(format!("Bad Sauspiel farbe: {}", str_selected_game_suit));
                                                    break;
                                                },
                                            }
                                        }),
                                        "Solo"|"Farbwenz" => format!("{}-{}",
                                            {
                                                match str_selected_game_suit {
                                                    "E" => "Eichel",
                                                    "G" => "Gras",
                                                    "H" => "Herz",
                                                    "S" => "Schellen",
                                                    _ => {
                                                        communicate_error(format!("Bad farbe: {}", str_selected_game_suit));
                                                        break;
                                                    }
                                                }
                                            },
                                            str_selected_game_name
                                        ),
                                        "Wenz"|"Geier" => str_selected_game_name.to_owned(),
                                        _ => {
                                            communicate_error(format!("Unknown game type: {}", str_selected_game_name));
                                            break;
                                        },
                                    }
                                },
                                n_epi_active,
                            ),
                            str_hand.to_owned(), // hand
                            str_cards_as_played.to_owned(), // cards in order
                        ])
                        .stdout(std::process::Stdio::piped())
                        .spawn()
                ).expect("Could not spawn process");
                let stdout = debug_verify!(cmd_openschafkopf.stdout.take()).unwrap();
                *debug_verify!(ocmd_openschafkopf.lock()).unwrap() = Some(cmd_openschafkopf);
                std::thread::spawn(move || {
                    let mut str_openschafkopf_out = String::new();
                    if std::io::BufReader::new(stdout).read_to_string(&mut str_openschafkopf_out).is_ok() {
                        debug_verify!(sendstr.send(str_openschafkopf_out)).unwrap();
                        debug_verify!(ocmd_openschafkopf.lock()).unwrap().take();
                    }
                });
            },
            Err(e) => {
                communicate_error(format!("{:?} (category {:?})", e, e.classify()));
            }
        };
    }
}
