use primitives::*;
use game::*;
use ncurses;
use rules::*;
use util::*;

pub fn init_ui() {
    ncurses::initscr();
    ncurses::keypad(ncurses::stdscr(), true);
    ncurses::noecho();
    ncurses::start_color();
}

pub fn end_ui() {
    ncurses::endwin();
}

pub fn wprintln(ncwin: ncurses::WINDOW, s: &str) {
    ncurses::waddstr(ncwin, s);
    ncurses::waddstr(ncwin, "\n");
    ncurses::wrefresh(ncwin);
}

fn wprint(ncwin: ncurses::WINDOW, s: &str) {
    ncurses::waddstr(ncwin, s);
    ncurses::wrefresh(ncwin);
}

pub fn logln(_s: &str) {
    ncurses::refresh();
}

fn print_string_with_nc_colors(ncwin: ncurses::WINDOW, color_fg: i16, color_bg: i16, str_output: &str) {
    let i_color_pair = color_fg * 8 + color_bg;
    ncurses::init_pair(i_color_pair, color_fg, color_bg);
    let nccolorpair = ncurses::COLOR_PAIR(i_color_pair);
    ncurses::wattron(ncwin, nccolorpair);
    wprint(ncwin, str_output);
    ncurses::wattroff(ncwin, nccolorpair);
}

fn print_card_with_farbe(ncwin: ncurses::WINDOW, card: SCard) {
    let paircolorcolor = { match card.farbe() {
        EFarbe::Eichel => (ncurses::COLOR_YELLOW, ncurses::COLOR_BLACK),
        EFarbe::Gras => (ncurses::COLOR_GREEN, ncurses::COLOR_BLACK),
        EFarbe::Herz => (ncurses::COLOR_RED, ncurses::COLOR_BLACK),
        EFarbe::Schelln => (ncurses::COLOR_CYAN, ncurses::COLOR_BLACK),
    }};
    print_string_with_nc_colors(ncwin, paircolorcolor.0, paircolorcolor.1, &format!("{}", card));
}

enum ESkUiWindow {
    Stich,
    Interaction,
    Hand,
    PlayerInfo (EPlayerIndex),
    GameInfo,
    AccountBalance,
}

fn do_in_window<FnDo, RetVal>(skuiwin: ESkUiWindow, fn_do: FnDo) -> RetVal
    where FnDo: FnOnce(ncurses::WINDOW) -> RetVal
{
    let (n_height, n_width) = {
        let mut n_height = 0;
        let mut n_width = 0;
        ncurses::getmaxyx(ncurses::stdscr(), &mut n_height, &mut n_width);
        (n_height, n_width)
    };
    let create_fullwidth_window = |n_top, n_bottom| {
        ncurses::newwin(
            n_bottom-n_top, // height
            n_width, // width
            n_top, // y
            0 // x
        )
    };
    let ncwin = match skuiwin {
        ESkUiWindow::PlayerInfo(eplayerindex) => {
            match eplayerindex {
                EPlayerIndex::EPI0 => {
                    create_fullwidth_window(n_height-2, n_height-1)
                },
                EPlayerIndex::EPI1 | EPlayerIndex::EPI2 | EPlayerIndex::EPI3 => {
                    ncurses::newwin(
                        1, // height
                        24, // width
                        0, // y
                        ((eplayerindex.to_usize() - 1)*25).as_num() // x
                    )
                }
            }
        },
        ESkUiWindow::Stich => {create_fullwidth_window(1, 6)},
        ESkUiWindow::Hand => {create_fullwidth_window(6, 17)},
        ESkUiWindow::Interaction => {create_fullwidth_window(17, n_height-3)},
        ESkUiWindow::GameInfo => {create_fullwidth_window(n_height-3, n_height-2)}
        ESkUiWindow::AccountBalance => {create_fullwidth_window(n_height-2, n_height-1)}
    };
    let retval = fn_do(ncwin);
    ncurses::delwin(ncwin);
    retval
}

pub fn print_vecstich(vecstich: &[SStich]) {
    do_in_window(
        ESkUiWindow::Stich,
        |ncwin| {
            for (i_stich, stich) in vecstich.iter().enumerate() {
                let n_x = (i_stich*10+3).as_num();
                let n_y = 1;
                let print_card = |eplayerindex, (n_y, n_x)| {
                    ncurses::wmove(ncwin, n_y, n_x);
                    wprint(ncwin, if eplayerindex==stich.first_playerindex() { ">" } else { " " });
                    match stich.get(eplayerindex) {
                        None => {wprint(ncwin, "..")},
                        Some(card) => {print_card_with_farbe(ncwin, *card)},
                    };
                };
                let n_card_width = 2;
                print_card(EPlayerIndex::EPI0, (n_y+1, n_x));
                print_card(EPlayerIndex::EPI1, (n_y, n_x-n_card_width));
                print_card(EPlayerIndex::EPI2, (n_y-1, n_x));
                print_card(EPlayerIndex::EPI3, (n_y, n_x+n_card_width));
            }
        }
    );
}

pub fn print_game_announcements(gameannouncements: &SGameAnnouncements) {
    for (eplayerindex, orules) in gameannouncements.iter() {
        do_in_window(
            ESkUiWindow::PlayerInfo(eplayerindex),
            |ncwin| {
                if let Some(rules) = *orules {
                    wprint(ncwin, &format!("{}: {}", eplayerindex, rules.to_string()));
                } else {
                    wprint(ncwin, &format!("{}: Nothing", eplayerindex));
                }
                ncurses::wrefresh(ncwin);
            }
        );
    }
}

