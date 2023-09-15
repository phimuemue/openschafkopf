#![cfg(windows)]

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
    borrow::Borrow,
    fs,
    ffi::{CString, c_char, c_int},
    fmt::Debug,
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

fn byte_is_farbe(byte: u8) -> bool {
    "EGHS".bytes().find(|c| c==&byte).is_some()
}
fn byte_is_schlag(byte: u8) -> bool {
    "789ZUOKA".bytes().find(|c| c==&byte).is_some()
}
fn bytes_are_card(slcbyte: &[u8]) -> bool {
    assert_eq!(3, slcbyte.len()); // TODO can we make this check at compile time
    byte_is_farbe(slcbyte[0])
        && byte_is_schlag(slcbyte[1])
        && {assert!(slcbyte[2]==0); true} // TODO use verify
}

#[allow(dead_code)]
unsafe fn log_bytes(pbyte: *const u8, n_bytes: usize) {
    for byte in unsafe{std::slice::from_raw_parts(pbyte, n_bytes)} {
        info!("{:0<3} {:#x} {}", byte, byte, char::from(*byte));
    }
}

trait UnpackAndApplyFn<Args, Return> {
    fn apply(self, args: Args) -> Return;
}
macro_rules! impl_unpack_and_apply_fn{($($T:ident)*) => {
    impl<$($T,)* R, F: FnOnce($($T,)*)->R> UnpackAndApplyFn<($($T,)*), R> for F {
        #[allow(non_snake_case)]
        fn apply(self, ($($T,)*): ($($T,)*)) -> R {
            self($($T,)*)
        }
    }
}}
impl_unpack_and_apply_fn!();
impl_unpack_and_apply_fn!(T0);
impl_unpack_and_apply_fn!(T0 T1);
impl_unpack_and_apply_fn!(T0 T1 T2 T3);
fn make_const_unpack_and_apply<Args, Return>(r: Return) -> impl UnpackAndApplyFn<Args, Return> {
    struct SConstUnpackAndApply<Return>(Return);
    impl<Args, Return> UnpackAndApplyFn<Args, Return> for SConstUnpackAndApply<Return> {
        fn apply(self, _args: Args) -> Return {
            self.0
        }
    }
    SConstUnpackAndApply(r)
}

fn log_in_out_cond<
    Args: Clone+Debug,
    ShouldLog: Debug,
    R: Debug,
    FnCond: UnpackAndApplyFn</*TODO could borrow?*/Args, Option<ShouldLog>>,
    F: UnpackAndApplyFn<Args, R>,
