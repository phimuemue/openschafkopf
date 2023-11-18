use openschafkopf_lib::{
    game::*,
    primitives::*,
    rules::*,
};
use openschafkopf_util::*;
use itertools::Itertools;
use plain_enum::{plain_enum_mod, EnumMap, PlainEnum};
use as_num::*;

// TODO do we update output too often?

pub struct STuiGuard {
    phantom: std::marker::PhantomData<()>,
}

impl STuiGuard {
    pub fn init_ui() -> Self {
        ncurses::initscr();
        ncurses::keypad(ncurses::stdscr(), true);
        ncurses::noecho();
        ncurses::start_color();
        STuiGuard{phantom: std::marker::PhantomData}
    }
}

impl Drop for STuiGuard {
    fn drop(&mut self) {
        ncurses::endwin();
    }
}

// TODO ideally, all tui methods would be members of STuiGuard

pub fn wprintln(ncwin: ncurses::WINDOW, s: &str) {
    ncurses::waddstr(ncwin, s);
    ncurses::waddstr(ncwin, "\n");
    ncurses::wrefresh(ncwin);
}

fn wprint(ncwin: ncurses::WINDOW, s: &str) {
    ncurses::waddstr(ncwin, s);
    ncurses::wrefresh(ncwin);
}

fn print_card_with_farbe(ncwin: ncurses::WINDOW, card: ECard) {
    let tplcolorcolor = { match card.farbe() {
        EFarbe::Eichel => (ncurses::COLOR_YELLOW, ncurses::COLOR_BLACK),
        EFarbe::Gras => (ncurses::COLOR_GREEN, ncurses::COLOR_BLACK),
        EFarbe::Herz => (ncurses::COLOR_RED, ncurses::COLOR_BLACK),
        EFarbe::Schelln => (ncurses::COLOR_CYAN, ncurses::COLOR_BLACK),
    }};
    let i_color_tpl = tplcolorcolor.0 * 8 + tplcolorcolor.1;
    ncurses::init_pair(i_color_tpl, tplcolorcolor.0, tplcolorcolor.1);
    let nccolorpair = ncurses::COLOR_PAIR(i_color_tpl);
    ncurses::wattron(ncwin, nccolorpair);
    wprint(ncwin, &format!("{}", card));
    ncurses::wattroff(ncwin, nccolorpair);
}

plain_enum_mod!(moderelativeplayerposition, ERelativePlayerPosition {
    Myself,
    Left,
    VisAVis,
    Right,
});

trait TEplayerIndexExt { // TODO? move to lib?
    fn to_relativeplayerposition(self, epi_myself: EPlayerIndex) -> ERelativePlayerPosition;
}

impl TEplayerIndexExt for EPlayerIndex {
    fn to_relativeplayerposition(self, epi_myself: EPlayerIndex) -> ERelativePlayerPosition {
        static_assert!(assert_eq(EPlayerIndex::SIZE, ERelativePlayerPosition::SIZE));
        match self.wrapped_difference(epi_myself).0 {
            EPlayerIndex::EPI0 => ERelativePlayerPosition::Myself,
            EPlayerIndex::EPI1 => ERelativePlayerPosition::Left,
            EPlayerIndex::EPI2 => ERelativePlayerPosition::VisAVis,
            EPlayerIndex::EPI3 => ERelativePlayerPosition::Right,
        }
    }
}

enum VSkUiWindow {
    Stich,
    Interaction,
    Hand,
    PlayerInfo (ERelativePlayerPosition),
    GameInfo,
    AccountBalance,
}