pub fn print_game_info(rules: &TRules, doublings: &SDoublings, vecstoss: &[SStoss]) {
    do_in_window(
        ESkUiWindow::GameInfo,
        |ncwin| {
            wprint(ncwin, &format!("{}", rules));
            if let Some(eplayerindex) = rules.playerindex() {
                wprint(ncwin, &format!(", played by {}", eplayerindex));
            }
            let print_special = |str_special, veceplayerindex: Vec<EPlayerIndex>| {
                if !veceplayerindex.is_empty() {
                    wprint(ncwin, str_special);
                    for eplayerindex in veceplayerindex {
                        wprint(ncwin, &format!("{},", eplayerindex));
                    }
                }
            };
            print_special(
                ". Doublings: ",
                doublings.iter()
                    .filter(|&(_eplayerindex, b_doubling)| *b_doubling)
                    .map(|(eplayerindex, _b_doubling)| eplayerindex)
                    .collect()
            );
            print_special(
                ". Stoesse: ",
                vecstoss.iter()
                    .map(|stoss| stoss.m_eplayerindex)
                    .collect()
            );
            ncurses::wrefresh(ncwin);
        }
    )
}

pub fn account_balance_string(accountbalance: &SAccountBalance) -> String {
    let mut str = "".to_string();
    for eplayerindex in EPlayerIndex::values() {
        str = str + &format!("{}: {} | ", eplayerindex, accountbalance.get_player(eplayerindex));
    }
    str = str + &format!("Stock: {}", accountbalance.get_stock());
    str
}

pub fn print_account_balance(accountbalance : &SAccountBalance) {
    do_in_window(
        ESkUiWindow::AccountBalance,
        |ncwin| {
            wprint(ncwin, &account_balance_string(accountbalance));
        }
    )
}

pub struct SAskForAlternativeKeyBindings {
    m_key_prev : i32,
    m_key_next : i32,
    m_key_choose : i32,
    m_key_suggest : i32,
}

pub fn choose_card_from_hand_key_bindings() -> SAskForAlternativeKeyBindings {
    SAskForAlternativeKeyBindings {
        m_key_prev : ncurses::KEY_LEFT,
        m_key_next : ncurses::KEY_RIGHT,
        m_key_choose : ncurses::KEY_UP,
        m_key_suggest : '?' as i32,
    }
}

pub fn choose_alternative_from_list_key_bindings() -> SAskForAlternativeKeyBindings {
    SAskForAlternativeKeyBindings {
        m_key_prev : ncurses::KEY_UP,
        m_key_next : ncurses::KEY_DOWN,
        m_key_choose : ncurses::KEY_RIGHT,
        m_key_suggest : '?' as i32,
    }
}

pub fn ask_for_alternative<T, FnFilter, FnCallback, FnSuggest>(
    vect: &[T],
    askforalternativekeybindings: SAskForAlternativeKeyBindings,
    fn_filter: FnFilter,
    fn_callback: FnCallback,
    fn_suggest: FnSuggest
) -> &T 
    where FnFilter : Fn(&T) -> bool,
          FnCallback : Fn(ncurses::WINDOW, usize, &Option<T>),
          FnSuggest : Fn() -> Option<T>
{
    do_in_window(
        ESkUiWindow::Interaction,
        |ncwin| {
            let mut ot_suggest = None;
            let vect = vect.into_iter().enumerate().filter(|&(_i_t, t)| fn_filter(t)).collect::<Vec<_>>();
            assert!(0<vect.len());
            let mut i_alternative = 0; // initially, point to 0th alternative
            fn_callback(ncwin, vect[i_alternative].0, &ot_suggest);
            if 1<vect.len() {
                let mut ch = askforalternativekeybindings.m_key_prev;
                while ch!=askforalternativekeybindings.m_key_choose {
                    ncurses::werase(ncwin);
                    if ch==askforalternativekeybindings.m_key_prev {
                        if 0<i_alternative {
                            i_alternative -= 1
                        }
                    } else if ch== askforalternativekeybindings.m_key_next {
                        if i_alternative<vect.len()-1 {
                            i_alternative += 1
                        }
                    } else if ch==askforalternativekeybindings.m_key_suggest {
                        ot_suggest = fn_suggest();
                    }
                    fn_callback(ncwin, vect[i_alternative].0, &ot_suggest);
                    ch = ncurses::getch();
                }
                ncurses::erase();
            }
            vect[i_alternative].1
        }
    )
}

pub fn print_hand(veccard: &[SCard], oi_card: Option<usize>) {
    do_in_window(
        ESkUiWindow::Hand,
        |ncwin| {
            let is_oi_card = |i| { oi_card.map_or(false, |i_card| i==i_card) };
            for (i, card) in veccard.iter().enumerate() {
                let n_card_width = 10;
                ncurses::wmove(ncwin,
                    /*n_y*/ if is_oi_card(i) { 0i32 } else { 1i32 },
                    /*n_x*/ (n_card_width * i).as_num()
                );
                wprint(ncwin, " +--");
                print_card_with_farbe(ncwin, *card);
                wprint(ncwin, "--+ ");
            }
            ncurses::refresh();
        }
    );
}
