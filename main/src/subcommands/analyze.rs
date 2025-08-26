use openschafkopf_lib::{
    primitives::*,
    game_analysis::{*, parser::*},
    game::*,
    ai::{handiterators::*, gametree::*, *},
    rules::{TRules, ruleset::VStockOrT},
};
use openschafkopf_util::*;
use std::{
    borrow::Cow,
    io::Write,
    time::Instant,
    sync::{Arc, atomic::{AtomicUsize, Ordering}, Mutex},
};
use std::fmt::Write as _;
use failure::*;
use rayon::prelude::*;
use itertools::Itertools;
use plain_enum::*;

pub fn subcommand(str_subcommand: &'static str) -> clap::Command<'static> {
    use super::shared_args::*;
    clap::Command::new(str_subcommand)
        .about("Analyze played games and spot suboptimal decisions")
        .arg(input_files_arg("sauspiel-files"))
        .arg(clap::Arg::new("include-no-findings") // TODO replace this by interactive option in resulting HTML
            .long("include-no-findings")
        )
        .arg(clap::Arg::new("simulate-all-hands")
            .long("simulate-all-hands")
            .help("Perform analysis as if other cards are unknown")
            .long_help("Perform analysis not only for the given distribution of cards, but instead for all possible combinations from the respective player's point of view.")
        )
        .arg(super::clap_arg("max-remaining-cards", if_dbg_else!({"2"}{"4"}))
            .help("Analyze only if a hand contains at most a certain amount of cards")
            .long_help("Perform analysis only if the respective player at the respective point of the game has at most a certain amount of cards left. Can be used to reduce computation time.")
        )
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let mut vecgamewithdesc = Vec::new();
    super::glob_files_or_read_stdin(
        clapmatches.values_of("sauspiel-files").into_iter().flatten(),
        |opath, str_input, _i_input| {
            let str_path = match &opath {
                Some(path) => path.to_string_lossy(),
                None => Cow::Borrowed("stdin"), // hope that path is not "stdin"
            };
            println!("Opened {str_path}");
            let mut b_found = false;
            let mut push_game = |str_description: String, resgameresult: Result<_, _>| {
                b_found = b_found || resgameresult.is_ok();
                vecgamewithdesc.push(SGameWithDesc{
                    str_description,
                    resgameresult,
                });
            };
            if let resgameresult@Ok(_) = analyze_sauspiel_html(&str_input)
                .map(|game| game.map(|_|(), |_|(), |_|()))
                .or_else(|_err| analyze_sauspiel_json(&str_input, |_,_,_,_| {})
                    .map(|game| game.map(|_|(), |_|(), |_|()))
                )
            {
                push_game(
                    str_path.clone().into_owned(),
                    resgameresult
                )
            } else {
                let mut b_found_plain = false;
                for (i, resgame) in analyze_plain(&str_input)
                    .chain(analyze_netschafkopf(&str_input).into_iter().flatten()
                        .map(|resgameresult| resgameresult.and_then(|gameresult| {
                            match gameresult.stockorgame {
                                VStockOrT::Stock(()) => Err(format_err!("Nothing to analyze.")),
                                VStockOrT::OrT(game) => Ok(game),
                            }
                        }))
                    )
                    .filter(|res| res.is_ok())
                    .enumerate()
                {
                    b_found_plain = true;
                    push_game(
                        format!("{str_path}_{i}"),
                        resgame.and_then(|game| game.finish().map_err(|_game| format_err!("Could not game.finish")))
                    )
                }
                if !b_found_plain {
                    push_game(str_path.clone().into_owned(), Err(format_err!("Nothing found in {}: Trying to continue.", str_path)));
                }
            }
            if !b_found {
                eprintln!("Nothing found in {str_path}: Trying to continue.");
            }
        },
    )?;
    let path_openschafkopf_executable = unwrap!(unwrap!(std::env::current_exe()).canonicalize());
    let path_out = analyze_games(
        std::path::Path::new("./analyze"), // TODO make customizable
        /*fn_link*/|str_description: &str| str_description.to_string(),
        vecgamewithdesc,
        /*b_include_no_findings*/clapmatches.is_present("include-no-findings"),
        /*n_max_remaining_cards*/unwrap!(clapmatches.value_of("max-remaining-cards")).parse()?,
        /*b_simulate_all_hands*/clapmatches.is_present("simulate-all-hands"),
        /*str_openschafkopf_executable*/unwrap!(path_openschafkopf_executable.to_str()),
        /*fn_output_card*/&|card, b_highlight| output_card(card, b_highlight).to_string(),
    )?;
    println!("Analysis written to {}.", path_out.display());
    Ok(())
}

struct SGameWithDesc {
    pub str_description: String,
    pub resgameresult: Result<SGameResult</*Ruleset*/()>, failure::Error>,
}

fn analyze_game(
    game_in: SGame,
    n_max_remaining_cards: usize,
    b_simulate_all_hands: bool,
) -> SGameAnalysis {
    let mut vecanalysispercard = Vec::new();
    let an_payout = unwrap!(game_in.clone().finish()).an_payout;
    let mut mapepivecpossiblepayout = EPlayerIndex::map_from_fn(|_epi| Vec::new());
    let game = unwrap!(SGame::new(
        game_in.aveccard.clone(),
        SExpensifiersNoStoss::new_with_doublings(
            game_in.expensifiers.n_stock,
            game_in.expensifiers.doublings.clone(),
        ),
        game_in.rules.clone()
    ).play_cards_and_stoss(
        &game_in.expensifiers.vecstoss,
        game_in.stichseq.visible_cards(),
        /*fn_before_zugeben*/|game, i_stich, epi_zugeben, card_played| {
            if game.stichseq.remaining_cards_per_hand()[epi_zugeben] <= n_max_remaining_cards {
                let stichseq = &game.stichseq;
                let fwd_to_determine_best_card = |epi, itahand| {
                    unwrap!(determine_best_card(
                        stichseq,
                        itahand,
                        equivalent_cards_filter(
                            /*n_until_stichseq_len, determined heuristically*/7,
                            game.rules.equivalent_when_on_same_hand(),
                        ),
                        /*TODO use SAlphaBetaPruner*/&|_stichseq, _ahand| SMinReachablePayout::new(
                            &game.rules,
                            epi,
                            game.expensifiers.clone(),
                        ),
                        /*fn_snapshotcache*/SSnapshotCacheNone::factory(), // TODO possibly use cache
                        /*fn_visualizer*/SNoVisualization::factory(),
                        /*fn_inspect*/&|_inspectionpoint, _i_ahand, _ahand| {},
                        /*fn_payout*/&|_stichseq, _ahand, n_payout| (n_payout, ()),
                    ))
                };
                for epi in EPlayerIndex::values() {
                    mapepivecpossiblepayout[epi].push(SPossiblePayout(
                        fwd_to_determine_best_card(
                            epi,
                            Box::new(std::iter::once(game.ahand.clone())) as Box<_>,
                        ),
                        (i_stich, stichseq.current_stich().size()),
                    ));
                }
                let look_for_mistakes = |determinebestcardresult: &SDetermineBestCardResult<SPerMinMaxStrategy<SPayoutStats<()>>>| {
                    macro_rules! look_for_mistake{($strategy:ident, $emistake:expr) => {{
                        let (veccard, minmax) = determinebestcardresult.cards_with_maximum_value(
                            |minmax_lhs, minmax_rhs| minmax_lhs.$strategy.0.min().cmp(&minmax_rhs.$strategy.0.min())
                        );
                        if_then_some!(
                            an_payout[epi_zugeben]<minmax.$strategy.0.min()
                                && !veccard.contains(&card_played), // TODO can we improve this?
                            SAnalysisCardAndPayout{
                                veccard,
                                n_payout: minmax.$strategy.0.min(),
                                emistake: $emistake,
                            }
                        ) // else The decisive mistake must occur in subsequent stichs. TODO assert that it actually occurs
                    }}}
                    look_for_mistake!(maxmin, EMistake::Min)
                        .or_else(|| look_for_mistake!(maxselfishmin, EMistake::SelfishMin))
                };
                let epi_current = unwrap!(game.stichseq.current_stich().current_playerindex());
                let look_for_mistakes_simulating = || {
                    look_for_mistakes(
                        &fwd_to_determine_best_card(
                            epi_zugeben,
                            Box::new(all_possible_hands(
                                &game.stichseq,
                                (game.ahand[epi_current].clone(), epi_current),
                                &game.rules,
                                &game.expensifiers.vecstoss,
                            )) as Box<_>,
                        )
                    )
                };
                let determinebestcardresult_cheating = unwrap!(mapepivecpossiblepayout[epi_current].last()).0.clone();
                let ocardandpayout_cheating = look_for_mistakes(&determinebestcardresult_cheating);
                vecanalysispercard.push(SAnalysisPerCard {
                    determinebestcardresult_cheating,
                    stichseq: game.stichseq.clone(),
                    card_played,
                    ahand: game.ahand.clone(),
                    oanalysisimpr: /*TODO? if_then_some*/if let Some(analysisimpr) = ocardandpayout_cheating
                            .map(|cardandpayout_cheating| {
                                SAnalysisImprovement {
                                    cardandpayout_cheating,
                                    improvementsimulating: if !b_simulate_all_hands {
                                        VImprovementSimulating::NotRequested
                                    } else if let Some(cardandpayout_simulating) = look_for_mistakes_simulating() {
                                        VImprovementSimulating::Found(cardandpayout_simulating)
                                    } else {
                                        VImprovementSimulating::NothingFound
                                    },
                                }
                            })
                        {
                            Some(analysisimpr)
                        } else {
                            debug_assert!(look_for_mistakes_simulating().is_none());
                            None
                        }
                })
            }
        }
    ));
    SGameAnalysis {
        game,
        vecanalysispercard,
        mapepivecpossiblepayout,
    }
}

fn analyze_games(path_analysis: &std::path::Path, fn_link: impl Fn(&str)->String+Sync, vecgamewithdesc: Vec<SGameWithDesc>, b_include_no_findings: bool, n_max_remaining_cards: usize, b_simulate_all_hands: bool, str_openschafkopf_executable: &str, fn_output_card: &(dyn Fn(ECard, bool/*b_highlight*/)->String + Sync)) -> Result<std::path::PathBuf, std::io::Error> {
    // TODO can all this be done with fewer locks and more elegant?
    std::fs::create_dir_all(path_analysis)?;
    generate_html_auxiliary_files(path_analysis)?;
    let str_date = format!("{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
    let str_index_html = Arc::new(Mutex::new(format!(
        r###"
        <!DOCTYPE html>
        <html lang="de" class="no-js">
            <head>
                <title>Schafkopf-Analyse: {str_date}</title>
                <meta http-equiv="Content-Type" content="text/html; charset=UTF-8">
            </head>
            <body>
                <h1>Schafkopf-Analyse: {str_date}</h1>
                "###,
    )));
    *unwrap!(str_index_html.lock()) += "<table>";
    let n_games_total = vecgamewithdesc.len();
    let n_games_done = Arc::new(AtomicUsize::new(0));
    let n_games_non_stock = Arc::new(AtomicUsize::new(0));
    let n_games_findings = Arc::new(AtomicUsize::new(0));
    vecgamewithdesc.into_par_iter().try_for_each(|gamewithdesc| -> Result<_, std::io::Error> {
        if let Ok(gameresult) = gamewithdesc.resgameresult {
            match gameresult.stockorgame {
                VStockOrT::Stock(_) => {
                    if b_include_no_findings {
                        unwrap!(write!(
                            *unwrap!(str_index_html.lock()),
                            r#"<tr>
                                <td>
                                    Stock: {}
                                </td>
                            </tr>"#,
                            gameresult.an_payout.iter().join("/"),
                        ));
                    }
                },
                VStockOrT::OrT(game) => {
                    n_games_non_stock.fetch_add(1, Ordering::SeqCst);
                    let str_rules = format!("{}", game.rules);
                    let path_analysis_game = path_analysis.join(gamewithdesc.str_description.replace(['/', '.'], "_"));
                    std::fs::create_dir_all(&path_analysis_game)?;
                    let path = path_analysis_game.join("analysis.html");
                    let instant_analysis_begin = Instant::now();
                    let gameanalysis = analyze_game(
                        game,
                        n_max_remaining_cards,
                        b_simulate_all_hands,
                    );
                    let duration = instant_analysis_begin.elapsed();
                    let path = write_html(path, &gameanalysis.generate_analysis_html(
                        &gamewithdesc.str_description,
                        &fn_link(&gamewithdesc.str_description),
                        str_openschafkopf_executable,
                        fn_output_card,
                    ))?;
                    let itanalysisimpr = gameanalysis.vecanalysispercard.iter()
                        .filter_map(|analysispercard| analysispercard.oanalysisimpr.as_ref());
                    let n_findings_cheating = itanalysisimpr.clone().count(); // TODO distinguish emistake
                    let n_findings_simulating = itanalysisimpr.clone() // TODO distinguish emistake
                        .filter(|analysisimpr| matches!(analysisimpr.improvementsimulating, VImprovementSimulating::Found(_)))
                        .count();
                    assert!(n_findings_simulating <= n_findings_cheating);
                    if 0<n_findings_cheating {
                        n_games_findings.fetch_add(1, Ordering::SeqCst);
                    }
                    if b_include_no_findings || 0 < n_findings_cheating {
                        unwrap!(write!(
                            *unwrap!(str_index_html.lock()),
                            r#"<tr>
                                <td>
                                    <a href="{str_path}">{str_rules}</a>
                                </td>
                                <td>
                                    ({n_findings_simulating}/{n_findings_cheating} Funde)
                                </td>
                                <td>
                                    ({chr_stopwatch} {str_duration_as_secs})
                                </td>
                            </tr>"#,
                            str_path = unwrap!(
                                unwrap!(path.strip_prefix(path_analysis)).to_str()
                            ),
                            str_rules = str_rules,
                            n_findings_simulating = n_findings_simulating,
                            n_findings_cheating = n_findings_cheating,
                            chr_stopwatch = '\u{23F1}',
                            str_duration_as_secs = {
                                let n_secs = duration.as_secs();
                                if 0==n_secs {
                                    "&lt;1s".to_owned()
                                } else {
                                    format!("{n_secs}s")
                                }
                            },
                        ));
                    }
                },
            }
        } else {
            unwrap!(write!(*unwrap!(str_index_html.lock()), "<tr><td>Fehler ({})</td></tr>", gamewithdesc.str_description));
        }
        n_games_done.fetch_add(1, Ordering::SeqCst);
        let n_games_non_stock = n_games_non_stock.load(Ordering::SeqCst);
        let n_games_done = n_games_done.load(Ordering::SeqCst);
        let n_games_findings = n_games_findings.load(Ordering::SeqCst);
        println!("Total: {n_games_total}. Done: {n_games_done}. Non-stock: {n_games_non_stock}. With findings: {n_games_findings}");
        Ok(())
    })?;
    let mut str_index_html = unwrap!(unwrap!(Arc::try_unwrap(str_index_html)).into_inner());
    str_index_html += "</table>";
    str_index_html += "</body></html>";
    write_html(path_analysis.join(format!("{str_date}.html")), &str_index_html)
}

#[derive(Clone, Debug)]
pub enum EMistake {
    Min,
    SelfishMin,
}

#[derive(Clone, Debug)]
pub struct SAnalysisCardAndPayout {
    pub veccard: Vec<ECard>,
    pub n_payout: isize,
    pub emistake: EMistake,
}

#[derive(Clone)]
pub struct SAnalysisPerCard {
    pub stichseq: SStichSequence, // TODO this is space-inefficient
    ahand: EnumMap<EPlayerIndex, SHand>, // TODO this is space-inefficient
    card_played: ECard,
    determinebestcardresult_cheating: SDetermineBestCardResult<SPerMinMaxStrategy<SPayoutStats<()>>>,
    pub oanalysisimpr: Option<SAnalysisImprovement>,
}

#[derive(Clone, Debug)]
pub enum VImprovementSimulating {
    NotRequested,
    NothingFound,
    Found(SAnalysisCardAndPayout),
}

#[derive(Clone)]
pub struct SAnalysisImprovement {
    pub cardandpayout_cheating: SAnalysisCardAndPayout,
    pub improvementsimulating: VImprovementSimulating,
}

pub struct SGameAnalysis {
    pub game: SGame,
    pub vecanalysispercard: Vec<SAnalysisPerCard>,
    pub mapepivecpossiblepayout: EnumMap<EPlayerIndex, Vec<SPossiblePayout>>,
}

pub struct SPossiblePayout(
    SDetermineBestCardResult<SPerMinMaxStrategy<SPayoutStats<()>>>,
    (/*i_stich*/usize, /*i_card*/usize),
);

impl SGameAnalysis {
    fn generate_analysis_html(
        &self,
        str_description: &str,
        str_link: &str,
        str_openschafkopf_executable: &str,
        fn_output_card: &dyn Fn(ECard, bool/*b_highlight*/)->String,
    ) -> String {
        let game = &self.game;
        let mapepin_payout = unwrap!(game.clone().finish()).an_payout;
        assert!(game.which_player_can_do_something().is_none()); // TODO use SGameResult (see comment in SGameResult)
        let ahand = EPlayerIndex::map_from_fn(|epi| {
            SHand::new_from_iter(game.stichseq.completed_cards_by(epi))
        });
        let epi_self = EPlayerIndex::EPI0;
        let stich_caption = |stichseq: &SStichSequence| {
            let epi_current = unwrap!(stichseq.current_stich().current_playerindex());
            (
                format!("Stich {}, Spieler {}",
                    stichseq.completed_stichs().len() + 1, // humans count 1-based
                    epi_current,
                ),
                epi_current,
            )
        };
        format!(
            r###"<!DOCTYPE html>
            <html lang="de" class="no-js">
                <head>
                    <title>Schafkopf-Analyse: {str_description}</title>
                    <meta http-equiv="Content-Type" content="text/html; charset=UTF-8">
                    <link rel="stylesheet" type="text/css" href="../css.css">
                </head>
                <body>
                    <h1>Schafkopf-Analyse: <a href="{str_link}">{str_description}</a></h1>
                    <h2>{str_rules}</h2>"###,
            str_description=str_description,
            str_link=str_link,
            str_rules=rules_to_string(&game.rules),
        )
        + "<table><tr>"
        + type_inference!(&str, &player_table_ahand(epi_self, &ahand, &game.rules, /*fn_border*/|_card| false, fn_output_card))
        + "</tr></table><table><tr>"
        + type_inference!(&str, &player_table_stichseq(epi_self, &game.stichseq, fn_output_card))
        + "</tr></table>"
        + "<ul>"
        + type_inference!(&str, &format!("{}", self.vecanalysispercard.iter()
            .filter_map(|analysispercard| analysispercard.oanalysisimpr.as_ref().map(|analysisimpr|
                (analysisimpr, &analysispercard.stichseq)
            ))
            .map(|(analysisimpr, stichseq)| {
                let epi = unwrap!(stichseq.current_stich().current_playerindex());
                let mut str_analysisimpr = format!(
                    r###"<li>
                        {str_stich_caption}:
                        Bei gegebener Kartenverteilung: {str_card_suggested_cheating} {str_gewinn}: {n_payout_cheating} (statt {n_payout_real}).
                    </li>"###,
                    str_stich_caption=stich_caption(stichseq).0,
                    str_card_suggested_cheating = analysisimpr.cardandpayout_cheating.veccard
                        .iter()
                        .map(ECard::to_string)
                        .join(", "),
                    str_gewinn = match analysisimpr.cardandpayout_cheating.emistake {
                        EMistake::Min => "garantierter Mindestgewinn",
                        EMistake::SelfishMin => "Mindestgewinn, wenn jeder Spieler optimal spielt",
                    },
                    n_payout_cheating = analysisimpr.cardandpayout_cheating.n_payout,
                    n_payout_real = mapepin_payout[epi],
                );
                if let VImprovementSimulating::Found(ref cardandpayout) = analysisimpr.improvementsimulating {
                    unwrap!(write!(
                        str_analysisimpr,
                        r###"
                            <ul><li>
                            Bei unbekannter Kartenverteilung: {str_card_suggested} {str_gewinn}: {n_payout} (statt {n_payout_real}).
                            </ul></li>
                        </li>"###,
                        str_card_suggested = cardandpayout.veccard
                            .iter()
                            .map(ECard::to_string)
                            .join(", "),
                        str_gewinn = match cardandpayout.emistake {
                            EMistake::Min => "garantierter Mindestgewinn",
                            EMistake::SelfishMin => "Mindestgewinn, wenn jeder Spieler optimal spielt",
                        },
                        n_payout = cardandpayout.n_payout,
                        n_payout_real = mapepin_payout[epi],
                    ));
                }
                str_analysisimpr += "</li>";
                str_analysisimpr
            }).format(""))
        )
        + "</ul>"
        + "<h2>Gewinnspanne pro Karte</h2>"
        + type_inference!(&str, &player_table(
            epi_self,
            |epi| {
                // TODO? replace this by a line/area chart
                // TODO group unchanged columns
                let vecpossiblepayout = &self.mapepivecpossiblepayout[epi];
                Some(format!("<table>{}</table>",
                    format!("<tr>{}</tr>",
                        vecpossiblepayout.iter()
                            .map(|SPossiblePayout(_perminmaxstrategyn_payout, (i_stich, i_card))|
                                format!("<td>{}/{}</td>", i_stich+1, i_card+1)
                            )
                            .join("")
                    )
                        + type_inference!(&str, &SPerMinMaxStrategy::accessors().iter().rev().map(|(_emmstrategy, fn_value_for_strategy)| {
                            format!("<tr>{}</tr>",
                                vecpossiblepayout.iter()
                                    .map(|SPossiblePayout(determinebestcardresult, (_i_stich, _i_card))|
                                        format!("<td>{}</td>",
                                            fn_value_for_strategy(&determinebestcardresult
                                                .t_combined
                                                .map(|payoutstats| verify_eq!(payoutstats.min(), payoutstats.max()))),
                                        )
                                    )
                                    .join("")
                            )
                        })
                        .join("")
                        )
                ))
            },
        ))
        + "<h2>Details</h2>"
        + type_inference!(&str, &format!("{}", self.vecanalysispercard.iter()
            .map(|analysispercard| {
                // TODO simplify output (as it currently only shows results from one ahand)
                let stichseq = &analysispercard.stichseq;
                let ahand = &analysispercard.ahand;
                let str_stich_caption = stich_caption(stichseq).0;
                let mut str_per_card = format!(r"<h3>{str_stich_caption}</h3>");
                unwrap!(write!(
                    str_per_card,
                    "<table><tr>{}{}</tr></table>",
                    player_table_stichseq(epi_self, stichseq, fn_output_card),
                    player_table_ahand(
                        epi_self,
                        ahand,
                        &game.rules,
                        /*fn_border*/|card| card==analysispercard.card_played,
                        fn_output_card,
                    ),
                ));
                append_html_payout_table::<SPerMinMaxStrategyHigherKinded>(
                    &mut str_per_card,
                    &game.rules,
                    &analysispercard.ahand,
                    &analysispercard.stichseq,
                    &analysispercard.determinebestcardresult_cheating,
                    analysispercard.card_played,
                    fn_output_card,
                );
                append_html_copy_button(
                    &mut str_per_card,
                    &game.rules,
                    &analysispercard.ahand,
                    &analysispercard.stichseq,
                    str_openschafkopf_executable,
                );
                str_per_card
            }).format("")
        ))
        + "</body></html>"
    }
}

fn write_html(path: std::path::PathBuf, str_html: &str) -> Result<std::path::PathBuf, std::io::Error> {
    std::fs::File::create(path.clone())?.write_all(str_html.as_bytes())?;
    Ok(path)
}

