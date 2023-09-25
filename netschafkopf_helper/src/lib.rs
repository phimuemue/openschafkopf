#![cfg(windows)]

use openschafkopf_util::*;
use openschafkopf_lib::{
    ai::{
        determine_best_card,
        gametree::{EMinMaxStrategy, SNoFilter, SMinReachablePayout, SSnapshotCacheNone, SNoVisualization},
        handiterators::all_possible_hands,
    },
    game::{SGame, SExpensifiersNoStoss, TGamePhase},
    primitives::{EKurzLang, EPlayerIndex, EFarbe, ESchlag, ECard, SStichSequence, SHand, SDisplayCardSlice, display_card_slices, SStaticEPI0},
    rules::{
        SDoublings,
        SStoss,
        SStossParams,
        parser::parse_rule_description,
    },
};
use plain_enum::{EnumMap, PlainEnum};
use as_num::AsNum;

// from https://docs.rs/winsafe/latest/src/winsafe/kernel/funcs.rs.html#1442-1444, https://docs.rs/winsafe/latest/winsafe/fn.MAKEDWORD.html
pub const fn make_dword(lo: u16, hi: u16) -> u32 {
	((lo as u32 & 0xffff) | ((hi as u32 & 0xffff) << 16)) as _
}

use winapi::{
    shared::{
        basetsd::INT_PTR,
        minwindef::{
            BOOL,
            FALSE,
            TRUE,
            DWORD,
            HINSTANCE,
            LPVOID,
            UINT,
            WPARAM,
            LPARAM,
            LRESULT,
            FARPROC,
        },
        windef::HWND,
        ntdef::SHORT,
    },
    um::{
        libloaderapi::{
            GetModuleHandleW,
            GetProcAddress,
        },
        winnt::{
            CHAR,
            DLL_PROCESS_ATTACH,
            LPCSTR,
        },
        winuser::{
            GetKeyState,
            GetDlgItem,
            SendMessageA,
            PostMessageA,
            IsWindow,
            IsWindowEnabled,
            IsWindowVisible,
            WM_KEYDOWN,
            WM_COMMAND,
            WM_INITDIALOG,
            WM_SETTEXT,
            WM_GETTEXT,
            WM_SHOWWINDOW,
            WM_USER,
            VK_LEFT,
            VK_RIGHT,
            VK_UP,
            VK_CONTROL,
            LB_SELECTSTRING,
            LBN_SELCHANGE,
            LB_ERR,
        },
    },
};
use std::{
    borrow::{Borrow, Cow},
    fs::{
        self,
        File,
    },
    ffi::{CString, c_char, c_int},
    fmt::Debug,
    io::Write,
};
use log::{info, error};
use itertools::Itertools;
use libc::size_t;

#[allow(non_camel_case_types)]
type rsize_t = size_t; // https://en.cppreference.com/w/c/error
#[allow(non_camel_case_types)]
type errno_t = isize; // https://en.cppreference.com/w/c/error: "typedef for the type int"

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
extern "system" fn DllMain(
    dll_module: HINSTANCE,
    call_reason: DWORD,
    reserved: LPVOID,
) -> BOOL {
    if DLL_PROCESS_ATTACH==call_reason {
        initialize();
    }
    TRUE
}

fn byte_is_farbe(byte: u8) -> Option<EFarbe> {
    match byte {
        b'E' => Some(EFarbe::Eichel),
        b'G' => Some(EFarbe::Gras),
        b'H' => Some(EFarbe::Herz),
        b'S' => Some(EFarbe::Schelln),
        _ => None,
    }
}
fn byte_is_schlag(byte: u8) -> Option<ESchlag> {
    match byte {
        b'7' => Some(ESchlag::S7),
        b'8' => Some(ESchlag::S8),
        b'9' => Some(ESchlag::S9),
        b'Z' => Some(ESchlag::Zehn),
        b'U' => Some(ESchlag::Unter),
        b'O' => Some(ESchlag::Ober),
        b'K' => Some(ESchlag::Koenig),
        b'A' => Some(ESchlag::Ass),
        _ => None,
    }
}
fn bytes_are_card(slcbyte: &[u8; N_BYTES_PER_NETSCHAFKOPF_CARD]) -> Option<ECard> {
    if_then_some!(
        let (Some(efarbe), Some(eschlag))=(
            byte_is_farbe(slcbyte[0]),
            byte_is_schlag(slcbyte[1]),
        ),
        {
            assert_eq!(slcbyte[2], 0);
            ECard::new(efarbe, eschlag)
        }
    )
}

#[allow(dead_code)]
unsafe fn log_bytes(pbyte: *const u8, n_bytes: usize) {
    for byte in unsafe{std::slice::from_raw_parts(pbyte, n_bytes)} {
        info!("{:0<3} {:#x} {}", byte, byte, char::from(*byte));
    }
}

macro_rules! stringify_matches{($expr:expr, ($($context:expr,)*), $($pat:pat)*) => {{
    let str_context = format!("{:?}", ($($context,)*)); // TODO should we make this lazy?
    match $expr {
        $($pat => Some(format!("{}; {}", stringify!($pat), str_context)),)*
        _ => None,
    }
}}}

fn log_in_out_cond<
    Args: Clone+Debug,
    ShouldLog: Debug,
    R: Debug,
    F: UnpackAndApplyFn<Args, R>,
>(
    str_f: &str,
    args: Args,
    fn_cond: impl FnOnce(&Args)->Option<ShouldLog>,
    f: F
) -> R {
    if let Some(shouldlog) = fn_cond(&args) {
        let args_clone = args.clone();
        info!("{} {:?} [{:?}] <-", str_f, args_clone, shouldlog);
        let retval = f.apply(args);
        info!("{} {:?} [{:?}] -> {:?}", str_f, args_clone, shouldlog, retval);
        retval
    } else {
        f.apply(args)
    }
}
fn log_in_out<
    Args: Clone+Debug,
    R: Debug,
    F: UnpackAndApplyFn<Args, R>
>(
    str_f: &str,
    args: Args,
    f: F,
) -> R {
    log_in_out_cond(str_f, args, |_| Some(()), f)
}

