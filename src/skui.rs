use stich::*;
use ncurses;

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

pub fn print_vecstich(vecstich: &Vec<CStich>) {
    let ncwin = ncurses::newwin(
        5, // height
        90, // width
        0, // y
        0, // x
    );
    // TODO lib: enummap!
    ncurses::init_pair(1, ncurses::COLOR_YELLOW, ncurses::COLOR_BLACK);
    ncurses::init_pair(2, ncurses::COLOR_GREEN, ncurses::COLOR_BLACK);
    ncurses::init_pair(3, ncurses::COLOR_RED, ncurses::COLOR_BLACK);
    ncurses::init_pair(4, ncurses::COLOR_CYAN, ncurses::COLOR_BLACK);
    let print_card_string = |vecnneplayerindex_space_plidx| {
        for stich in vecstich {
            for &(n_space_before, n_space_after, eplayerindex) in &vecnneplayerindex_space_plidx {
                let onccolorpair = stich.get(eplayerindex).map(|card| {
                    ncurses::COLOR_PAIR((card.farbe() as i16)+1) // TODO lib: enummap
                });
                if let Some(nccolorpair) = onccolorpair {
                    ncurses::wattron(ncwin, nccolorpair as i32);
                }
                for _n_space in 0..n_space_before {
                    wprint(ncwin, " ");
                }
                wprint(ncwin, if eplayerindex==stich.first_player_index() { ">" } else { " " });
                match stich.get(eplayerindex) {
                    None => {wprint(ncwin, "..")},
                    Some(card) => {wprint(ncwin, &format!("{}", card))},
                };
                for _n_space in 0..n_space_after {
                    wprint(ncwin, " ");
                }
                if let Some(nccolorpair) = onccolorpair {
                    ncurses::wattroff(ncwin, nccolorpair as i32);
                }
            }
        }
        wprintln(ncwin, "");
    };
    print_card_string(vec!((3, 4, /*eplayerindex*/2)));
    print_card_string(vec!((1, 1, /*eplayerindex*/1), (0, 2, /*eplayerindex*/3)));
    print_card_string(vec!((3, 4, /*eplayerindex*/0)));
    ncurses::delwin(ncwin);
}

pub fn ask_for_alternative<T, FnFormat>(vect: &Vec<T>, fn_format: FnFormat) -> T 
    where T : Clone,
          FnFormat : Fn(&T) -> String
{
    let ncwin = ncurses::newwin(
        (vect.len() as i32)+1, // height
        80, // width
        10, // y, leave space for stich
        0, // x
    );
    assert!(0<vect.len());
    let mut i_alternative = 0; // initially, point to 0th alternative
    if 1==vect.len() {
        return vect[0].clone(); // just return if there's no choice anyway
    }
    let print_alternatives = |i_alternative| {
        wprintln(ncwin, "Please choose:");
        for (i_t, t) in vect.iter().enumerate() {
            wprintln(ncwin, &format!("{} {} ({})",
                if i_t==i_alternative {"*"} else {" "},
                fn_format(&t),
                i_t
            ));
        }
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
    ncurses::delwin(ncwin);
    vect[i_alternative].clone()
}

