use primitives::*;
use game::*;
use ncurses;

pub fn init_ui() {
    ncurses::initscr();
    ncurses::keypad(ncurses::stdscr, true);
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

pub fn log(_s: &str) {
    ncurses::refresh();
}

fn print_string_with_nc_colors(ncwin: ncurses::WINDOW, color_fg: i16, color_bg: i16, str_output: &str) {
    let i_color_pair = color_fg * 8 + color_bg;
    ncurses::init_pair(i_color_pair, color_fg, color_bg);
    let nccolorpair = ncurses::COLOR_PAIR(i_color_pair) as i32;
    ncurses::wattron(ncwin, nccolorpair);
    wprint(ncwin, str_output);
    ncurses::wattroff(ncwin, nccolorpair);
}

fn print_card_with_farbe(ncwin: ncurses::WINDOW, card: SCard) {
    let vecpaircolorcolor = vec! [ // TODO lib: enummap!
        (ncurses::COLOR_YELLOW, ncurses::COLOR_BLACK),
        (ncurses::COLOR_GREEN, ncurses::COLOR_BLACK),
        (ncurses::COLOR_RED, ncurses::COLOR_BLACK),
        (ncurses::COLOR_CYAN, ncurses::COLOR_BLACK),
    ];
    let i_paircolor = card.farbe() as usize;
    print_string_with_nc_colors(ncwin, vecpaircolorcolor[i_paircolor].0, vecpaircolorcolor[i_paircolor].1, &format!("{}", card));
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
        ncurses::getmaxyx(ncurses::stdscr, &mut n_height, &mut n_width);
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
            if 0==eplayerindex {
                create_fullwidth_window(n_height-2, n_height-1)
            } else {
                assert!(1==eplayerindex || 2==eplayerindex || 3==eplayerindex);
                ncurses::newwin(
                    1, // height
                    24, // width
                    0, // y
                    (eplayerindex as i32 - 1)*25 // x
                )
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
                let n_x = (i_stich as i32)*10+3;
                let n_y = 1;
                let print_card = |eplayerindex, (n_y, n_x)| {
                    ncurses::wmove(ncwin, n_y, n_x);
                    wprint(ncwin, if eplayerindex==stich.first_player_index() { ">" } else { " " });
                    match stich.get(eplayerindex) {
                        None => {wprint(ncwin, "..")},
                        Some(card) => {print_card_with_farbe(ncwin, card)},
                    };
                };
                let n_card_width = 2;
                print_card(0, (n_y+1, n_x));
                print_card(1, (n_y, n_x-n_card_width));
                print_card(2, (n_y-1, n_x));
                print_card(3, (n_y, n_x+n_card_width));
            }
        }
    );
}

pub fn print_game_announcements(vecgameannouncement: &[SGameAnnouncement]) {
    for gameannouncement in vecgameannouncement {
        do_in_window(
            ESkUiWindow::PlayerInfo(gameannouncement.m_eplayerindex),
            |ncwin| {
                if gameannouncement.m_opairrulespriority.is_none() {
                    wprint(ncwin, &format!("{}: Nothing", gameannouncement.m_eplayerindex));
                } else {
                    wprint(ncwin, &format!("{}: {}", gameannouncement.m_eplayerindex, gameannouncement.m_opairrulespriority.as_ref().unwrap().0.to_string()));
                }
                ncurses::wrefresh(ncwin);
            }
        );
    }
}

pub fn print_game_info(game: &SGame) {
    do_in_window(
        ESkUiWindow::GameInfo,
        |ncwin| {
            wprint(ncwin, &format!("{}", game.m_rules));
            if let Some(eplayerindex) = game.m_rules.playerindex() {
                wprint(ncwin, &format!(", played by {}. Stoesse: ", eplayerindex));
                for stoss in &game.m_vecstoss {
                    wprint(ncwin, &format!("{},", stoss.m_eplayerindex));
                }
            }
            ncurses::wrefresh(ncwin);
        }
    )
}

pub fn account_balance_string(accountbalance: &SAccountBalance) -> String {
    let mut str = "".to_string();
    for eplayerindex in 0..4 {
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

pub fn ask_for_alternative<'vect, T, FnFilter, FnCallback, FnSuggest>(
    vect: &'vect [T],
    askforalternativekeybindings: SAskForAlternativeKeyBindings,
    fn_filter: FnFilter,
    fn_callback: FnCallback,
    fn_suggest: FnSuggest
) -> &'vect T 
    where FnFilter : Fn(&T) -> bool,
          FnCallback : Fn(ncurses::WINDOW, usize, &Option<T>),
          FnSuggest : Fn() -> Option<T>
{
    do_in_window(
        ESkUiWindow::Interaction,
        |ncwin| {
            let mut ot_suggest = None;
            let vect = vect.into_iter().enumerate().filter(|&(_i_t, ref t)| fn_filter(&t)).collect::<Vec<_>>();
            assert!(0<vect.len());
            let mut i_alternative = 0; // initially, point to 0th alternative
            fn_callback(ncwin, vect[i_alternative].0, &ot_suggest);
            if 1<vect.len() {
                let mut ch = askforalternativekeybindings.m_key_prev;
                while ch!=askforalternativekeybindings.m_key_choose {
                    ncurses::werase(ncwin);
                    if ch==askforalternativekeybindings.m_key_prev {
                        if 0<i_alternative {
                            i_alternative = i_alternative - 1
                        }
                    } else if ch== askforalternativekeybindings.m_key_next {
                        if i_alternative<vect.len()-1 {
                            i_alternative = i_alternative + 1
                        }
                    } else if ch==askforalternativekeybindings.m_key_suggest {
                        ot_suggest = fn_suggest();
                    }
                    fn_callback(ncwin, vect[i_alternative].0, &ot_suggest);
                    ch = ncurses::getch();
                }
                ncurses::erase();
            }
            vect.into_iter().nth(i_alternative).unwrap().1
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
                    /*n_y*/ {if is_oi_card(i) { 0 } else { 1 }} as i32,
                    /*n_x*/ n_card_width * i as i32
                );
                wprint(ncwin, " +--");
                print_card_with_farbe(ncwin, *card);
                wprint(ncwin, "--+ ");
            }
            ncurses::refresh();
        }
    );
}
