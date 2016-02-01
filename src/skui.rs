use stich::*;

pub fn print_vecstich(vecstich: &Vec<CStich>) {
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
        print!("   {}    ", tui_card_string(stich, /*eplayerindex*/2));
    }
    println!("");
    for stich in vecstich {
        print!(" {} {}  ",
           tui_card_string(stich, /*eplayerindex*/1),
           tui_card_string(stich, /*eplayerindex*/3)
        );
    }
    println!("");
    for stich in vecstich {
        print!("   {}    ", tui_card_string(stich, /*eplayerindex*/0));
    }
    println!("");
}

