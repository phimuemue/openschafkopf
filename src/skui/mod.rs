use primitives::*;
use game::*;
use ncurses;
use rules::*;
use util::*;
use itertools::Itertools;

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

fn print_card_with_farbe(ncwin: ncurses::WINDOW, card: SCard) {
    let paircolorcolor = { match card.farbe() {
        EFarbe::Eichel => (ncurses::COLOR_YELLOW, ncurses::COLOR_BLACK),
        EFarbe::Gras => (ncurses::COLOR_GREEN, ncurses::COLOR_BLACK),
        EFarbe::Herz => (ncurses::COLOR_RED, ncurses::COLOR_BLACK),
        EFarbe::Schelln => (ncurses::COLOR_CYAN, ncurses::COLOR_BLACK),
    }};
    let i_color_pair = paircolorcolor.0 * 8 + paircolorcolor.1;
    ncurses::init_pair(i_color_pair, paircolorcolor.0, paircolorcolor.1);
    let nccolorpair = ncurses::COLOR_PAIR(i_color_pair);
    ncurses::wattron(ncwin, nccolorpair);
    wprint(ncwin, &format!("{}", card));
    ncurses::wattroff(ncwin, nccolorpair);
}

enum VSkUiWindow {
    Stich,
    Interaction,
    Hand,
    PlayerInfo (EPlayerIndex),
    GameInfo,
    AccountBalance,
}

fn do_in_window<FnDo, RetVal>(skuiwin: &VSkUiWindow, fn_do: FnDo) -> RetVal
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
    let ncwin = match *skuiwin {
        VSkUiWindow::PlayerInfo(epi) => {
            match epi {
                EPlayerIndex::EPI0 => {
                    create_fullwidth_window(n_height-2, n_height-1)
                },
                EPlayerIndex::EPI1 | EPlayerIndex::EPI2 | EPlayerIndex::EPI3 => {
                    ncurses::newwin(
                        1, // height
                        24, // width
                        0, // y
                        ((epi.to_usize() - 1)*25).as_num() // x
                    )
                }
            }
        },
        VSkUiWindow::Stich => {create_fullwidth_window(1, 6)},
        VSkUiWindow::Hand => {create_fullwidth_window(6, 17)},
        VSkUiWindow::Interaction => {create_fullwidth_window(17, n_height-3)},
        VSkUiWindow::GameInfo => {create_fullwidth_window(n_height-3, n_height-2)}
        VSkUiWindow::AccountBalance => {create_fullwidth_window(n_height-2, n_height-1)}
    };
    let retval = fn_do(ncwin);
    ncurses::delwin(ncwin);
    retval
}