macro_rules! make_redirect_function(
    (
        $fn_name:ident,
        $pfn_original:expr,
        ($($extern:tt)*) ($($paramname:ident : $paramtype:ty,)*)->$rettype:ty,
        $fn_new:expr,
    ) => {
        pub unsafe extern $($extern)* fn $fn_name($($paramname: $paramtype,)*)->$rettype {
            $fn_name::redirected_should_only_be_called_from_wrapper($($paramname,)*)
        }
        mod $fn_name {
            use super::*;
            use retour::GenericDetour;

            static mut OHOOK: Option<GenericDetour<
                unsafe extern $($extern)* fn ($($paramname: $paramtype,)*)->$rettype
            >> = None;

            #[inline(always)]
            pub unsafe fn call_original(
                $($paramname: $paramtype,)*
            ) -> $rettype {
                unwrap!(OHOOK.as_ref()).call($($paramname,)*)
            }

            pub extern $($extern)* fn redirected_should_only_be_called_from_wrapper($($paramname: $paramtype,)*)->$rettype {
                $fn_new
            }

            pub unsafe fn redirect() {
                log_in_out(&format!("{}::redirect", stringify!($fn_name)), (), || {
                    let pfn_original: unsafe extern $($extern)* fn($($paramtype,)*)->$rettype =
                        std::mem::transmute($pfn_original);
                    OHOOK = Some(unwrap!(GenericDetour::new(pfn_original, redirected_should_only_be_called_from_wrapper)));
                    unwrap!(unwrap!(OHOOK.as_ref()).enable());
                })
            }
        }
    }
);
make_redirect_function!(
    netschk_increment_playerindex,
    /*pfn_original*/0x00463a20,
    ("C") (i_epi: isize,)->isize,
    {
        log_in_out("increment_playerindex", (i_epi,), |i_epi| unsafe{call_original(i_epi)})
    },
);

#[derive(Debug, PartialEq, Eq, Clone)]
enum EKnownDuAktion {
    Kartenwahl,
    // StichBestaetigen,
}

unsafe fn scan_until_0<'slc>(pch: *const u8, on_max_bytes: impl Into<Option<usize>>) -> &'slc [u8] {
    // TODO can we somehow restrict unsafe's scope within this function?
    let mut pch_current = pch;
    let mut n_bytes_before_0 = 0;
    let n_max_bytes = on_max_bytes.into().unwrap_or(usize::MAX);
    while *pch_current!=0 && n_bytes_before_0 <= n_max_bytes {
        pch_current = pch_current.add(1);
        n_bytes_before_0 += 1;
    }
    std::slice::from_raw_parts(pch as *const u8, n_bytes_before_0)
}

make_redirect_function!(
    netschk_strcpy_s,
    /*pfn_original*/0x00473757,
    ("C") (dst: *mut c_char, n_bytes_requested: rsize_t, src: *const c_char,)->errno_t,
    {
        let res = unsafe{call_original(
            dst,
            n_bytes_requested,
            src,
        )};
        if let Some(card) = if_then_some!(n_bytes_requested==N_BYTES_PER_NETSCHAFKOPF_CARD, ()).and_then(|()|
            bytes_are_card(unwrap!(unsafe{std::slice::from_raw_parts(src as *const u8, N_BYTES_PER_NETSCHAFKOPF_CARD)}.try_into()))
        ) {
            info!("Moving card {}: {:?} => {:?}",
                card,
                src,
                dst,
            );
        } else {
            let str_src = String::from_utf8_lossy(
                unsafe{scan_until_0(src as *const u8, n_bytes_requested)}
            );
            info!("strcpy_s: {:?} => {:?}: {}",
                src,
                dst,
                str_src,
            );
        }
        res
    },
);

const N_INDEX_GAST : isize = 4;