>(
    str_f: &str,
    args: Args,
    fn_cond: FnCond,
    f: F
) -> R {
    if let Some(shouldlog) = fn_cond.apply(args.clone()) {
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
    log_in_out_cond(str_f, args, make_const_unpack_and_apply(Some("")), f)
}

macro_rules! make_redirect_function(
    (
        $mod_fn_redirect:ident,
        $pfn_original:expr,
        ($($extern:tt)*) ($($paramname:ident : $paramtype:ty,)*)->$rettype:ty,
        $fn_new:expr,
    ) => {
        mod $mod_fn_redirect {
            use super::*;
            use retour::GenericDetour;

            static mut OHOOK: Option<GenericDetour<
                unsafe extern $($extern)* fn ($($paramname: $paramtype,)*)->$rettype
            >> = None;

            #[inline(always)]
            pub unsafe fn call_original(
                $($paramname: $paramtype,)*
            ) -> $rettype {
                OHOOK.as_ref().unwrap().call($($paramname,)*)
            }

            pub unsafe extern $($extern)* fn redirected($($paramname: $paramtype,)*)->$rettype {
                $fn_new
            }

            pub unsafe fn redirect() {
                log_in_out(&format!("{}::redirect", stringify!($mod_fn_redirect)), (), || {
                    let pfn_original: unsafe extern $($extern)* fn($($paramtype,)*)->$rettype =
                        std::mem::transmute($pfn_original);
                    OHOOK = Some(GenericDetour::new(pfn_original, redirected).unwrap());
                    OHOOK.as_ref().unwrap().enable().unwrap();
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
        log_in_out("increment_playerindex", (i_epi,), |i_epi| call_original(i_epi))
    },
);

#[derive(Debug, PartialEq, Eq, Clone)]
enum EKnownDuAktion {
    Kartenwahl,
    // StichBestaetigen,
}

use std::thread_local;
thread_local!(
    static PHWND_MAIN : *const HWND = unsafe{std::mem::transmute(0x004b6538)};
);

make_redirect_function!(
    netschk_strcpy_s,
    /*pfn_original*/0x00473757,
    ("C") (dst: *mut c_char, n_bytes_requested: rsize_t, src: *const c_char,)->errno_t,
    {
        let res = netschk_strcpy_s::call_original(
            dst,
            n_bytes_requested,
            src,
        );
        let ach_card : &[u8; 3] = std::mem::transmute(src);
        if n_bytes_requested==3 && bytes_are_card(&ach_card[0..3]) {
            info!("Moving card {}{}: {:?} => {:?}",
                char::from(ach_card[0]),
                char::from(ach_card[1]),
                src,
                dst,
            );
        } else {
            let mut pch = src;
            let mut n_bytes_before_0 = 0;
            while *pch!=0 && n_bytes_before_0 <= n_bytes_requested {
                pch = pch.add(1);
                n_bytes_before_0 += 1;
            }
            let str_src = String::from_utf8_lossy(
                unsafe{std::slice::from_raw_parts(src as *const u8, n_bytes_before_0)},
            );
            info!("strcpy_s: {:?} => {:?}: {}",
                dst,
                src,
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
            |_hwnd, u_msg, _wparam, _lparam| match u_msg {
                WM_KEYDOWN => {
                    if wparam==std::mem::transmute(VK_LEFT) || wparam==std::mem::transmute(VK_RIGHT) {
                        Some(format!(
                            "WM_KEYDOWN, VK_LEFT/VK_RIGHT: {:?}",
                            unsafe{std::slice::from_raw_parts(std::mem::transmute::<_, *const u8>(0x004ca2b0), 4)},
                        ))
                    } else if
                        0!=(GetKeyState(VK_CONTROL)&std::mem::transmute::<_,SHORT>(0x8000u16))
                        && wparam==std::mem::transmute(0x4F) // "O key" https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
                    {
                        Some(format!("WM_KEYDOWN, Ctrl+O"))
                    } else {
                        None
                    }
                },
                WM_COMMAND => {
                    Some(format!("WM_COMMAND: {}, {}", wparam, lparam))
                },
                NETSCHK_MSG_SPIELABFRAGE_1 => {
                    Some(format!("NETSCHK_MSG NETSCHK_MSG_SPIELABFRAGE_1, {}, {}", wparam, lparam))
                },
                NETSCHK_MSG_SPIELABFRAGE_2 => {
                    Some(format!("NETSCHK_MSG NETSCHK_MSG_SPIELABFRAGE_2, {}, {}", wparam, lparam))
                },
                NETSCHK_MSG_AKTIONSABFRAGE => {
                    Some(format!("NETSCHK_MSG NETSCHK_MSG_AKTIONSABFRAGE, {}, {}", wparam, lparam))
                },
                NETSCHK_MSG_KARTEGESPIELT => {
                    Some(format!("NETSCHK_MSG NETSCHK_MSG_KARTEGESPIELT, {}, {}", wparam, lparam))
                },
                netschk_msg@WM_USER..=0xffff => {
                    Some(format!("NETSCHK_MSG {:#x}, {}, {}", netschk_msg, wparam, lparam))
                },
                _ => None,
            },
            |hwnd, u_msg, wparam, lparam| {
                PHWND_MAIN.with(|&phwnd_main| assert_eq!(hwnd, unsafe{*phwnd_main}));
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
                    let retval = call_original(hwnd, u_msg, wparam, lparam);
                    let src = unsafe{std::mem::transmute::<_,*const c_char>(0x004c8438)};
                    let mut pch = src;
                    let mut n_bytes_before_0 = 0;
                    while *pch!=0 {
                        pch = pch.add(1);
                        n_bytes_before_0 += 1;
                    }
                    let str_status = String::from_utf8_lossy(
                        unsafe{std::slice::from_raw_parts(src as *const u8, n_bytes_before_0)},
                    );
                    info!("str_status: {}", str_status);
                    match str_status.borrow() {
                        "Du ? Kartenwahl" => {
                            // "Vorschlag machen"
                            let n_postemssage = PostMessageA(
                                hwnd,
                                WM_COMMAND,
                                105548,
                                0,
                            );
                            assert!(0!=n_postemssage);
                            let n_postemssage = PostMessageA(
                                hwnd,
                                WM_KEYDOWN,
                                VK_UP as WPARAM,
                                0,
                            );
                            assert!(0!=n_postemssage);
                        },
                        "Stich best\u{FFFD}tigen" => {
                            let n_postemssage = PostMessageA(
                                hwnd,
                                WM_KEYDOWN,
                                VK_UP as WPARAM,
                                0,
                            );
                            assert!(0!=n_postemssage);
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
                    let retval = netschk_process_window_message::call_original(hwnd, u_msg, wparam, lparam);
                    if 
                        (u_msg==NETSCHK_MSG_SPIELABFRAGE_1 && lparam==N_INDEX_GAST)
                        || (u_msg==NETSCHK_MSG_SPIELABFRAGE_2 && lparam==N_INDEX_GAST)
                    {
                        let hwnd_spielabfrage = *(0x004bd4dc as *mut HWND);
                        if 0!=IsWindow(hwnd_spielabfrage) {
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
                                || netschk_maybe_vorschlag_spielabfrage_1::redirected(N_INDEX_GAST),
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
    netschk_maybe_vorschlag,
    /*pfn_original*/0x004356d0,
    ("C") (pchar_unknown: *mut c_char, n_unknown: size_t,)->(),
    {
        log_in_out("maybe_vorschlag", (pchar_unknown, n_unknown), |pchar_unknown, n_unknown| {
            log_game();
            let retval = call_original(pchar_unknown, n_unknown);
            let mut pch = pchar_unknown;
            let mut n_bytes_before_0 = 0;
            while *pch!=0 && n_bytes_before_0 <= n_unknown {
                pch = pch.add(1);
                n_bytes_before_0 += 1;
            }
            let str_unknown = String::from_utf8_lossy(
                unsafe{std::slice::from_raw_parts(pchar_unknown as *const u8, n_bytes_before_0)},
            );
            info!("maybe_vorschlag: {}", str_unknown);
            retval
        })
    },
);
make_redirect_function!(
    netschk_maybe_vorschlag_suggest_card_1,
    /*pfn_original*/0x00433f90,
    ("C") ()->isize,
    {
        log_in_out("maybe_vorschlag_suggest_card_1", (), || {
            log_game();
            call_original()
        })
    },
);
make_redirect_function!(
    netschk_maybe_vorschlag_suggest_card_2,
    /*pfn_original*/0x0042fef0,
    ("C") ()->isize,
    {
        log_in_out("maybe_vorschlag_suggest_card_2", (), || {
            log_game();
            call_original()
        })
    },
);
make_redirect_function!(
    netschk_maybe_vorschlag_suggest_card_3,
    /*pfn_original*/0x0041b0b0,
    ("C") (n_unknown: isize,)->isize,
    {
        log_in_out("maybe_vorschlag_suggest_card_3", (n_unknown,), |n_unknown| {
            log_game();
            call_original(n_unknown)
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
            call_original(n_unknown)
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
            call_original(n_unknown)
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
            call_original(n_unknown)
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
            call_original(n_unknown)
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
            call_original(n_unknown)
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
            call_original(n_unknown)
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
            call_original(n_unknown)
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
            call_original(n_unknown)
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
    let symbol = CString::new(symbol).unwrap();
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
            let mut pch = lpcstr;
            let mut i = 0;
            while *pch!=0 {
                pch = pch.add(1);
                i += 1;
            }
            info!("SetWindowTextA: {:?}", CString::new(
                std::slice::from_raw_parts(lpcstr, i)
                    .iter()
                    .map(|&c| c as u8)
                    .collect::<Vec<_>>()
            ));
            call_original(hwnd, lpcstr)
        })
    },
);
make_redirect_function!(
    post_message_a,
    /*pfn_original*/get_module_symbol_address("user32.dll", "PostMessageA"),
    ("system") (hwnd: HWND, n_msg: UINT, wparam: WPARAM, lparam: LPARAM,)->BOOL,
    {
        log_in_out("PostMessageA", (hwnd, n_msg, wparam, lparam), |hwnd, n_msg, wparam, lparam| {
            call_original(hwnd, n_msg, wparam, lparam)
        })
    },
);

fn highlight_button(hwnd_dialog: HWND, n_id_dlg_item: c_int) -> HWND {
    let hwnd_button = unsafe{GetDlgItem(
        hwnd_dialog,
        n_id_dlg_item,
    )};
    assert!(hwnd_button!=std::ptr::null_mut());
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
    let b_success = unsafe{SendMessageA(
        hwnd_button,
        WM_SETTEXT,
        /*wparam*/0, // unused as per documentation
        /*lparam*/vecch.as_mut_ptr() as LPARAM,
    )};
    assert!(b_success!=(FALSE as isize));
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
                let n_sendmessage = unsafe{SendMessageA(
                    hwnd_dialog,
                    WM_COMMAND,
                    std::mem::transmute(n_id_dlg_item),
                    hwnd_button as LPARAM,
                )};
                assert_eq!(n_sendmessage, 0);
            },
            ESendOrPost::Post => {
                let b_postmessage = unsafe{PostMessageA(
                    hwnd_dialog,
                    WM_COMMAND,
                    std::mem::transmute(n_id_dlg_item),
                    hwnd_button as LPARAM,
                )};
                assert_eq!(b_postmessage, TRUE);
            },
        }
        Ok(()) // TODO? if_then_ok?
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
            |_hwnd, n_msg, _wparam, _lparam| {
                if n_msg==WM_COMMAND {
                    Some("WM_COMMAND".to_owned())
                } else if n_msg==WM_INITDIALOG {
                    Some("WM_INITDIALOG".to_owned())
                } else if n_msg==NETSCHK_MSG_MAG_AUCH {
                    Some("NETSCHK_MSG_MAG_AUCH".to_owned())
                } else if n_msg==NETSCHK_MSG_SPIEL_BEKOMMEN {
                    Some("NETSCHK_MSG_SPIEL_BEKOMMEN".to_owned())
                } else if n_msg==NETSCHK_MSG_SPIEL_BEKOMMEN_NACH_PRIO {
                    Some("NETSCHK_MSG_SPIEL_BEKOMMEN_NACH_PRIO".to_owned())
                } else if matches!(n_msg,
                    0x473
                    | 0x477
                    | 0x479
                    | 0x47a
                    | 0x47c
                    | 0x47d
                    | 0x47e
                    | 0x47f
                    | 0x480
                    | 0x481
                    | 0x482
                    | 0x483
                    | 0x484
                    | 0x485
                    | 0x486
                    | 0x487
                ) {
                    Some(format!("{:#x}", n_msg))
                } else {
                    None // TODO if_then_some
                }
            },
            |hwnd, n_msg, wparam, lparam| {
                let res = call_original(hwnd, n_msg, wparam, lparam);
                if 
                    n_msg==NETSCHK_MSG_SPIEL_BEKOMMEN
                    || n_msg==NETSCHK_MSG_SPIEL_BEKOMMEN_NACH_PRIO
                    || n_msg==NETSCHK_MSG_MAG_AUCH
                {
                    let hwnd_spielabfrage = *(0x004bd4dc as *mut HWND);
                    assert_eq!(hwnd, hwnd_spielabfrage);
                    let mut vecch_orig : Vec<CHAR> = vec![0; 1000];
                    netschk_maybe_vorschlag::redirected(vecch_orig.as_mut_ptr(), vecch_orig.len());
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
                        let hwnd_list = GetDlgItem(hwnd_spielabfrage, n_id_list as _); // TODO verify
                        let n_sendmessage = SendMessageA(
                            hwnd_list,
                            LB_SELECTSTRING,
                            std::mem::transmute(-1), // search entire list
                            std::mem::transmute(str_item.as_ptr()),
                        );
                        assert!(n_sendmessage!=LB_ERR);
                        let n_sendmessage = SendMessageA(
                            hwnd_spielabfrage,
                            WM_COMMAND,
                            std::mem::transmute(make_dword(
                                n_id_list,
                                LBN_SELCHANGE,
                            )),
                            std::mem::transmute(hwnd_list),
                        );
                        assert!(n_sendmessage!=LB_ERR);
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
            |_hwnd, n_msg, _wparam, _lparam| {
                if n_msg==WM_COMMAND {
                    Some("WM_COMMAND".to_owned())
                } else if n_msg==WM_INITDIALOG {
                    Some("WM_INITDIALOG".to_owned())
                } else if n_msg==WM_SHOWWINDOW {
                    Some("WM_SHOWWINDOW".to_owned())
                } else {
                    None
                }
            },
            |hwnd, n_msg, wparam, lparam| {
                let retval = call_original(hwnd, n_msg, wparam, lparam);
                if n_msg==WM_SHOWWINDOW {
                    click_button(
                        hwnd,
                        /*n_id_dlg_item*/if TRUE==netschk_maybe_vorschlag_should_stoss::redirected(N_INDEX_GAST) {
                            /*Ja*/1082
                        } else {
                            /*Nein*/1081
                        },
                        /*b_allow_invisible*/true, // WM_SHOWWINDOW called very early
                        ESendOrPost::Send,
                    ).unwrap();
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
        log_in_out_cond(
            "dialogproc_analyse_weiter_1",
            (hwnd, n_msg, wparam, lparam),
            |_hwnd, _n_msg, _wparam, _lparam| {
                Some(())
            },
            |hwnd, n_msg, wparam, lparam| {
                call_original(hwnd, n_msg, wparam, lparam)
            },
        )
    },
);
make_redirect_function!(
    netschk_dialogproc_analyse_weiter_2_maybe_ja_nein_1,
    /*pfn_original*/0x00412050,
    ("system") (hwnd: HWND, n_msg: UINT, wparam: WPARAM, lparam: LPARAM,) -> INT_PTR,
    {
        log_in_out_cond(
            "dialogproc_analyse_weiter_2_maybe_ja_nein",
            (hwnd, n_msg, wparam, lparam),
            |_hwnd, _n_msg, _wparam, _lparam| {
                Some(())
            },
            |hwnd, n_msg, wparam, lparam| {
                let retval = call_original(hwnd, n_msg, wparam, lparam);
                if n_msg==WM_SHOWWINDOW {
                    click_button(
                        hwnd,
                        /*weiter*/1083,
                        /*b_allow_invisible*/true, // WM_SHOWWINDOW called very early
                        ESendOrPost::Send,
                    ).unwrap();
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
        log_in_out_cond(
            "wndproc_status_bar",
            (hwnd, n_msg, wparam, lparam),
            |_hwnd, _n_msg, _wparam, _lparam| {
                Some(())
            },
            |hwnd, n_msg, wparam, lparam| {
                call_original(hwnd, n_msg, wparam, lparam)
            },
        )
    },
);


#[repr(C)]
struct SNetSchafkopfCard {
    byte_farbe: u8,
    byte_schlag: u8,
    byte_maybe_back_of_card_but_mostly_zero: u8,
}
impl std::fmt::Display for SNetSchafkopfCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}{}", char::from(self.byte_farbe), char::from(self.byte_schlag))
    }
}
const N_BYTES_PER_NETSCHAFKOPF_CARD : usize = std::mem::size_of::<SNetSchafkopfCard>();

unsafe fn interpret_as_cards<'lft>(pbyte: *const u8, n_cards_max: usize) -> &'lft [SNetSchafkopfCard] {
    let slcbyte = std::slice::from_raw_parts(pbyte, n_cards_max * N_BYTES_PER_NETSCHAFKOPF_CARD);
    let mut n_cards = 0;
    while n_cards < n_cards_max && {
        let i_byte = n_cards * N_BYTES_PER_NETSCHAFKOPF_CARD;
        bytes_are_card(&slcbyte[i_byte..i_byte+N_BYTES_PER_NETSCHAFKOPF_CARD])
    } {
        n_cards += 1;
    }
    std::slice::from_raw_parts(std::mem::transmute(pbyte), n_cards)
}

static mut G_B_LOG_GAME : bool = true;

fn log_game() {
    if !unsafe{G_B_LOG_GAME} {
        return;
    }
    info!("log_game <-");
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
    for (str_player, n_ptr_hand) in [
        ("links", 0x4b5e67),
        ("oben", 0x4b5e8b),
        ("rechts", 0x4b5eaf),
        ("gast", 0x4b5ed3),
    ] {
        info!("Hand von {}: {}",
            str_player,
            unsafe {interpret_as_cards(std::mem::transmute(n_ptr_hand), /*n_cards_max*/8)}
                .iter()
                .join(" "),
        );
    }
    for (str_player, n_ptr_played) in [
        ("links", 0x4c60de),
        ("oben", 0x4c60f9),
        ("rechts", 0x4c6114),
        ("gast", 0x4c612f),
    ] {
        info!("Gespielte Karten von {}: {}",
            str_player,
            unsafe {interpret_as_cards(std::mem::transmute(n_ptr_played), /*n_cards_max*/8)}
                .iter()
                .join(" "),
        );
    }
    info!("Geber: {}",
        unsafe{*std::mem::transmute::<_, *const isize>(0x004ca578)}
    );
    info!("# komplette Stiche: {}",
        unsafe{*std::mem::transmute::<_, *const isize>(0x004b5988)}
    );
    info!("# played cards in current stich: {}",
        unsafe{*std::mem::transmute::<_, *const isize>(0x004963e4)}
    );
    info!("g_iEPIPresumablyNextCard: {}",
        unsafe{*std::mem::transmute::<_, *const isize>(0x004b596c)}
    );
    info!("log_game ->");
}

fn initialize() {
    let path_user_data = dirs::data_dir().unwrap();
    fs::create_dir_all(&path_user_data).unwrap();
    simple_logging::log_to_file(
        path_user_data.join("netschafkopf_helper.log"),
        log::LevelFilter::Info,
    ).unwrap();
    info!("initialize <- (after logging setup)");
    info!("pid: {}", std::process::id());
    info!("process_path: {}", std::env::current_exe().unwrap().display());

    unsafe{netschk_strcpy_s::redirect()};
    if false { // Redirecting this function is very, very slow
        unsafe{netschk_increment_playerindex::redirect()};
    }
    unsafe{netschk_process_window_message::redirect()};
    unsafe{netschk_maybe_vorschlag::redirect()};
    unsafe{netschk_maybe_vorschlag_suggest_card_1::redirect()};
    unsafe{netschk_maybe_vorschlag_suggest_card_2::redirect()};
    unsafe{netschk_maybe_vorschlag_suggest_card_3::redirect()};
    unsafe{netschk_maybe_vorschlag_suggest_card_4::redirect()};
    unsafe{netschk_maybe_vorschlag_should_stoss::redirect()};
    unsafe{netschk_maybe_vorschlag_spielabfrage_1::redirect()};
    unsafe{netschk_maybe_vorschlag_spielabfrage_2::redirect()};
    unsafe{netschk_maybe_vorschlag_unknown_1::redirect()};
    unsafe{netschk_maybe_vorschlag_unknown_2::redirect()};
    unsafe{netschk_maybe_vorschlag_unknown_4::redirect()};
    unsafe{netschk_maybe_vorschlag_unknown_5::redirect()};
    unsafe{netschk_dialogproc_spielabfrage::redirect()};
    unsafe{netschk_dialogproc_contra_geben::redirect()};
    unsafe{netschk_dialogproc_analyse_weiter_1::redirect()};
    unsafe{netschk_dialogproc_analyse_weiter_2_maybe_ja_nein_1::redirect()};
    unsafe{netschk_wndproc_status_bar::redirect()};
    unsafe{set_window_text_a::redirect()};
    unsafe{post_message_a::redirect()};

    let fn_panic_handler_original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panicinfo| {
        error!("panic: {}", panicinfo);
        log_game();
        fn_panic_handler_original(panicinfo)
    }));

    info!("initialize ->");
}