pub fn print_vecstich(vecstich: &[SStich]) {
    do_in_window(&VSkUiWindow::Stich, |ncwin| {
        for (i_stich, stich) in vecstich.iter().enumerate() {
            let n_x = (i_stich*10+3).as_num();
            let n_y = 1;
            let print_card = |epi, (n_y, n_x)| {
                ncurses::wmove(ncwin, n_y, n_x);
                wprint(ncwin, if epi==stich.first_playerindex() { ">" } else { " " });
                match stich.get(epi) {
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
    });
}

pub fn print_game_announcements(gameannouncements: &SGameAnnouncements) {
    for (epi, orules) in gameannouncements.iter() {
        do_in_window(&VSkUiWindow::PlayerInfo(epi), |ncwin| {
            if let Some(ref rules) = *orules {
                wprint(ncwin, &format!("{}: {}", epi, rules));
            } else {
                wprint(ncwin, &format!("{}: Nothing", epi));
            }
            ncurses::wrefresh(ncwin);
        });
    }
}

pub fn print_game_info(rules: &TRules, doublings: &SDoublings, vecstoss: &[SStoss]) {
    do_in_window(&VSkUiWindow::GameInfo, |ncwin| {
        wprint(ncwin, &format!("{}", rules));
        if let Some(epi) = rules.playerindex() {
            wprint(ncwin, &format!(", played by {}", epi));
        }
        let print_special = |str_special, vecepi: Vec<EPlayerIndex>| {
            if !vecepi.is_empty() {
                wprint(ncwin, str_special);
                for epi in vecepi {
                    wprint(ncwin, &format!("{},", epi));
                }
            }
        };
        print_special(
            ". Doublings: ",
            doublings.iter()
                .filter(|&(_epi, b_doubling)| *b_doubling)
                .map(|(epi, _b_doubling)| epi)
                .collect()
        );
        print_special(
            ". Stoesse: ",
            vecstoss.iter()
                .map(|stoss| stoss.epi)
                .collect()
        );
        ncurses::wrefresh(ncwin);
    })
}

pub fn account_balance_string(accountbalance: &SAccountBalance) -> String {
    EPlayerIndex::values()
        .map(|epi| format!("{}: {} | ", epi, accountbalance.get_player(epi)))
        .join("")
        + &format!("Stock: {}", accountbalance.get_stock())
}

pub fn print_account_balance(accountbalance : &SAccountBalance) {
    do_in_window(&VSkUiWindow::AccountBalance, |ncwin| {
        wprint(ncwin, &account_balance_string(accountbalance));
    })
}

pub struct SAskForAlternativeKeyBindings {
    key_prev : i32,
    key_next : i32,
    key_choose : i32,
    key_suggest : i32,
}

pub fn choose_card_from_hand_key_bindings() -> SAskForAlternativeKeyBindings {
    SAskForAlternativeKeyBindings {
        key_prev : ncurses::KEY_LEFT,
        key_next : ncurses::KEY_RIGHT,
        key_choose : ncurses::KEY_UP,
        key_suggest : '?' as i32,
    }
}

pub fn choose_alternative_from_list_key_bindings() -> SAskForAlternativeKeyBindings {
    SAskForAlternativeKeyBindings {
        key_prev : ncurses::KEY_UP,
        key_next : ncurses::KEY_DOWN,
        key_choose : ncurses::KEY_RIGHT,
        key_suggest : '?' as i32,
    }
}

pub fn ask_for_alternative<'vect, T, FnFilter, FnCallback, FnSuggest>(
    vect: &'vect [T],
    askforalternativekeybindings: &SAskForAlternativeKeyBindings,
    fn_filter: FnFilter,
    fn_callback: FnCallback,
    fn_suggest: FnSuggest
) -> &'vect T
    where FnFilter : Fn(&T) -> bool,
          FnCallback : Fn(ncurses::WINDOW, usize, &Option<T>),
          FnSuggest : Fn() -> Option<T>
{
    do_in_window(&VSkUiWindow::Interaction, |ncwin| {
        let mut ot_suggest = None;
        let vect = vect.into_iter().enumerate().filter(|&(_i_t, t)| fn_filter(t)).collect::<Vec<_>>();
        assert!(0<vect.len());
        let mut i_alternative = 0; // initially, point to 0th alternative
        fn_callback(ncwin, vect[i_alternative].0, &ot_suggest);
        if 1<vect.len() {
            let mut ch = askforalternativekeybindings.key_prev;
            while ch!=askforalternativekeybindings.key_choose {
                ncurses::werase(ncwin);
                if ch==askforalternativekeybindings.key_prev {
                    if 0<i_alternative {
                        i_alternative -= 1
                    }
                } else if ch== askforalternativekeybindings.key_next {
                    if i_alternative<vect.len()-1 {
                        i_alternative += 1
                    }
                } else if ch==askforalternativekeybindings.key_suggest {
                    ot_suggest = fn_suggest();
                }
                fn_callback(ncwin, vect[i_alternative].0, &ot_suggest);
                ch = ncurses::getch();
            }
            ncurses::erase();
        }
        vect[i_alternative].1
    })
}

pub fn print_hand(veccard: &[SCard], oi_card: Option<usize>) {
    do_in_window(&VSkUiWindow::Hand, |ncwin| {
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
    });
}