make_redirect_function!(
    netschk_process_window_message,
    /*pfn_original*/0x0043f3a0,
    ("system") (hwnd: HWND, u_msg: UINT, wparam: WPARAM, lparam: LPARAM,)->LRESULT,
    {
        const NETSCHK_MSG_SPIELABFRAGE_1 : UINT = 0x471;
        const NETSCHK_MSG_SPIELABFRAGE_2 : UINT = 0x475;
        const NETSCHK_MSG_AKTIONSABFRAGE: UINT = 0x42a;
        const NETSCHK_MSG_KARTEGESPIELT: UINT = 0x42b;
        log_in_out_cond(
            "process_window_message",
            (hwnd, u_msg, wparam, lparam),
            |&(_hwnd, u_msg, _wparam, _lparam)| match u_msg {
                WM_KEYDOWN => {
                    if wparam==unsafe{std::mem::transmute(VK_LEFT)} || wparam==unsafe{std::mem::transmute(VK_RIGHT)} {
                        Some(format!(
                            "WM_KEYDOWN, VK_LEFT/VK_RIGHT: {:?}",
                            unsafe{std::slice::from_raw_parts(std::mem::transmute::<_, *const u8>(0x004ca2b0), 4)},
                        ))
                    } else if
                        0!=(unsafe{GetKeyState(VK_CONTROL)}&unsafe{std::mem::transmute::<_,SHORT>(0x8000u16)})
                        && wparam==unsafe{std::mem::transmute(0x4F)} // "O key" https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
                    {
                        Some(format!("WM_KEYDOWN, Ctrl+O"))
                    } else {
                        None
                    }
                },
                netschk_msg@WM_USER..=0xffff => {
                    Some(format!("NETSCHK_MSG {:#x}, {}, {}", netschk_msg, wparam, lparam))
                },
                u_msg => stringify_matches!(u_msg, (wparam, lparam,),
                    WM_COMMAND
                    NETSCHK_MSG_SPIELABFRAGE_1
                    NETSCHK_MSG_SPIELABFRAGE_2
                    NETSCHK_MSG_AKTIONSABFRAGE
                    NETSCHK_MSG_KARTEGESPIELT
                ),
            },
            |hwnd, u_msg, wparam, lparam| {
                let resoknownduaktion_expected = dbg!(match (u_msg, wparam, lparam) {
                    (NETSCHK_MSG_AKTIONSABFRAGE, _, _/*no check for N_INDEX_GAST - Kartenabfrage also happens in other circumstances*/) => {
                        Ok(None) // expectation, but no specific one
                    },
                    (0x48e, 11|12|13|15|63, 1|2|4) => {
                        dbg!(Ok(Some(EKnownDuAktion::Kartenwahl)))
                    },
                    _ => Err(()), // no expectation at all
                });
                info!("{:?}", resoknownduaktion_expected);
                if let Ok(_oknownduaktion_expected) = resoknownduaktion_expected {
                    let retval = unsafe{call_original(hwnd, u_msg, wparam, lparam)};
                    let src = unsafe{std::mem::transmute::<_,*const c_char>(0x004c8438)};
                    let str_status = String::from_utf8_lossy(
                        unsafe{scan_until_0(src as *const u8, None)}
                    );
                    info!("str_status: {}", str_status);
                    match str_status.borrow() {
                        "Du ? Kartenwahl" => {
                            // "Vorschlag machen"
                            verify_ne!(
                                unsafe{PostMessageA(
                                    hwnd,
                                    WM_COMMAND,
                                    105548,
                                    0,
                                )},
                                0
                            );
                            verify_ne!(
                                unsafe{PostMessageA(
                                    hwnd,
                                    WM_KEYDOWN,
                                    VK_UP as WPARAM,
                                    0,
                                )},
                                0
                            );
                        },
                        "Stich best\u{FFFD}tigen" => {
                            verify_ne!(
                                unsafe{PostMessageA(
                                    hwnd,
                                    WM_KEYDOWN,
                                    VK_UP as WPARAM,
                                    0,
                                )},
                                0
                            );
                        },
                        "Warten"
                            | ""
                            | "Der Computer mischt"
                            | "Du ? Spielen"
                            | "PcLinks ? Spielen"
                            | "PcOben ? Spielen"
                            | "PcRechts ? Spielen"
                            | "Du ? Spielwahl"
                            | "PcLinks ? Spielwahl"
                            | "PcOben ? Spielwahl"
                            | "PcRechts ? Spielwahl"
                            | "PcLinks ? Kartenwahl"
                            | "PcOben ? Kartenwahl"
                            | "PcRechts ? Kartenwahl"
                            | "Du ? Sto\u{FFFD}en"
                            | "PcLinks ? Sto\u{FFFD}en"
                            | "PcOben ? Sto\u{FFFD}en"
                            | "PcRechts ? Sto\u{FFFD}en"
                        => {},
                        str_unknown_msg => {
                            info!("Unknown status: {}", str_unknown_msg);
                        },
                    }
                    return retval
                } else {
                    let retval = unsafe{call_original(hwnd, u_msg, wparam, lparam)};
                    if 
                        (u_msg==NETSCHK_MSG_SPIELABFRAGE_1 && lparam==N_INDEX_GAST)
                        || (u_msg==NETSCHK_MSG_SPIELABFRAGE_2 && lparam==N_INDEX_GAST)
                    {
                        let hwnd_spielabfrage = unsafe{*(0x004bd4dc as *mut HWND)};
                        if 0!=unsafe{IsWindow(hwnd_spielabfrage)} {
                            let internal_click_button = |n_id_dlg_item| {
                                if let Err(str_error) = click_button(
                                    hwnd_spielabfrage,
                                    n_id_dlg_item,
                                    /*b_allow_invisible*/false,
                                    ESendOrPost::Send,
                                ) {
                                    info!("click_button failed: {}", str_error);
                                }
                            };
                            // TODO can we move this to dialogproc_spielabfrage?
                            match log_in_out(
                                "[manual] netschk_maybe_vorschlag_spielabfrage_1(N_INDEX_GAST)",
                                (),
                                || unsafe{netschk_maybe_vorschlag_spielabfrage_1(N_INDEX_GAST)},
                            ) {
                                0 => {
                                    "Weiter";
                                    internal_click_button(/*n_id_dlg_item: Button "Nein"*/1081)
                                },
                                1 => {
                                    "Rufspiel";
                                    internal_click_button(/*n_id_dlg_item: Button "Ja"*/1082)
                                },
                                2 => {
                                    "Farbgeier";
                                    internal_click_button(/*n_id_dlg_item: Button "Ja"*/1082)
                                },
                                3 => {
                                    "Geier";
                                    internal_click_button(/*n_id_dlg_item: Button "Ja"*/1082)
                                },
                                4 => {
                                    "Farbwenz";
                                    internal_click_button(/*n_id_dlg_item: Button "Ja"*/1082)
                                },
                                5 => {
                                    "Wenz";
                                    internal_click_button(/*n_id_dlg_item: Button "Ja"*/1082)
                                },
                                6 => {
                                    "Solo";
                                    internal_click_button(/*n_id_dlg_item: Button "Ja"*/1082)
                                },
                                7 => {
                                    "Bettel";
                                    internal_click_button(/*n_id_dlg_item: Button "Ja"*/1082)
                                },
                                9 => {
                                    "Wenz or Farbwenz Tout";
                                    internal_click_button(/*n_id_dlg_item: Button "Ja"*/1082)
                                },
                                _ => panic!(),
                            }
                        }
                    }
                    retval
                }
            },
        )
    },
);