fn do_in_window<RetVal>(skuiwin: &VSkUiWindow, fn_do: impl FnOnce(ncurses::WINDOW)->RetVal) -> RetVal {
    let (n_height, n_width) = via_out_param(|(n_height, n_width)| ncurses::getmaxyx(ncurses::stdscr(), n_height, n_width)).0;
    let create_fullwidth_window = |n_top, n_bottom| {
        ncurses::newwin(
            n_bottom-n_top, // height
            n_width, // width
            n_top, // y
            0 // x
        )
    };
    let ncwin = match *skuiwin {
        VSkUiWindow::PlayerInfo(erelativeplayerpos) => {
            match erelativeplayerpos {
                ERelativePlayerPosition::Myself => {
                    create_fullwidth_window(n_height-2, n_height-1)
                },
                ERelativePlayerPosition::Left | ERelativePlayerPosition::VisAVis | ERelativePlayerPosition::Right => {
                    ncurses::newwin(
                        1, // height
                        24, // width
                        0, // y
                        ((erelativeplayerpos.to_usize() - 1)*25).as_num() // x
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
    ncurses::werase(ncwin);
    let retval = fn_do(ncwin);
    ncurses::wrefresh(ncwin);
    ncurses::delwin(ncwin);
    retval
}

pub fn print_stichseq(epi_myself: EPlayerIndex, stichseq: &SStichSequence) {
    do_in_window(&VSkUiWindow::Stich, |ncwin| {
        for (i_stich, stich) in stichseq.visible_stichs().iter().enumerate() {
            let n_x = (i_stich*10+3).as_num();
            let n_y = 1;
            let n_card_width = 2;
            for epi in EPlayerIndex::values() {
                let move_cursor = |n_y_inner, n_x_inner| {
                    ncurses::wmove(ncwin, n_y_inner, n_x_inner);
                };
                match epi.to_relativeplayerposition(epi_myself) {
                    ERelativePlayerPosition::Myself => move_cursor(n_y+1, n_x),
                    ERelativePlayerPosition::Left => move_cursor(n_y, n_x-n_card_width),
                    ERelativePlayerPosition::VisAVis => move_cursor(n_y-1, n_x),
                    ERelativePlayerPosition::Right => move_cursor(n_y, n_x+n_card_width),
                };
                wprint(ncwin, if epi==stich.first_playerindex() { ">" } else { " " });
                match stich.get(epi) {
                    None => {wprint(ncwin, "..")},
                    Some(card) => {print_card_with_farbe(ncwin, *card)},
                };
            }
        }
    });
}

pub fn print_game_announcements(epi_myself: EPlayerIndex, gameannouncements: &SGameAnnouncements) {
    for (epi, orules) in gameannouncements.iter() {
        do_in_window(&VSkUiWindow::PlayerInfo(epi.to_relativeplayerposition(epi_myself)), |ncwin| {
            if let Some(ref rules) = *orules {
                wprint(ncwin, &format!("{}: {}", epi, rules));
            } else {
                wprint(ncwin, &format!("{}: Nothing", epi));
            }
        });
    }
}

pub fn print_game_info(rules: &SRules, expensifiers: &SExpensifiers) {
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
            expensifiers.doublings.iter()
                .filter(|&(_epi, b_doubling)| *b_doubling)
                .map(|(epi, _b_doubling)| epi)
                .collect()
        );
        print_special(
            ". Stoesse: ",
            expensifiers.vecstoss.iter()
                .map(|stoss| stoss.epi)
                .collect()
        );
    })
}

pub fn account_balance_string(an: &EnumMap<EPlayerIndex, isize>, n_stock: isize) -> String {
    EPlayerIndex::values()
        .map(|epi| format!("{}: {} | ", epi, an[epi]))
        .join("")
        + type_inference!(&str, &format!("Stock: {}", n_stock))
}

pub fn print_account_balance(an: &EnumMap<EPlayerIndex, isize>, n_stock: isize) {
    do_in_window(&VSkUiWindow::AccountBalance, |ncwin| {
        wprint(ncwin, &account_balance_string(an, n_stock));
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

pub fn ask_for_alternative<'vect, T>(
    vect: &'vect [T],
    askforalternativekeybindings: &SAskForAlternativeKeyBindings,
    fn_filter: impl Fn(&T)->bool,
    fn_callback: impl Fn(ncurses::WINDOW, usize, &Option<T>),
    fn_suggest: impl Fn()->Option<T>
) -> &'vect T {
    do_in_window(&VSkUiWindow::Interaction, |ncwin| {
        let mut ot_suggest = None;
        let vect = vect.iter().enumerate().filter(|&(_i_t, t)| fn_filter(t)).collect::<Vec<_>>();
        assert!(!vect.is_empty());
        let mut i_alternative = 0; // initially, point to 0th alternative
        fn_callback(ncwin, vect[i_alternative].0, &ot_suggest);
        ncurses::refresh();
        if 1<vect.len() {
            let mut ch = askforalternativekeybindings.key_prev;
            while ch!=askforalternativekeybindings.key_choose {
                if ch==askforalternativekeybindings.key_prev {
                    #[allow(clippy::implicit_saturating_sub)] // TODO? something like SInterval::clamp
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
                ncurses::werase(ncwin);
                fn_callback(ncwin, vect[i_alternative].0, &ot_suggest);
                ncurses::refresh();
                ch = ncurses::getch();
            }
        }
        ncurses::werase(ncwin);
        vect[i_alternative].1
    })
}

pub fn print_hand(veccard: &[ECard], oi_card: Option<usize>) {
    do_in_window(&VSkUiWindow::Hand, |ncwin| {
        let is_oi_card = |i| { oi_card.map_or(false, |i_card| i==i_card) };
        for (i, card) in veccard.iter().enumerate() {
            let n_card_width = 10;
            ncurses::wmove(ncwin,
                /*n_y; convert bool to isize*/ i32::from(!is_oi_card(i)),
                /*n_x*/ (n_card_width * i).as_num()
            );
            wprint(ncwin, " +--");
            print_card_with_farbe(ncwin, *card);
            wprint(ncwin, "--+ ");
        }
    });
}
