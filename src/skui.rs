use card::*;
use stich::*;
use hand::*;
use ncurses;

enum ESkUiWindow {
    Stich,
    Interaction,
    Hand,
}

pub fn init_ui() {
    ncurses::initscr();
    ncurses::keypad(ncurses::stdscr, true);
    ncurses::noecho();
    ncurses::start_color();
}

pub fn end_ui() {
    ncurses::endwin();
}

fn println(s: &str) {
    ncurses::printw(s);
    ncurses::printw("\n");
    ncurses::refresh();
}

fn print(s: &str) {
    ncurses::printw(s);
    ncurses::refresh();
}

fn wprintln(ncwin: ncurses::WINDOW, s: &str) {
    ncurses::wprintw(ncwin, s);
    ncurses::wprintw(ncwin, "\n");
    ncurses::wrefresh(ncwin);
}

fn wprint(ncwin: ncurses::WINDOW, s: &str) {
    ncurses::wprintw(ncwin, s);
    ncurses::wrefresh(ncwin);
}

pub fn logln(s: &str) {
    ncurses::refresh();
}

pub fn log(s: &str) {
    ncurses::refresh();
}

fn print_card_with_farbe(ncwin: ncurses::WINDOW, card: CCard) {
    // TODO lib: enummap!
    ncurses::init_pair(1, ncurses::COLOR_YELLOW, ncurses::COLOR_BLACK);
    ncurses::init_pair(2, ncurses::COLOR_GREEN, ncurses::COLOR_BLACK);
    ncurses::init_pair(3, ncurses::COLOR_RED, ncurses::COLOR_BLACK);
    ncurses::init_pair(4, ncurses::COLOR_CYAN, ncurses::COLOR_BLACK);
    let nccolorpair = ncurses::COLOR_PAIR((card.farbe() as i16)+1); // TODO lib: enummap
    ncurses::wattron(ncwin, nccolorpair as i32);
    wprint(ncwin, &format!("{}", card));
    ncurses::wattroff(ncwin, nccolorpair as i32);
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
        ESkUiWindow::Stich => {create_fullwidth_window(0, 5)},
        ESkUiWindow::Hand => {create_fullwidth_window(5, 7)}
        ESkUiWindow::Interaction => {create_fullwidth_window(7, n_height-1)}
    };
    let retval = fn_do(ncwin);
    ncurses::delwin(ncwin);
    retval
}

pub fn print_vecstich(vecstich: &Vec<CStich>) {
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

pub struct SAskForAlternativeKeyBindings {
    m_key_prev : i32,
    m_key_next : i32,
    m_key_choose : i32,
}

pub fn choose_card_from_hand_key_bindings() -> SAskForAlternativeKeyBindings {
    SAskForAlternativeKeyBindings {
        m_key_prev : ncurses::KEY_LEFT,
        m_key_next : ncurses::KEY_RIGHT,
        m_key_choose : ncurses::KEY_UP,
    }
}

pub fn choose_alternative_from_list_key_bindings() -> SAskForAlternativeKeyBindings {
    SAskForAlternativeKeyBindings {
        m_key_prev : ncurses::KEY_UP,
        m_key_next : ncurses::KEY_DOWN,
        m_key_choose : ncurses::KEY_RIGHT,
    }
}

pub fn ask_for_alternative<T, FnFormat, FnFilter, FnCallback>(
    str_question: &str,
    vect: Vec<T>,
    askforalternativekeybindings: SAskForAlternativeKeyBindings,
    fn_filter: FnFilter,
    fn_format: FnFormat,
    fn_callback: FnCallback
) -> T 
    where FnFormat : Fn(&T) -> String,
          FnFilter : Fn(&T) -> bool,
          FnCallback : Fn(&T, usize)
{
    do_in_window(
        ESkUiWindow::Interaction,
        |ncwin| {
            let vect = vect.into_iter().enumerate().filter(|&(_i_t, ref t)| fn_filter(&t)).collect::<Vec<_>>();
            assert!(0<vect.len());
            let mut i_alternative = 0; // initially, point to 0th alternative
            if 1==vect.len() {
                return vect.into_iter().nth(0).unwrap().1; // just return if there's no choice anyway
            }
            {
                let print_alternatives = |i_alternative| {
                    wprintln(ncwin, str_question);
                    for (i_t, t) in vect.iter().enumerate() {
                        wprintln(ncwin, &format!("{} {} ({})",
                            if i_t==i_alternative {"*"} else {" "},
                            fn_format(&t.1),
                            i_t
                        ));
                    }
                    fn_callback(&vect[i_alternative].1, vect[i_alternative].0);
                };
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
                    }
                    print_alternatives(i_alternative);
                    ch = ncurses::getch();
                }
                ncurses::erase();
            }
            vect.into_iter().nth(i_alternative).unwrap().1
        }
    )
}

pub fn print_hand(veccard: &Vec<CCard>, oi_card: Option<usize>) {
    do_in_window(
        ESkUiWindow::Hand,
        |ncwin| {
            let is_oi_card = |i| { oi_card.map_or(false, |i_card| i==i_card) };
            for (i, card) in veccard.iter().enumerate() {
                if is_oi_card(i) {
                    ncurses::wattron(ncwin, ncurses::A_REVERSE() as i32);
                }
                wprint(ncwin, " ");
                print_card_with_farbe(ncwin, *card);
                if is_oi_card(i) {
                    ncurses::wattroff(ncwin, ncurses::A_REVERSE() as i32);
                }
            }
            ncurses::refresh();
        }
    );
}