make_redirect_function!(
    netschk_fill_regel_to_registry_bytes,
    /*pfn_original*/0x0042aa00,
    ("C") (pbyte_out: *mut u8,)->(),
    {
        log_in_out("fill_regel_to_registry_bytes", (pbyte_out,), |pbyte_out| {
            unsafe{call_original(pbyte_out)}
        })
    },
);
make_redirect_function!(
    netschk_read_regel_to_registry_bytes,
    /*pfn_original*/0x0042ab60,
    ("C") (pbyte: *const u8,)->(),
    {
        log_in_out("read_regel_to_registry_bytes", (pbyte,), |pbyte| {
            let retval = unsafe{call_original(pbyte)};
            unsafe{*PN_TOTAL_GAMES = 10000};
            retval
        })
    },
);
make_redirect_function!(
    netschk_maybe_vorschlag,
    /*pfn_original*/0x004356d0,
    ("C") (pchar_answer: *mut c_char, n_bytes: size_t,)->(),
    {
        log_in_out("maybe_vorschlag", (pchar_answer, n_bytes), |pchar_answer, n_bytes| {
            log_game();
            let retval = unsafe{call_original(pchar_answer, n_bytes)};
            info!("maybe_vorschlag: {}", String::from_utf8_lossy(
                unsafe{scan_until_0(pchar_answer as *const u8, n_bytes)}
            ));
            retval
        })
    },
);
fn internal_suggest(fn_call_original: &dyn Fn()->isize, b_improve_netschafkopf: bool) -> isize {
    let i_suggestion_netschk_1_based = fn_call_original();
    let (aveccard_netschafkopf, game) = unwrap!(log_game());
    let (epi_active, _vecepi_stoss) = unwrap!(game.which_player_can_do_something());
    if game.stichseq.remaining_cards_per_hand()[epi_active]<=if_dbg_else!({2}{3}) {
        let determinebestcardresult = unwrap!(determine_best_card(
            &game.stichseq,
            game.rules.as_ref(),
            Box::new(all_possible_hands(
                &game.stichseq,
                game.ahand[epi_active].clone(),
                epi_active,
                game.rules.as_ref(),
                &game.expensifiers.vecstoss,
            )),
            /*fn_make_filter*/SNoFilter::factory(),
            /*foreachsnapshot*/&SMinReachablePayout::new(
                game.rules.as_ref(),
                epi_active,
                game.expensifiers.clone(),
            ),
            SSnapshotCacheNone::factory(),
            SNoVisualization::factory(),
            /*fn_inspect*/&|_b_before, _i_ahand, _ahand, _card| {},
            /*epi_result*/epi_active,
            /*fn_payout*/&|_stichseq, _ahand, n_payout: isize| (n_payout, n_payout.cmp(&0)),
        ));
        let card_suggestion_netschk = aveccard_netschafkopf[epi_active][(i_suggestion_netschk_1_based-1).as_num::<usize>()];
        assert!(
            determinebestcardresult.cards_and_ts()
                .find(|&(card, _)| card==card_suggestion_netschk)
                .is_some()
        );
        let veccard_suggestion_openschafkopf = determinebestcardresult.cards_and_ts()
            .filter_map(|(card, payoutstatsperstrategy)| {
                let n_payout_relevant = payoutstatsperstrategy.0[EMinMaxStrategy::Min].min();
                if_then_some!(n_payout_relevant > 0, (card, n_payout_relevant))
            })
            .max_set_by_key(|&(_card, n_payout_relevant)| n_payout_relevant)
            .into_iter()
            .map(|(card, _n_payout_relevant)| card)
            .collect::<Vec<_>>();
        if !veccard_suggestion_openschafkopf.is_empty() {
            if !veccard_suggestion_openschafkopf.contains(&card_suggestion_netschk){
                let str_file_osk_replay = format!("{}_{}.sh",
                    game.stichseq.visible_cards().map(|(_epi, card)|card).join(""),
                    game.ahand[epi_active].cards().iter().join(""),
                );
                info!("Writing replay to {}", str_file_osk_replay);
                {
                    let str_rules = format!("{}{}",
                        game.rules,
                        if let Some(epi) = game.rules.playerindex() {
                            format!(" von {}", epi)
                        } else {
                            "".to_owned()
                        },
                    );
                    let mut file_osk_replay = unwrap!(File::create(str_file_osk_replay));
                    unwrap!(writeln!(&mut file_osk_replay, "echo '{}'", str_rules));
                    unwrap!(writeln!(&mut file_osk_replay, "echo 'Stichs so far:'"));
                    for stich in game.stichseq.visible_stichs() {
                        unwrap!(writeln!(&mut file_osk_replay, "echo '{}'", &stich));
                    }
                    unwrap!(writeln!(&mut file_osk_replay, "echo 'Hand: {}'", SDisplayCardSlice::new(game.ahand[epi_active].cards().clone(), &game.rules)));
                    unwrap!(writeln!(&mut file_osk_replay, "echo 'NetSchafkopf suggests {}'", card_suggestion_netschk));
                    unwrap!(writeln!(&mut file_osk_replay, "./target/release/openschafkopf suggest-card --rules \"{str_rules}\" --cards-on-table \"{str_cards_on_table}\" --hand \"{str_hand}\" --branching \"equiv7\" --points",
                        str_cards_on_table=game.stichseq.visible_stichs().iter()
                            .filter_map(|stich| if_then_some!(!stich.is_empty(), stich.iter().map(|(_epi, card)| *card).join(" ")))
                            .join("  "),
                        str_hand=SDisplayCardSlice::new(game.ahand[epi_active].cards().clone(), &game.rules),
                    ));
                }
                if b_improve_netschafkopf {
                    return verify_ne!(
                        (unwrap!(
                            aveccard_netschafkopf[epi_active]
                                .iter()
                                .position(|&card| card==veccard_suggestion_openschafkopf[0])
                        ) + 1).as_num::<isize>(),
                        i_suggestion_netschk_1_based
                    );
                }
            }
        }
    }
    i_suggestion_netschk_1_based
}
make_redirect_function!(
    netschk_maybe_vorschlag_suggest_card_1,
    /*pfn_original*/0x00433f90,
    ("C") ()->isize,
    {
        log_in_out("maybe_vorschlag_suggest_card_1", (), || internal_suggest(
            &|| unsafe{call_original()},
            /*b_improve_netschafkopf*/true, // TODO only for N_INDEX_GAST?
        ))
    },
);
make_redirect_function!(
    netschk_maybe_vorschlag_suggest_card_2,
    /*pfn_original*/0x0042fef0,
    ("C") ()->isize,
    {
        log_in_out("maybe_vorschlag_suggest_card_2", (), || internal_suggest(
            &|| unsafe{call_original()},
            /*b_improve_netschafkopf*/true, // TODO only for N_INDEX_GAST?
        ))
    },
);
make_redirect_function!(
    netschk_maybe_vorschlag_suggest_card_3,
    /*pfn_original*/0x0041b0b0,
    ("C") (n_unknown: isize,)->isize,
    {
        log_in_out("maybe_vorschlag_suggest_card_3", (n_unknown,), |n_unknown| {
            log_game();
            unsafe{call_original(n_unknown)}
        })
    },
);
make_redirect_function!(
    netschk_maybe_vorschlag_suggest_card_4,
    /*pfn_original*/0x0041b680,
    ("C") (n_unknown: isize,)->isize,
    {
        log_in_out("maybe_vorschlag_suggest_card_4", (n_unknown,), |n_unknown| {
            log_game();
            unsafe{call_original(n_unknown)}
        })
    },
);
make_redirect_function!(
    netschk_maybe_vorschlag_should_stoss,
    /*pfn_original*/0x0041a220,
    ("C") (n_unknown: isize,)->BOOL,
    {
        log_in_out("maybe_vorschlag_should_stoss", (n_unknown,), |n_unknown| {
            log_game();
            unsafe{call_original(n_unknown)}
        })
    },
);
make_redirect_function!(
    netschk_maybe_vorschlag_unknown_1,
    /*pfn_original*/0x004315e0,
    ("C") (n_unknown: isize,)->isize,
    {
        log_in_out("maybe_vorschlag_unknown_1", (n_unknown,), |n_unknown| {
            log_game();
            unsafe{call_original(n_unknown)}
        })
    },
);
make_redirect_function!(
    netschk_maybe_vorschlag_unknown_2,
    /*pfn_original*/0x0042aca0,
    ("C") (n_unknown: isize,)->isize,
    {
        log_in_out("maybe_vorschlag_unknown_2", (n_unknown,), |n_unknown| {
            log_game();
            unsafe{call_original(n_unknown)}
        })
    },
);
make_redirect_function!(
    netschk_maybe_vorschlag_spielabfrage_2,
    /*pfn_original*/0x00419a80,
    ("C") (n_unknown: isize,)->isize,
    {
        log_in_out("maybe_vorschlag_spielabfrage_2", (n_unknown,), |n_unknown| {
            log_game();
            unsafe{call_original(n_unknown)}
        })
    },
);
make_redirect_function!(
    netschk_maybe_vorschlag_unknown_4,
    /*pfn_original*/0x0042ad50,
    ("C") (n_unknown: isize,)->isize,
    {
        log_in_out("maybe_vorschlag_unknown_4", (n_unknown,), |n_unknown| {
            log_game();
            unsafe{call_original(n_unknown)}
        })
    },
);
make_redirect_function!(
    netschk_maybe_vorschlag_unknown_5,
    /*pfn_original*/0x0042ae20,
    ("C") (n_unknown: isize,)->isize,
    {
        log_in_out("maybe_vorschlag_unknown_5", (n_unknown,), |n_unknown| {
            log_game();
            unsafe{call_original(n_unknown)}
        })
    },
);
make_redirect_function!(
    netschk_maybe_vorschlag_spielabfrage_1,
    /*pfn_original*/0x0042acc0,
    ("C") (n_unknown: isize,)->isize,
    {
        log_in_out("maybe_vorschlag_spielabfrage_1", (n_unknown,), |n_unknown| {
            log_game();
            unsafe{call_original(n_unknown)}
        })
    },
);
fn get_module_symbol_address(module: &str, symbol: &str) -> FARPROC {
    // taken from https://github.com/Hpmason/retour-rs/blob/master/examples/messageboxw_detour.rs
    let module = module
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    info!("module {:?}", module);
    let symbol = unwrap!(CString::new(symbol));
    info!("symbol {:?}", symbol);
    unsafe {
        let handle = GetModuleHandleW(module.as_ptr() as _);
        info!("handle {:?}", handle);
        let procaddress = GetProcAddress(handle, symbol.as_ptr() as _);
        info!("procaddress {:?}", procaddress);
        procaddress
    }
}
make_redirect_function!(
    set_window_text_a,
    /*pfn_original*/get_module_symbol_address("user32.dll", "SetWindowTextA"),
    ("system") (hwnd: HWND, lpcstr: LPCSTR,)->BOOL,
    {
        log_in_out("SetWindowTextA", (hwnd, lpcstr,), |hwnd, lpcstr: LPCSTR| {
            //info!("SetWindowText {:?}", OsString::new(lpcstr));
            info!("SetWindowTextA: {:?}", CString::new(
                unsafe{scan_until_0(lpcstr as *const u8, None)}
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>()
            ));
            unsafe{call_original(hwnd, lpcstr)}
        })
    },
);
make_redirect_function!(
    post_message_a,
    /*pfn_original*/get_module_symbol_address("user32.dll", "PostMessageA"),
    ("system") (hwnd: HWND, n_msg: UINT, wparam: WPARAM, lparam: LPARAM,)->BOOL,
    {
        log_in_out("PostMessageA", (hwnd, n_msg, wparam, lparam), |hwnd, n_msg, wparam, lparam| {
            unsafe{call_original(hwnd, n_msg, wparam, lparam)}
        })
    },
);

