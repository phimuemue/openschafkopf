use openschafkopf_lib::{
    game_analysis::parser::{analyze_sauspiel_html, analyze_sauspiel_json},
    game::*,
    rules::{
        SRuleStateCache,
        ruleset::{VStockOrT, TRuleSet},
        TRules,
    },
    primitives::*,
};
use openschafkopf_util::*;
use itertools::Itertools;
use std::io::Write;
use failure::*;
use plain_enum::PlainEnum;

pub fn subcommand(str_subcommand: &'static str) -> clap::Command {
    use super::shared_args::*;
    clap::Command::new(str_subcommand)
        .about("Parse a game into a simple format")
        .arg(input_files_arg("file"))
        .arg(clap::Arg::new("neural-net")
            .long("neural-net")
        )
        .arg(clap::Arg::new("raw")
            .long("raw")
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

fn card_to_neural_network_input(ocard: Option<ECard>) -> usize {
    if let Some(card) = ocard {
        macro_rules! inner(($(($efarbe:ident, $eschlag:ident, $n:expr))*) => {
            match (card.farbe(), card.schlag()) {
                $((EFarbe::$efarbe, ESchlag::$eschlag) => $n,)*
            }
        });
        verify_ne!(card_neural_network_mapping!(inner), 0)
    } else {
        0
    }
}

fn neural_network_input_to_card(n: usize) -> Result<Option<ECard>, &'static str> {
    macro_rules! inner(($(($efarbe:ident, $eschlag:ident, $n:expr))*) => {
        match n {
            0 => Ok(None),
            $($n => Ok(Some(ECard::new(EFarbe::$efarbe, ESchlag::$eschlag))),)*
            _/*TODORUST 33..=usize::MAX*/ => Err("Unknown neural network input index"),
        }
    });
    card_neural_network_mapping!(inner)
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let mut omapstrfile_neural_net = if_then_some!(clapmatches.is_present("neural-net"),
        std::collections::HashMap::new()
    );
    let opath_raw_parse = if_then_some!(clapmatches.is_present("raw"),
        std::path::PathBuf::from(&format!("raw/{}",
            chrono::Local::now().format("%Y%m%d%H%M%S"),
        ))
    );
    fn write_columns<
        PlayerIndexActive: std::fmt::Display,
        PlayerIndexStichSeq: std::fmt::Display,
        CardIndicatorVariable: std::fmt::Display,
        CardStichSeq: std::fmt::Display,
        ResultColumn: std::fmt::Display,
        PointsPerPlayer: std::fmt::Display,
        StichsPerPlayer: std::fmt::Display,
    >(
        wrtr: &mut impl std::io::Write,
        oepi_active: Option<PlayerIndexActive>,
        fn_card_in_hand: impl Fn(usize, ECard) -> CardIndicatorVariable,
        fn_card_allowed: impl Fn(usize, ECard) -> CardIndicatorVariable,
        stichseq: &SStichSequence,
        fn_card_stichseq: impl Fn(usize, Option<ECard>) -> CardStichSeq,
        fn_epi_stichseq: impl Fn(usize, Option<EPlayerIndex>) -> PlayerIndexStichSeq,
        fn_points_for_player: impl Fn(EPlayerIndex) -> PointsPerPlayer,
        fn_stichs_for_player: impl Fn(EPlayerIndex) -> StichsPerPlayer,
        fn_result_column: impl Fn(EPlayerIndex, ECard)->ResultColumn,
    ) {
        if let Some(ref epi_active)=oepi_active {
            unwrap!(write!(wrtr, "{},", epi_active));
        }
        let ekurzlang = stichseq.kurzlang();
        let n_cards_total = ekurzlang.cards_per_player() * EPlayerIndex::SIZE;
        fn write_indicator_vars<CardIndicatorVariable: std::fmt::Display>(
            wrtr: &mut impl std::io::Write,
            n_cards_total: usize,
            fn_card_indicator_variable: impl Fn(usize, ECard)->CardIndicatorVariable,
        ) {
            for i_card_0_based in 0..n_cards_total {
                let i_card = i_card_0_based + 1;
                unwrap!(write!(
                    wrtr,
                    "{},",
                    fn_card_indicator_variable(
                        i_card,
                        unwrap!(unwrap!(neural_network_input_to_card(i_card))),
                    ),
                ));
            }
        }
        write_indicator_vars(wrtr, n_cards_total, fn_card_in_hand);
        write_indicator_vars(wrtr, n_cards_total, fn_card_allowed);
        for (i_card_stichseq, otplepicard_stichseq) in stichseq.visible_cards()
            .map(|(epi, &card_stichseq)| (Some((epi, card_stichseq))))
            .pad_using(n_cards_total, |_| None)
            .enumerate()
        {
            unwrap!(write!(wrtr, "{},", fn_card_stichseq(i_card_stichseq, otplepicard_stichseq.map(|tplepicard| tplepicard.1))));
            unwrap!(write!(wrtr, "{},", fn_epi_stichseq(i_card_stichseq, otplepicard_stichseq.map(|tplepicard| tplepicard.0))));
        }
        for epi in EPlayerIndex::values() {
            unwrap!(write!(wrtr, "{},", fn_points_for_player(epi)));
            unwrap!(write!(wrtr, "{},", fn_stichs_for_player(epi)));
        }
        for (epi, card) in itertools::iproduct!(EPlayerIndex::values(), ECard::values(ekurzlang)) {
            if ekurzlang.supports_card(card) {
                unwrap!(write!(wrtr, "{},", fn_result_column(epi, card)));
            }
        }
        unwrap!(write!(wrtr, "\n"));
    }
    let path_neural_network = std::path::PathBuf::from(&format!("neural_network_input/{}",
        chrono::Local::now().format("%Y%m%d%H%M%S"),
    ));
    super::glob_files_or_read_stdin(
        clapmatches.values_of("file").into_iter().flatten(),
        |opath, str_input, i_input| {
            if let Ok(ref gameresult@SGameResultGeneric{stockorgame: VStockOrT::OrT(ref game), ..}) = analyze_sauspiel_html(&str_input)
                .map(|game| game.map(|_|(), |_|(), |ruleset| ruleset.kurzlang()))
                .or_else(|_err| analyze_sauspiel_json(&str_input, |_,_,_,_| {}))
            {
                let mut game_csv = SGame::new(
                    game.aveccard.clone(),
                    SExpensifiersNoStoss::new_with_doublings(
                        game.expensifiers.n_stock,
                        game.expensifiers.doublings.clone(),
                    ),
                    game.rules.clone(),
                );
                assert_eq!(game.stichseq.visible_stichs(), game.stichseq.completed_stichs());
                if let Some(ref mut mapstrfile_neural_net) = omapstrfile_neural_net {
                    let path_gameresult = path_neural_network.join(super::gameresult_to_dir(gameresult));
                    let oepi_active = verify_eq!(game_csv.rules.playerindex(), game.rules.playerindex());
                    let file = mapstrfile_neural_net.entry(path_gameresult.clone())
                        .or_insert_with(|| {
                            unwrap!(std::fs::create_dir_all(&path_gameresult));
                            let mut file = std::io::BufWriter::new(unwrap!(std::fs::File::create(path_gameresult.join("csv.csv"))));
                            write_columns(
                                &mut file,
                                oepi_active.map(|_epi| "epi_active"),
                                /*fn_card_in_hand*/|i_card, _card| format!("card_hand_{}", i_card),
                                /*fn_card_allowed*/|i_card, _card| format!("card_allowed_{}", i_card),
                                &game.stichseq,
                                /*fn_card_stichseq*/|i_card, _ocard| format!("card_stichseq_{}", i_card),
                                /*fn_epi_stichseq*/|i_card, _oepi| format!("epi_stichseq_{}", i_card),
                                /*fn_points_for_player*/|epi| format!("n_points_{}", epi),
                                /*fn_stichs_for_player*/|epi| format!("n_stichs_{}", epi),
                                /*fn_result_column*/|epi, card| format!("{}_at_epi{}", card, epi.to_usize()),
                            );
                            file
                        });
                    let mut rulestatecache = SRuleStateCache::new(
                        (&game_csv.ahand, &game_csv.stichseq),
                        &game_csv.rules,
                    );
                    for stich in verify_eq!(game.stichseq.completed_stichs(), game.stichseq.visible_stichs()).iter().map(SFullStich::new) {
                        for (epi, &card_zugeben) in stich.iter() {
                            assert_eq!(epi, unwrap!(game_csv.which_player_can_do_something()).0);
                            let veccard_allowed = game_csv.rules.all_allowed_cards(&game_csv.stichseq, &game_csv.ahand[epi]);
                            let bool_to_usize = usize::from;
                            write_columns(
                                file,
                                oepi_active,
                                /*fn_card_in_hand*/|_i_card, card| bool_to_usize(game_csv.ahand[epi].contains(card)),
                                /*fn_card_allowed*/|_i_card, card| bool_to_usize(veccard_allowed.contains(&card)),
                                &game_csv.stichseq,
                                /*fn_card_stichseq*/|_i_card, ocard| {
                                    card_to_neural_network_input(ocard)
                                },
                                /*fn_epi_stichseq*/|_i_card, oepi| {
                                    if let Some(epi) = oepi {
                                        verify_ne!(epi.to_usize() + 1, 0)
                                    } else {
                                        0
                                    }
                                },
                                /*fn_points_for_player*/|epi| rulestatecache.changing.mapepipointstichcount[epi].n_point,
                                /*fn_stichs_for_player*/|epi| rulestatecache.changing.mapepipointstichcount[epi].n_stich,
                                /*fn_result_column*/|epi, card| bool_to_usize(rulestatecache.fixed.who_has_card(card)==epi) ,
                            );
                            unwrap!(game_csv.zugeben(card_zugeben, epi)); // validated by analyze_sauspiel_html
                        }
                        rulestatecache.register_stich(stich, game_csv.rules.winner_index(stich));
                        debug_assert_eq!(
                            rulestatecache,
                            SRuleStateCache::new(
                                (&game_csv.ahand, &game_csv.stichseq),
                                &game_csv.rules,
                            ),
                        );
                    }
                }
                if let Some(ref path_raw_parse) = opath_raw_parse {
                    let path_gameresult = path_raw_parse.join(super::gameresult_to_dir(gameresult));
                    unwrap!(std::fs::create_dir_all(&path_gameresult));
                    unwrap!(unwrap!(std::fs::File::create(
                        path_gameresult.join(format!("{}.html", i_input))
                    )).write_all(str_input.as_bytes()));
                }
            } else {
                eprintln!("Nothing found in {:?}: Trying to continue.", opath);
                if let Some(ref path_raw_parse) = opath_raw_parse {
                    let path_err = path_raw_parse.join("error");
                    unwrap!(std::fs::create_dir_all(&path_err));
                    unwrap!(unwrap!(std::fs::File::create(
                        path_err.join(format!("{}.html", i_input))
                    )).write_all(str_input.as_bytes()));
                }
            }
        },
    )?;
    if let Some(mut mapstrfile_neural_net) = omapstrfile_neural_net {
        for (_str_path, file) in mapstrfile_neural_net.iter_mut() {
            unwrap!(file.flush());
        }
    }
    Ok(())
}
