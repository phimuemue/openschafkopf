use crate::subcommands::analyze::{analyze_sauspiel_html, analyze_plain}; // TODO move functions to own module
use crate::game::*;
use crate::rules::ruleset::{VStockOrT};
use crate::util::*;
use itertools::Itertools;
use crate::primitives::*;
use std::io::Write;

pub fn subcommand(str_subcommand: &str) -> clap::App {
    clap::SubCommand::with_name(str_subcommand)
        .about("Parse a game into a simple format")
        .arg(clap::Arg::with_name("file") // TODO? shared function to glob for files
            .required(true)
            .takes_value(true)
            .multiple(true)
        )
}

macro_rules! card_neural_network_mapping(($macro:ident) => {
    $macro!(
        (Eichel, Ass, 1)
        (Gras, Ass, 2)
        (Herz, Ass, 3)
        (Schelln, Ass, 4)
        (Eichel, Zehn, 5)
        (Gras, Zehn, 6)
        (Herz, Zehn, 7)
        (Schelln, Zehn, 8)
        (Eichel, Koenig, 9)
        (Gras, Koenig, 10)
        (Herz, Koenig, 11)
        (Schelln, Koenig, 12)
        (Eichel, Ober, 13)
        (Gras, Ober, 14)
        (Herz, Ober, 15)
        (Schelln, Ober, 16)
        (Eichel, Unter, 17)
        (Gras, Unter, 18)
        (Herz, Unter, 19)
        (Schelln, Unter, 20)
        (Eichel, S9, 21)
        (Gras, S9, 22)
        (Herz, S9, 23)
        (Schelln, S9, 24)
        (Eichel, S8, 25)
        (Gras, S8, 26)
        (Herz, S8, 27)
        (Schelln, S8, 28)
        (Eichel, S7, 29)
        (Gras, S7, 30)
        (Herz, S7, 31)
        (Schelln, S7, 32)
    )
});

fn card_to_neural_network_input(ocard: Option<SCard>) -> usize {
    if let Some(card) = ocard {
        macro_rules! inner(($(($efarbe:ident, $eschlag:ident, $n:expr))*) => {
            match (card.farbe(), card.schlag()) {
                $((EFarbe::$efarbe, ESchlag::$eschlag) => $n,)*
            }
        });
        card_neural_network_mapping!(inner)
    } else {
        0
    }
}

fn neural_network_input_to_card(n: usize) -> Result<Option<SCard>, Error> {
    macro_rules! inner(($(($efarbe:ident, $eschlag:ident, $n:expr))*) => {
        match n {
            0 => Ok(None),
            $($n => Ok(Some(SCard::new(EFarbe::$efarbe, ESchlag::$eschlag))),)*
            _/*TODORUST 33..=usize::MAX*/ => bail!("Unknown neural network input index"),
        }
    });
    card_neural_network_mapping!(inner)
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let mut mapstrfile = std::collections::HashMap::new();
    let path_dst = std::path::PathBuf::from(&format!("neural_network_input/{}",
        chrono::Local::now().format("%Y%m%d%H%M%S"),
    ));
    super::glob_files(
        unwrap!(clapmatches.values_of("file")),
        |path, str_input| {
            if let Ok(ref gameresult@SGameResultGeneric{stockorgame: VStockOrT::OrT(ref game), ..}) = analyze_sauspiel_html(&str_input) {
                let str_out = format!("{}{}: {}",
                    game.rules,
                    if let Some(epi) = game.rules.playerindex() {
                        format!(" von {}", epi)
                    } else {
                        "".into()
                    },
                    game.stichseq.visible_cards()
                        .map(|(_epi, card)| card)
                        .join(" "),
                );
                {
                    let game_check = unwrap!(unwrap!(analyze_plain(&str_out).exactly_one()));
                    assert_eq!(game_check.rules.to_string(), game.rules.to_string()); // TODO? better comparison?
                    assert_eq!(game_check.rules.playerindex(), game.rules.playerindex());
                    assert_eq!(game_check.stichseq, game.stichseq);
                }
                let mut game_csv = SGame::new(
                    game.aveccard.clone(),
                    game.doublings.clone(),
                    game.ostossparams.clone(),
                    game.rules.clone(),
                    game.n_stock,
                );
                assert_eq!(game.stichseq.visible_stichs(), game.stichseq.completed_stichs());
                let path_gameresult = path_dst.join(super::gameresult_to_dir(&gameresult));
                let file = mapstrfile.entry(path_gameresult.clone())
                    .or_insert_with(|| {
                        unwrap!(std::fs::create_dir_all(&path_gameresult));
                        std::io::BufWriter::new(unwrap!(std::fs::File::create(path_gameresult.join("csv.csv"))))
                    });
                let oepi_active = verify_eq!(game_csv.rules.playerindex(), game.rules.playerindex());
                let ekurzlang = verify_eq!(game_csv.kurzlang(), game.kurzlang());
                for (epi, &card_zugeben) in game.stichseq.visible_cards() {
                    assert_eq!(epi, unwrap!(game_csv.which_player_can_do_something()).0);
                    if let Some(ref epi_active)=oepi_active {
                        unwrap!(write!(file, "{},", epi_active));
                    }
                    for ocard_hand in game_csv.ahand[epi].cards().iter().copied().map(Some)
                        .pad_using(ekurzlang.cards_per_player(), |_| None)
                    {
                        unwrap!(write!(file, "{},", card_to_neural_network_input(ocard_hand)));
                    }
                    for ocard_played_so_far in game_csv.stichseq.visible_cards()
                        .map(|(_epi, &card_played_so_far)| Some(card_played_so_far))
                        .pad_using(ekurzlang.cards_per_player() * EPlayerIndex::SIZE, |_| None)
                    {
                        unwrap!(write!(file, "{},", card_to_neural_network_input(ocard_played_so_far)));
                    }
                    unwrap!(write!(file, "{}", card_to_neural_network_input(Some(card_zugeben))));
                    unwrap!(write!(file, "\n"));
                    unwrap!(game_csv.zugeben(card_zugeben, epi)); // validated by analyze_sauspiel_html
                }
            } else {
                eprintln!("Nothing found in {:?}: Trying to continue.", path);
            }
        },
    )?;
    for (_str_path, file) in mapstrfile.iter_mut() {
        unwrap!(file.flush());
    }
    Ok(())
}