fn highlight_button(hwnd_dialog: HWND, n_id_dlg_item: c_int) -> HWND {
    let hwnd_button = verify!(unsafe{GetDlgItem(
        hwnd_dialog,
        n_id_dlg_item,
    )});
    let mut vecch : Vec<CHAR> = vec![0; 100];
    let n_copied = unsafe{SendMessageA(
        hwnd_button,
        WM_GETTEXT,
        /*wparam: maximum number of characters to be copied*/vecch.len(),
        /*lparam: buffer*/vecch.as_mut_ptr() as LPARAM,
    )};
    assert!(0<n_copied);
    vecch.insert(0, '>' as CHAR);
    vecch.insert(0, '>' as CHAR);
    vecch.insert(0, ' ' as CHAR);
    verify_ne!(
        unsafe{SendMessageA(
            hwnd_button,
            WM_SETTEXT,
            /*wparam*/0, // unused as per documentation
            /*lparam*/vecch.as_mut_ptr() as LPARAM,
        )},
        FALSE as isize
    );
    hwnd_button
}

#[allow(dead_code)] // Send vs. Post useful for debugging
#[derive(Clone, Copy, Debug)]
enum ESendOrPost {Send, Post} // TODO even needed?

fn click_button(
    hwnd_dialog: HWND,
    n_id_dlg_item: c_int,
    b_allow_invisible: bool, // TODO this should also incorporate enabled (via an enumset)
    esendorpost: ESendOrPost,
) -> Result<(), String> {
    let hwnd_button = highlight_button(hwnd_dialog, n_id_dlg_item);
    if !b_allow_invisible && FALSE==unsafe{IsWindowVisible(hwnd_button)} {
        Err(format!("Button {n_id_dlg_item} invisible"))
    } else if FALSE==unsafe{IsWindowEnabled(hwnd_button)} {
        Err(format!("Button {n_id_dlg_item} disabled"))
    } else {
        match esendorpost {
            ESendOrPost::Send => {
                verify_eq!(
                    unsafe{SendMessageA(
                        hwnd_dialog,
                        WM_COMMAND,
                        std::mem::transmute(n_id_dlg_item),
                        hwnd_button as LPARAM,
                    )},
                    0
                );
            },
            ESendOrPost::Post => {
                verify_eq!(
                    unsafe{PostMessageA(
                        hwnd_dialog,
                        WM_COMMAND,
                        std::mem::transmute(n_id_dlg_item),
                        hwnd_button as LPARAM,
                    )},
                    TRUE
                );
            },
        }
        Ok(())
    }
}

