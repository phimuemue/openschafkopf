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
    where FnDo: Fn(ncurses::WINDOW) -> RetVal
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
        ESkUiWindow::Interaction => {create_fullwidth_window(5, n_height-3)}
        ESkUiWindow::Hand => {create_fullwidth_window(n_height-3, n_height-1)}
    };
    let retval = fn_do(ncwin);
    ncurses::delwin(ncwin);
    retval
}

pub fn print_vecstich(vecstich: &Vec<CStich>) {
    do_in_window(
        ESkUiWindow::Stich,
        |ncwin| {
            let print_card_string = |vecnneplayerindex_space_plidx| {
                for stich in vecstich {
                    for &(n_space_before, n_space_after, eplayerindex) in &vecnneplayerindex_space_plidx {
                        for _n_space in 0..n_space_before {
                            wprint(ncwin, " ");
                        }
                        wprint(ncwin, if eplayerindex==stich.first_player_index() { ">" } else { " " });
                        match stich.get(eplayerindex) {
                            None => {wprint(ncwin, "..")},
                            Some(card) => {print_card_with_farbe(ncwin, card)},
                        };
                        for _n_space in 0..n_space_after {
                            wprint(ncwin, " ");
                        }
                    }
                }
                wprintln(ncwin, "");
            };
            print_card_string(vec!((3, 4, /*eplayerindex*/2)));
            print_card_string(vec!((1, 1, /*eplayerindex*/1), (0, 2, /*eplayerindex*/3)));
            print_card_string(vec!((3, 4, /*eplayerindex*/0)));
        }
    );
}

pub fn ask_for_alternative<T, FnFormat, FnFilter, FnCallback>(str_question: &str, vect: &Vec<T>, fn_filter: FnFilter, fn_format: FnFormat, fn_callback: FnCallback) -> T 
    where T : Clone,
          FnFormat : Fn(&T) -> String,
          FnFilter : Fn(&T) -> bool,
          FnCallback : Fn(&T, usize)
{
    do_in_window(
        ESkUiWindow::Interaction,
        |ncwin| {
            let vect = vect.iter().enumerate().filter(|&(_i_t, t)| fn_filter(t)).collect::<Vec<_>>();
            assert!(0<vect.len());
            let mut i_alternative = 0; // initially, point to 0th alternative
            if 1==vect.len() {
                return vect[0].1.clone(); // just return if there's no choice anyway
            }
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
            let mut ch = ncurses::KEY_UP;
            while ch!=ncurses::KEY_RIGHT {
                ncurses::werase(ncwin);
                match ch {
                    ncurses::KEY_UP => {
                        if 0<i_alternative {
                            i_alternative = i_alternative - 1
                        }
                    },
                    ncurses::KEY_DOWN => {
                        if i_alternative<vect.len()-1 {
                            i_alternative = i_alternative + 1
                        }
                    },
                    _ => {},
                }
                print_alternatives(i_alternative);
                ch = ncurses::getch();
            }
            ncurses::erase();
            vect[i_alternative].1.clone()
        }
    )
}

pub fn print_hand(hand: &CHand, oi_card: Option<usize>) {
    do_in_window(
        ESkUiWindow::Hand,
        |ncwin| {
            if let Some(i_card)=oi_card {
                for i in 0..hand.cards().len() {
                    if i_card==i {
                        wprint(ncwin, " vv");
                    } else {
                        wprint(ncwin, " ..");
                    }
                }
            }
            wprintln(ncwin, "");
            for card in hand.cards() {
                wprint(ncwin, " ");
                print_card_with_farbe(ncwin, *card);
            }
            ncurses::refresh();
        }
    );
}
