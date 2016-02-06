use stich::*;
use ncurses;

pub fn println(s: &str) {
    ncurses::printw(s);
    ncurses::printw("\n");
    ncurses::refresh();
}

pub fn print(s: &str) {
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

pub fn print_vecstich(vecstich: &Vec<CStich>) {
    let ncwin = ncurses::newwin(
        5, // height
        90, // width
        0, // y
        0, // x
    );
    let tui_card_string = |stich: &CStich, eplayerindex| {
        let str_card = format!("{}{}",
            if eplayerindex==stich.first_player_index() { ">" } else { " " },
            match stich.get(eplayerindex) {
                None => {"..".to_string()},
                Some(card) => {format!("{}", card)},
            }
        );
        assert_eq!(str_card.len(), 3);
        str_card
    };
    for stich in vecstich {
        wprint(ncwin, &format!("   {}    ", tui_card_string(stich, /*eplayerindex*/2)));
    }
    wprintln(ncwin, "");
    for stich in vecstich {
        wprint(ncwin, &format!(" {} {}  ",
           tui_card_string(stich, /*eplayerindex*/1),
           tui_card_string(stich, /*eplayerindex*/3)
        ));
    }
    wprintln(ncwin, "");
    for stich in vecstich {
        wprint(ncwin, &format!("   {}    ", tui_card_string(stich, /*eplayerindex*/0)));
    }
    wprintln(ncwin, "");
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