make_redirect_function!(
    netschk_dialogproc_spielabfrage,
    /*pfn_original*/0x0040ecc0,
    ("system") (hwnd: HWND, n_msg: UINT, wparam: WPARAM, lparam: LPARAM,) -> INT_PTR,
    {
        const NETSCHK_MSG_MAG_AUCH: UINT = 0x476;
        const NETSCHK_MSG_SPIEL_BEKOMMEN: UINT = 0x47b;
        const NETSCHK_MSG_SPIEL_BEKOMMEN_NACH_PRIO: UINT = 0x478;
        log_in_out_cond(
            "dialogproc_spielabfrage",
            (hwnd, n_msg, wparam, lparam),
            |&(_hwnd, n_msg, _wparam, _lparam)| stringify_matches!(n_msg, (),
                WM_COMMAND
                WM_INITDIALOG
                NETSCHK_MSG_MAG_AUCH
                NETSCHK_MSG_SPIEL_BEKOMMEN
                NETSCHK_MSG_SPIEL_BEKOMMEN_NACH_PRIO
                0x473
                0x477
                0x479
                0x47a
                0x47c
                0x47d
                0x47e
                0x47f
                0x480
                0x481
                0x482
                0x483
                0x484
                0x485
                0x486
                0x487
            ),
            |hwnd, n_msg, wparam, lparam| {
                let res = unsafe{call_original(hwnd, n_msg, wparam, lparam)};
                if 
                    n_msg==NETSCHK_MSG_SPIEL_BEKOMMEN
                    || n_msg==NETSCHK_MSG_SPIEL_BEKOMMEN_NACH_PRIO
                    || n_msg==NETSCHK_MSG_MAG_AUCH
                {
                    let hwnd_spielabfrage = unsafe{*(0x004bd4dc as *mut HWND)};
                    assert_eq!(hwnd, hwnd_spielabfrage);
                    let mut vecch_orig : Vec<CHAR> = vec![0; 1000];
                    unsafe{netschk_maybe_vorschlag(vecch_orig.as_mut_ptr(), vecch_orig.len())};
                    let vecch = vecch_orig.into_iter()
                        .map(|c| c as u8)
                        .filter(|&c| c!=0)
                        .collect::<Vec<_>>();
                    let (str_rules_kind, ostr_farbe) : (&'static [u8], Option<&'static [u8]>) =
                    if vecch==b"Weiter" {
                        (b"Weiter", None)
                    } else if vecch==b"Mit der Eichel-Ass" {
                        (b"Rufspiel", Some(b"Mit der Eichel-Ass"))
                    } else if vecch==b"Mit der Gr\xFCn-Ass" {
                        (b"Rufspiel", Some(b"Mit der Gr\xFCn-Ass"))
                    } else if vecch==b"Mit der Schellen-Ass" {
                        (b"Rufspiel", Some(b"Mit der Schellen-Ass"))
                    } else if vecch==b"Eichel-Solo" {
                        (b"Solo", Some(b"Eichel"))
                    } else if vecch==b"Gr\xFCn-Solo" {
                        (b"Solo", Some(b"Gr\xFCn"))
                    } else if vecch==b"Herz-Solo" {
                        (b"Solo", Some(b"Herz"))
                    } else if vecch==b"Schellen-Solo" {
                        (b"Solo", Some(b"Schellen"))
                    } else if vecch==b"Wenz" {
                        (b"Wenz", None)
                    } else if vecch==b"Eichel-Wenz" {
                        (b"Farbwenz", Some(b"Eichel"))
                    } else if vecch==b"Gr\xFCn-Wenz" {
                        (b"Farbwenz", Some(b"Gr\xFCn"))
                    } else if vecch==b"Herz-Wenz" {
                        (b"Farbwenz", Some(b"Herz"))
                    } else if vecch==b"Schellen-Wenz" {
                        (b"Farbwenz", Some(b"Schellen"))
                    } else if vecch==b"Geier" {
                        (b"Geier", None)
                    } else if vecch==b"Eichel-Geier" {
                        (b"Farbgeier", Some(b"Eichel"))
                    } else if vecch==b"Gr\xFCn-Geier" {
                        (b"Farbgeier", Some(b"Gr\xFCn"))
                    } else if vecch==b"Herz-Geier" {
                        (b"Farbgeier", Some(b"Herz"))
                    } else if vecch==b"Schellen-Geier" {
                        (b"Farbgeier", Some(b"Schellen"))
                    } else if vecch==b"Eichel-Solo Tout" {
                        (b"Solo Tout", Some(b"Eichel"))
                    } else if vecch==b"Gr\xFCn-Solo Tout" {
                        (b"Solo Tout", Some(b"Gr\xFCn"))
                    } else if vecch==b"Herz-Solo Tout" {
                        (b"Solo Tout", Some(b"Herz"))
                    } else if vecch==b"Schellen-Solo Tout" {
                        (b"Solo Tout", Some(b"Schellen"))
                    } else if vecch==b"Wenz Tout" {
                        (b"Wenz Tout", Some(b"Farblos"))
                    } else if vecch==b"Eichel-Wenz Tout" {
                        (b"Wenz Tout", Some(b"Eichel"))
                    } else if vecch==b"Gr\xFCn-Wenz Tout" {
                        (b"Wenz Tout", Some(b"Gr\xFCn"))
                    } else if vecch==b"Herz-Wenz Tout" {
                        (b"Wenz Tout", Some(b"Herz"))
                    } else if vecch==b"Schellen-Wenz Tout" {
                        (b"Wenz Tout", Some(b"Schellen"))
                    } else if vecch==b"Geier Tout" {
                        (b"Geier Tout", Some(b"Farblos"))
                    } else if vecch==b"Eichel-Geier Tout" {
                        (b"Geier Tout", Some(b"Eichel"))
                    } else if vecch==b"Gr\xFCn-Geier Tout" {
                        (b"Geier Tout", Some(b"Gr\xFCn"))
                    } else if vecch==b"Herz-Geier Tout" {
                        (b"Geier Tout", Some(b"Herz"))
                    } else if vecch==b"Schellen-Geier Tout" {
                        (b"Geier Tout", Some(b"Schellen"))
                    } else if vecch==b"Sie" {
                        (b"Solo Tout", Some(b"Sie"))
                    } else if vecch==b"Bettel" {
                        (b"Bettel", None)
                    } else {
                        panic!("Unknown Vorschlag: {:?}", vecch)
                    };
                    // TODO interpret vecch (or go one level deeper) and select
                    let select_item = |n_id_list: u16, str_item: &[u8]| {
                        let str_item = str_item.iter().copied().chain(std::iter::once(0)).collect::<Vec<_>>();
                        let hwnd_list = unsafe{GetDlgItem(hwnd_spielabfrage, n_id_list as _)}; // TODO verify
                        verify_ne!(
                            unsafe{SendMessageA(
                                hwnd_list,
                                LB_SELECTSTRING,
                                std::mem::transmute(-1), // search entire list
                                std::mem::transmute(str_item.as_ptr()),
                            )},
                            LB_ERR
                        );
                        verify_ne!(
                            unsafe{SendMessageA(
                                hwnd_spielabfrage,
                                WM_COMMAND,
                                std::mem::transmute(make_dword(
                                    n_id_list,
                                    LBN_SELCHANGE,
                                )),
                                std::mem::transmute(hwnd_list),
                            )},
                            LB_ERR
                        );
                    };
                    select_item(/*rule kind list*/1287, str_rules_kind);
                    if let Some(str_farbe) = ostr_farbe {
                        select_item(/*Farbe/Farblos list*/1288, str_farbe);
                    }
                    if let Err(str_error) = click_button(
                        hwnd_spielabfrage, // TODO verify_eq
                        /*n_id_dlg_item: Button "Fertig"*/1097,
                        /*b_allow_invisible*/false,
                        ESendOrPost::Send,
                    ) {
                        info!("click_button failed: {}", str_error);
                    }
                }
                res
            }
        )
    },
);
make_redirect_function!(
    netschk_dialogproc_contra_geben,
    /*pfn_original*/0x00413d20,
    ("system") (hwnd: HWND, n_msg: UINT, wparam: WPARAM, lparam: LPARAM,) -> INT_PTR,
    {
        log_in_out_cond(
            "dialogproc_contra_geben",
            (hwnd, n_msg, wparam, lparam),
            |&(_hwnd, n_msg, _wparam, _lparam)| stringify_matches!(n_msg, (),
                WM_COMMAND
                WM_INITDIALOG
                WM_SHOWWINDOW
            ),
            |hwnd, n_msg, wparam, lparam| {
                let retval = unsafe{call_original(hwnd, n_msg, wparam, lparam)};
                if n_msg==WM_SHOWWINDOW {
                    unwrap!(click_button(
                        hwnd,
                        /*n_id_dlg_item*/if TRUE==unsafe{netschk_maybe_vorschlag_should_stoss(N_INDEX_GAST)} {
                            /*Ja*/1082
                        } else {
                            /*Nein*/1081
                        },
                        /*b_allow_invisible*/true, // WM_SHOWWINDOW called very early
                        ESendOrPost::Send,
                    ));
                }
                retval
            },
        )
    },
);
make_redirect_function!(
    netschk_dialogproc_analyse_weiter_1,
    /*pfn_original*/0x00412050,
    ("system") (hwnd: HWND, n_msg: UINT, wparam: WPARAM, lparam: LPARAM,) -> INT_PTR,
    {
        log_in_out(
            "dialogproc_analyse_weiter_1",
            (hwnd, n_msg, wparam, lparam),
            |hwnd, n_msg, wparam, lparam| {
                unsafe{call_original(hwnd, n_msg, wparam, lparam)}
            },
        )
    },
);
make_redirect_function!(
    netschk_dialogproc_analyse_weiter_2_maybe_ja_nein_1,
    /*pfn_original*/0x00412050,
    ("system") (hwnd: HWND, n_msg: UINT, wparam: WPARAM, lparam: LPARAM,) -> INT_PTR,
    {
        log_in_out(
            "dialogproc_analyse_weiter_2_maybe_ja_nein",
            (hwnd, n_msg, wparam, lparam),
            |hwnd, n_msg, wparam, lparam| {
                let retval = unsafe{call_original(hwnd, n_msg, wparam, lparam)};
                if n_msg==WM_SHOWWINDOW {
                    unwrap!(click_button(
                        hwnd,
                        /*weiter*/1083,
                        /*b_allow_invisible*/true, // WM_SHOWWINDOW called very early
                        ESendOrPost::Send,
                    ));
                }
                retval
            },
        )
    },
);
make_redirect_function!(
    netschk_wndproc_status_bar,
    /*pfn_original*/0x0045f940,
    ("system") (hwnd: HWND, n_msg: UINT, wparam: WPARAM, lparam: LPARAM,) -> INT_PTR,
    {
        log_in_out(
            "wndproc_status_bar",
            (hwnd, n_msg, wparam, lparam),
            |hwnd, n_msg, wparam, lparam| {
                unsafe{call_original(hwnd, n_msg, wparam, lparam)}
            },
        )
    },
);

const N_BYTES_PER_NETSCHAFKOPF_CARD: usize = 3;

unsafe fn interpret_as_cards(pbyte: *const u8, n_cards_max: usize) -> Vec<ECard> {
    let slcbyte = std::slice::from_raw_parts(pbyte, n_cards_max * N_BYTES_PER_NETSCHAFKOPF_CARD);
    let mut veccard = Vec::new();
    while veccard.len() < n_cards_max && {
        let i_byte = veccard.len() * N_BYTES_PER_NETSCHAFKOPF_CARD;
        bytes_are_card(unwrap!(slcbyte[i_byte..i_byte+N_BYTES_PER_NETSCHAFKOPF_CARD].try_into()))
            .map(|card| veccard.push(card))
            .is_some()
    } {}
    veccard
}

static mut B_LOG_GAME : bool = true;

fn log_game() -> Option<(EnumMap<EPlayerIndex, Vec<ECard>>, SGame)> {
    log_in_out_cond("log_game", (), |_| if_then_some!(unsafe{B_LOG_GAME},()), || {
        let pbyte_card_stack = 0x004bd500 as *const u8;
        const N_CARDS_STACK : usize = 33;
        info!("Card stack including 0: {}",
            unsafe{interpret_as_cards(
                pbyte_card_stack,
                /*n_cards_max*/N_CARDS_STACK
            ).iter().join(" ")},
        );
        info!("Card stack excluding 0: {}",
            unsafe{interpret_as_cards(
                pbyte_card_stack.add(N_BYTES_PER_NETSCHAFKOPF_CARD),
                /*n_cards_max*/N_CARDS_STACK-1
            ).iter().join(" ")},
        );
        let astr_player = ["links", "oben", "rechts", "gast"];
        let aveccard_hand = [0x4b5e67, 0x4b5e8b, 0x4b5eaf, 0x4b5ed3].map(|pbyte_hand: usize|
            unsafe {interpret_as_cards(std::mem::transmute(pbyte_hand), /*n_cards_max*/8)}
        );
        let aveccard_played = [0x4c60de, 0x4c60f9, 0x4c6114, 0x4c612f].map(|pbyte_played: usize|
            unsafe {interpret_as_cards(std::mem::transmute(pbyte_played), /*n_cards_max*/8)}
        );
        for (str_player, veccard_hand) in astr_player.iter().zip_eq(aveccard_hand.iter()) {
            info!("Hand von {}: {}",
                str_player,
                veccard_hand.iter().join(" "),
            );
        }
        for (str_player, veccard_played) in astr_player.iter().zip_eq(aveccard_played.iter()) {
            info!("Gespielte Karten von {}: {}",
                str_player,
                veccard_played.iter().join(" "),
            );
        }
        let i_netschafkopf_geber = unsafe{*std::mem::transmute::<_, *const usize>(0x004ca578)};
        info!("Geber: {}", i_netschafkopf_geber);
        let n_stichs_completed = unsafe{*std::mem::transmute::<_, *const usize>(0x004b5988)};
        info!("# komplette Stiche: {}", n_stichs_completed);
        let n_current_stich_size = unsafe{*std::mem::transmute::<_, *const usize>(0x004963e4)};
        info!("# played cards in current stich: {}", n_current_stich_size);
        info!("g_iEPIPresumablyNextCard: {}",
            unsafe{*std::mem::transmute::<_, *const isize>(0x004b596c)}
        );
        let str_rules_pri = String::from_utf8_lossy(
            unsafe{scan_until_0(0x004ad0cc as *const u8, 260)}
        );
        let str_active_player = String::from_utf8_lossy(
            unsafe{scan_until_0(0x004ad1d0 as *const u8, 260)}
        );
        let (str_rules_pri, ostr_active_player) = if str_rules_pri=="Jungfrau" || str_rules_pri=="hat Stich" {
            (Cow::Borrowed("Ramsch"), None)
        } else {
            (str_rules_pri, Some(str_active_player))
        };
        let str_rules = format!("{}{}", str_rules_pri, if let Some(str_active_player)=ostr_active_player {
            format!(" von {}", str_active_player)
        } else {
            "".to_string()
        });
        info!("Rules: {str_rules}");
        let to_openschafkopf_playerindex = |i_netschafkopf_player: usize| {
            assert!(1 <= i_netschafkopf_geber);
            assert!(i_netschafkopf_geber <= 4);
            assert!(1 <= i_netschafkopf_player);
            assert!(i_netschafkopf_player <= 4);
            unwrap!(EPlayerIndex::checked_from_usize(
                (i_netschafkopf_player + 8 - (i_netschafkopf_geber+1)) % 4
            ))
        };
        let epi_to_netschafkopf_playerindex = |epi: EPlayerIndex| {
            let mut i_netschafkopf = (i_netschafkopf_geber + 1 + epi.to_usize())%4;
            if i_netschafkopf==0 {
                i_netschafkopf = 4;
            }
            i_netschafkopf
        };
        let n_stichs_remaining = unsafe{*std::mem::transmute::<_, *const usize>(0x004963b8)};
        info!("n_stichs_remaining: {}", n_stichs_remaining);
        let n_presumably_total_games = unsafe{*PN_TOTAL_GAMES};
        info!("n_presumably_total_games: {}", n_presumably_total_games);
        if_then_some!("Normal"!=str_rules_pri, {
            let rules = unwrap!(parse_rule_description(
                &str_rules,
                (/*n_tarif_extra*/10, /*n_tarif_ruf*/20, /*n_tarif_solo*/50), // TODO extract from NetSchafkopf
                SStossParams::new(/*n_stoss_max*/4), // TODO extract from NetSchafkopf
                /*fn_player_to_epi*/|str_player| Ok(to_openschafkopf_playerindex(match str_player {
                    // TODO extract from NetSchafkopf
                    "PcLinks" => 1,
                    "PcOben" => 2,
                    "PcRechts" => 3,
                    "Du selbst" => 4,
                    _ => panic!("Unknown value for str_player: {}", str_player),
                }))
            ));
            info!("{}", rules);
            let ekurzlang = EKurzLang::Lang; // TODO extract from NetSchafkopf
            let mut stichseq = SStichSequence::new(ekurzlang);
            for _i_card in 0..n_stichs_completed*EPlayerIndex::SIZE + n_current_stich_size {
                stichseq.zugeben(
                    aveccard_played
                        [epi_to_netschafkopf_playerindex(unwrap!(stichseq.current_stich().current_playerindex()))-1]
                        [stichseq.completed_stichs().len()],
                    rules.as_ref(),
                );
            }
            info!("{:?}", stichseq);
            let an_cards_hand = stichseq.remaining_cards_per_hand();
            let aveccard_netschafkopf = EPlayerIndex::map_from_fn(|epi| {
                aveccard_hand[epi_to_netschafkopf_playerindex(epi)-1][0..an_cards_hand[epi]]
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
            });
            let ahand = aveccard_netschafkopf.map(|veccard| SHand::new_from_iter(veccard));
            info!("{}", display_card_slices(&ahand, &rules, " | "));
            // TODO extract stoss from NetSchafkopf
            (
                aveccard_netschafkopf,
                unwrap!(
                    SGame::new_with(
                        /*aveccard*/EPlayerIndex::map_from_fn(|epi| // TODO extract from NetSchafkopf - should be the cards in order they were dealt
                            stichseq.cards_from_player(&ahand[epi], epi).collect()
                        ),
                        SExpensifiersNoStoss::new_with_doublings(
                            /*n_stock*/0, // TODO extract from NetSchafkopf
                            /*doublings*/SDoublings::new(SStaticEPI0{}), // TODO extract from NetSchafkopf
                        ),
                        rules,
                        /*ruleset*/(), // TODO extract from NetSchafkopf
                        /*gameannouncements*/(), // TODO extract from NetSchafkopf
                        /*determinerules*/(), // TODO extract from NetSchafkopf
                    )
                    .play_cards_and_stoss(
                        /*itstoss*/std::iter::empty::<SStoss>(), // TODO extract from NetSchafkopf
                        /*ittplepicard*/stichseq.visible_cards(),
                        /*fn_before_zugeben*/|_,_,_,_|(),
                    )
                ),
            )
        })
    })
}

const PN_TOTAL_GAMES : *mut usize = 0x004c60ac as *mut usize;

fn initialize() {
    let path_user_data = unwrap!(dirs::data_dir());
    unwrap!(fs::create_dir_all(&path_user_data));
    unwrap!(simple_logging::log_to_file(
        path_user_data.join("netschafkopf_helper.log"),
        log::LevelFilter::Info,
    ));
    info!("initialize <- (after logging setup)");
    info!("pid: {}", std::process::id());
    info!("process_path: {}", unwrap!(std::env::current_exe()).display());

    unsafe{
        netschk_strcpy_s::redirect();
        if false { // Redirecting this function is very, very slow
            netschk_increment_playerindex::redirect();
        }
        netschk_process_window_message::redirect();
        netschk_fill_regel_to_registry_bytes::redirect();
        netschk_read_regel_to_registry_bytes::redirect();
        netschk_maybe_vorschlag::redirect();
        netschk_maybe_vorschlag_suggest_card_1::redirect();
        netschk_maybe_vorschlag_suggest_card_2::redirect();
        netschk_maybe_vorschlag_suggest_card_3::redirect();
        netschk_maybe_vorschlag_suggest_card_4::redirect();
        netschk_maybe_vorschlag_should_stoss::redirect();
        netschk_maybe_vorschlag_spielabfrage_1::redirect();
        netschk_maybe_vorschlag_spielabfrage_2::redirect();
        netschk_maybe_vorschlag_unknown_1::redirect();
        netschk_maybe_vorschlag_unknown_2::redirect();
        netschk_maybe_vorschlag_unknown_4::redirect();
        netschk_maybe_vorschlag_unknown_5::redirect();
        netschk_dialogproc_spielabfrage::redirect();
        netschk_dialogproc_contra_geben::redirect();
        netschk_dialogproc_analyse_weiter_1::redirect();
        netschk_dialogproc_analyse_weiter_2_maybe_ja_nein_1::redirect();
        netschk_wndproc_status_bar::redirect();
        set_window_text_a::redirect();
        post_message_a::redirect();
    }

    let fn_panic_handler_original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panicinfo| {
        error!("panic: {}", panicinfo);
        log_game();
        fn_panic_handler_original(panicinfo)
    }));

    info!("initialize ->");
}
