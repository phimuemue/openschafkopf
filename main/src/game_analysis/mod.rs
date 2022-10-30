use crate::ai::{handiterators::*, gametree::*, *};
use crate::game::*;
use crate::primitives::*;
use crate::rules::{payoutdecider::*, ruleset::VStockOrT, rulessolo::*, *};
use crate::util::*;
use crate::game_analysis::determine_best_card_table::table;
use itertools::Itertools;
use std::{
    io::Write,
    time::{Instant, Duration},
};
use std::fmt::Write as _;

pub mod determine_best_card_table;
pub mod parser;

pub trait TPayoutDeciderSoloLikeDefault : TPayoutDeciderSoloLike {
    fn default_payoutdecider(n_payout_base: isize, n_payout_schneider_schwarz: isize, laufendeparams: SLaufendeParams) -> Self;
}
impl TPayoutDeciderSoloLikeDefault for SPayoutDeciderPointBased<VGameAnnouncementPrioritySoloLike> {
    fn default_payoutdecider(n_payout_base: isize, n_payout_schneider_schwarz: isize, laufendeparams: SLaufendeParams) -> Self {
        Self::new(
            SPayoutDeciderParams::new(n_payout_base, n_payout_schneider_schwarz, laufendeparams),
            VGameAnnouncementPrioritySoloLike::SoloSimple(0),
        )
    }
}
impl TPayoutDeciderSoloLikeDefault for SPayoutDeciderTout {
    fn default_payoutdecider(n_payout_base: isize, n_payout_schneider_schwarz: isize, laufendeparams: SLaufendeParams) -> Self {
        Self::new(
            SPayoutDeciderParams::new(n_payout_base, n_payout_schneider_schwarz, laufendeparams),
            0,
        )
    }
}

#[cfg(test)]
pub fn make_stich_vector(vectplepiacard_stich: &[(EPlayerIndex, [SCard; 4])]) -> Vec<SStich> {
    vectplepiacard_stich.iter()
        .map(|&(epi, acard)| {
            SStich::new_full(epi, acard)
        })
        .collect()
}

#[derive(Clone, Debug)]
pub enum EMistake {
    Min,
    SelfishMin,
}

#[derive(Clone, Debug)]
pub struct SAnalysisCardAndPayout {
    pub veccard: Vec<SCard>,
    pub n_payout: isize,
    pub emistake: EMistake,
}

#[derive(Clone)]
pub struct SAnalysisPerCard {
    stichseq: SStichSequence, // TODO this is space-inefficient
    ahand: EnumMap<EPlayerIndex, SHand>, // TODO this is space-inefficient
    card_played: SCard,
    determinebestcardresult_cheating: SDetermineBestCardResult<SPayoutStatsPerStrategy>,
    oanalysisimpr: Option<SAnalysisImprovement>,
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
    pub str_html: String,
    pub n_findings_cheating: usize,
    pub n_findings_simulating: usize,
    pub duration: Duration,
}

struct SPossiblePayout(
    EnumMap<EMinMaxStrategy, isize>,
    (/*i_stich*/usize, /*i_card*/usize),
);

pub fn analyze_game(
    str_description: &str,
    str_link: &str,
    game_in: SGame,
    n_max_remaining_cards: usize,
    b_simulate_all_hands: bool,
) -> SGameAnalysis {
    let instant_begin = Instant::now();
    let mut vecanalysispercard = Vec::new();
    let an_payout = unwrap!(game_in.clone().finish()).an_payout;
    let str_rules = format!("{}{}",
        game_in.rules,
        if let Some(epi) = game_in.rules.playerindex() {
            format!(" von {}", epi)
        } else {
            "".to_owned()
        },
    );
    let mut mapepivecpossiblepayout = EPlayerIndex::map_from_fn(|_epi| Vec::new());
    let game = unwrap!(SGame::new_finished(
        game_in.rules.clone(),
        game_in.doublings.clone(),
        game_in.ostossparams.clone(),
        game_in.vecstoss.clone(),
        game_in.n_stock,
        SStichSequenceGameFinished::new(&game_in.stichseq),
        /*fn_before_zugeben*/|game, i_stich, epi_zugeben, card_played| {
            if game.stichseq.remaining_cards_per_hand()[epi_zugeben] <= n_max_remaining_cards {
                for epi in EPlayerIndex::values() {
                    mapepivecpossiblepayout[epi].push(SPossiblePayout(
                        explore_snapshots(
                            &mut game.ahand.clone(),
                            game.rules.as_ref(),
                            &mut game.stichseq.clone(),
                            &|_,_| equivalent_cards_filter(
                                /*n_until_stichseq_len*/7,
                                /*cardspartition*/game.rules.equivalent_when_on_same_hand(),
                            )(&game.stichseq, &game.ahand),
                            &SMinReachablePayout::new(
                                game.rules.as_ref(),
                                epi,
                                game.expensifiers(),
                            ),
                            &SSnapshotCacheNone::factory(), // TODO possibly use cache
                            &mut SNoVisualization,
                        )
                            .0
                            .map(|mapepin_payout| mapepin_payout[epi]),
                        (i_stich, game.stichseq.current_stich().size())
                    ));
                }
                let stichseq = &game.stichseq;
                macro_rules! look_for_mistakes{($itahand: expr$(,)?) => {{
                    let determinebestcardresult = unwrap!(determine_best_card(
                        stichseq,
                        game.rules.as_ref(),
                        Box::new($itahand) as Box<_>,
                        equivalent_cards_filter(
                            /*n_until_stichseq_len, determined heuristically*/7,
                            game.rules.equivalent_when_on_same_hand(),
                        ),
                        &SMinReachablePayout::new_from_game(game),
                        /*fn_snapshotcache*/SSnapshotCacheNone::factory(), // TODO possibly use cache
                        /*fn_visualizer*/SNoVisualization::factory(),
                        /*fn_inspect*/&|_b_before, _i_ahand, _ahand, _card| {},
                    ));
                    let mut ocardandpayout = None;
                    for (eminmaxstrategy, emistake) in [
                        (EMinMaxStrategy::Min, EMistake::Min),
                        (EMinMaxStrategy::SelfishMin, EMistake::SelfishMin),
                    ].into_iter()
                    {
                        assert!(ocardandpayout.is_none());
                        let (veccard, minmax) = determinebestcardresult.cards_with_maximum_value(
                            |minmax_lhs, minmax_rhs| minmax_lhs.0[eminmaxstrategy].min().cmp(&minmax_rhs.0[eminmaxstrategy].min())
                        );
                        if 
                            an_payout[epi_zugeben]<minmax.0[eminmaxstrategy].min()
                            && !veccard.contains(&card_played) // TODO can we improve this?
                        {
                            ocardandpayout = Some(SAnalysisCardAndPayout{
                                veccard,
                                n_payout: minmax.0[eminmaxstrategy].min(),
                                emistake,
                            });
                            break;
                        } else {
                            // The decisive mistake must occur in subsequent stichs.
                            // TODO assert that it actually occurs
                        }
                    }
                    (determinebestcardresult.clone(), ocardandpayout)
                }}}
                let look_for_mistakes_simulating = || {
                    let epi_current = unwrap!(game.stichseq.current_stich().current_playerindex());
                    look_for_mistakes!(all_possible_hands(
                        &game.stichseq,
                        game.ahand[epi_current].clone(),
                        epi_current,
                        game.rules.as_ref(),
                    )).1
                };
                let (determinebestcardresult_cheating, ocardandpayout_cheating) = look_for_mistakes!(
                    std::iter::once(game.ahand.clone()),
                );
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
        },
    ));
    let itanalysisimpr = vecanalysispercard.iter()
        .filter_map(|analysispercard| analysispercard.oanalysisimpr.as_ref());
    SGameAnalysis {
        str_html: generate_analysis_html(
            &game,
            str_description,
            str_link,
            &str_rules,
            &unwrap!(game.clone().finish()).an_payout,
            &vecanalysispercard,
            &mapepivecpossiblepayout,
        ),
        n_findings_cheating: itanalysisimpr.clone().count(), // TODO distinguish emistake
        n_findings_simulating: itanalysisimpr.clone() // TODO distinguish emistake
            .filter(|analysisimpr| matches!(analysisimpr.improvementsimulating, VImprovementSimulating::Found(_)))
            .count(),
        duration: instant_begin.elapsed(),
    }
}

pub fn generate_html_auxiliary_files(path_out_dir: &std::path::Path) -> Result<(), failure::Error> {
    macro_rules! write_auxiliary_file(($str_filename: expr) => {
        std::fs::File::create(
            path_out_dir
                .join($str_filename)
        )?
            .write_all(
                include_bytes!(
                    concat!(env!("OUT_DIR"), "/", $str_filename) // https://doc.rust-lang.org/cargo/reference/build-scripts.html#case-study-code-generation
                )
            )
    });
    write_auxiliary_file!("cards.png")?;
    write_auxiliary_file!("css.css")?;
    Ok(())
}

fn generate_analysis_html(
    game: &crate::game::SGame,
    str_description: &str,
    str_link: &str,
    str_rules: &str,
    mapepin_payout: &EnumMap<EPlayerIndex, isize>,
    slcanalysispercard: &[SAnalysisPerCard],
    mapepivecpossiblepayout: &EnumMap<EPlayerIndex, Vec<SPossiblePayout>>,
) -> String {
    use crate::game::*;
    assert!(game.which_player_can_do_something().is_none()); // TODO use SGameResult (see comment in SGameResult)
    let ahand = EPlayerIndex::map_from_fn(|epi| {
        SHand::new_from_iter(game.stichseq.completed_stichs().iter().map(|stich| stich[epi]))
    });
    let epi_self = EPlayerIndex::EPI0;
    let stich_caption = |stichseq: &SStichSequence| {
        format!("Stich {}, Spieler {}",
            stichseq.completed_stichs().len() + 1, // humans count 1-based
            unwrap!(stichseq.current_stich().current_playerindex()),
        )
    };
    format!(
        r###"<!DOCTYPE html>
        <html lang="de" class="no-js">
            <head>
                <title>Schafkopf-Analyse: {str_description}</title>
                <meta http-equiv="Content-Type" content="text/html; charset=UTF-8">
                <link rel="stylesheet" type="text/css" href="../css.css">
                <script>
                    function copyToClipboard(str, btn) {{
                        navigator.clipboard.writeText(str).then(
                            function() {{
                                btn.innerHTML = "Copied (" + (new Date()).toLocaleString() + ")";
                            }},
                            function() {{
                                // indicate fail by doing nothing
                            }},
                        );
                    }}
                </script>
            </head>
            <body>
                <h1>Schafkopf-Analyse: <a href="{str_link}">{str_description}</a></h1>
                <h2>{str_rules}</h2>"###,
        str_description=str_description,
        str_link=str_link,
        str_rules=str_rules,
    )
    + "<table><tr>"
    + &crate::ai::gametree::player_table_ahand(epi_self, &ahand, game.rules.as_ref(), /*fn_border*/|_card| false)
    + "</tr></table><table><tr>"
    + &crate::ai::gametree::player_table_stichseq(epi_self, &game.stichseq)
    + "</tr></table>"
    + "<ul>"
    + &format!("{}", slcanalysispercard.iter()
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
                str_stich_caption=stich_caption(stichseq),
                str_card_suggested_cheating = analysisimpr.cardandpayout_cheating.veccard
                    .iter()
                    .map(SCard::to_string)
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
                        .map(SCard::to_string)
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
    + "</ul>"
    + "<h2>Gewinnspanne pro Karte</h2>"
    + &crate::ai::gametree::player_table(
        epi_self,
        |epi| {
            // TODO? replace this by a line/area chart
            // TODO group unchanged columns
            let vecpossiblepayout = &mapepivecpossiblepayout[epi];
            Some(format!("<table>{}</table>",
                format!("<tr>{}</tr>",
                    vecpossiblepayout.iter()
                        .map(|SPossiblePayout(_, (i_stich, i_card))|
                            format!("<td>{}/{}</td>", i_stich+1, i_card+1)
                        )
                        .join("")
                )
                    + &EMinMaxStrategy::values().rev().map(|emmstrategy| {
                        format!("<tr>{}</tr>",
                            vecpossiblepayout.iter()
                                .map(|possiblepayout|
                                    format!("<td>{}</td>",
                                        possiblepayout.0[emmstrategy]
                                    )
                                )
                                .join("")
                        )
                    })
                    .join("")
            ))
        },
    )
    + "<h2>Details</h2>"
    + &format!("{}", slcanalysispercard.iter()
        .map(|analysispercard| {
            let vecoutputline = table(
                &analysispercard.determinebestcardresult_cheating,
                game.rules.as_ref(),
                /*fn_human_readable_payout*/&|f_payout| f_payout,
            ).vecoutputline;
            // TODO simplify output (as it currently only shows results from one ahand)
            let stichseq = &analysispercard.stichseq;
            let ahand = &analysispercard.ahand;
            let epi_current = unwrap!(stichseq.current_stich().current_playerindex());
            let mut str_per_card = format!(r###"<h3>{} <button onclick='copyToClipboard("{}", this)'>&#128203</button></h3>"###,
                stich_caption(stichseq),
                format!("{str_exe} suggest-card --rules \"{str_rules}\" --cards-on-table \"{str_cards_on_table}\" --hand \"{str_hand}\" --branching \"equiv7\"",
                    // TODO error handling
                    str_exe=unwrap!(unwrap!(unwrap!(std::env::current_exe()).canonicalize()).to_str()),
                    str_cards_on_table=stichseq.visible_stichs().iter()
                        .filter_map(|stich| if_then_some!(!stich.is_empty(), stich.iter().map(|(_epi, card)| *card).join(" ")))
                        .join("  "),
                    str_hand=display_card_slices(ahand, &game.rules, "  "),
                ).replace('\"', "\\\""),
            );
            unwrap!(write!(
                str_per_card,
                "<table><tr>{}{}</tr></table>",
                player_table_stichseq(epi_self, stichseq),
                player_table_ahand(
                    epi_self,
                    ahand,
                    game.rules.as_ref(),
                    /*fn_border*/|card| card==analysispercard.card_played,
                ),
            ));
            str_per_card += "<table>";
            for outputline in vecoutputline.iter() {
                str_per_card += "<tr>";
                str_per_card += "<td>";
                for &card in outputline.vect.iter() {
                    str_per_card += &crate::ai::gametree::output_card(card, /*b_border*/card==analysispercard.card_played);
                }
                str_per_card += "</td>";
                for atplstrf in outputline.mapemmstrategyatplstrf.iter() {
                    // TODO simplify to one item per emmstrategy
                    for (str_num, _f) in atplstrf.iter() {
                        str_per_card += "<td>";
                        // TODO colored output as in suggest_card
                        str_per_card += str_num;
                        str_per_card += "</td>";
                    }
                }
                str_per_card += "</tr>";
            }
            str_per_card += "<tr>";
            // TODO? should veccard_non_allowed be a separate row in determine_best_card_table::table?
            let veccard_allowed = game.rules.all_allowed_cards(
                stichseq,
                &ahand[epi_current],
            );
            let mut veccard_non_allowed = ahand[epi_current].cards().iter()
                .filter_map(|card| if_then_some!(!veccard_allowed.contains(card), *card))
                .collect::<Vec<_>>();
            game.rules.sort_cards_first_trumpf_then_farbe(&mut veccard_non_allowed);
            if !veccard_non_allowed.is_empty() {
                str_per_card += "<td>";
                for card in veccard_non_allowed {
                    str_per_card += &crate::ai::gametree::output_card(card, /*b_border*/false);
                }
                str_per_card += "</td>";
                unwrap!(write!(
                    str_per_card,
                    "<td colspan=\"{}\">N.A.</td>",
                    verify_eq!(
                        determine_best_card_table::N_COLUMNS * EMinMaxStrategy::SIZE,
                        unwrap!(vecoutputline.iter()
                            .map(|outputline|
                                outputline.mapemmstrategyatplstrf.iter().flatten().count()
                            )
                            .all_equal_item())
                    ),
                ));
            }
            str_per_card += "</tr>";
            str_per_card += "</table>";
            str_per_card
        }).format("")
    )
    + "</body></html>"
}

fn create_dir_if_not_existent(path: &std::path::Path) -> Result<(), failure::Error> {
    if !path.exists() {
        std::fs::create_dir(path).map_err(|err| format_err!("{:?}", err))
    } else {
        Ok(())
    }
}

fn write_html(path: std::path::PathBuf, str_html: &str) -> Result<std::path::PathBuf, failure::Error> {
    std::fs::File::create(path.clone())?.write_all(str_html.as_bytes())?;
    Ok(path)
}

pub struct SGameWithDesc {
    pub str_description: String,
    pub str_link: String,
    pub resgameresult: Result<SGameResult, failure::Error>,
}

pub fn analyze_games(path_analysis: &std::path::Path, fn_link: impl Fn(&str)->String, vecgamewithdesc: Vec<SGameWithDesc>, b_include_no_findings: bool, n_max_remaining_cards: usize, b_simulate_all_hands: bool) -> Result<std::path::PathBuf, failure::Error> {
    create_dir_if_not_existent(path_analysis)?;
    generate_html_auxiliary_files(path_analysis)?;
    let str_date = format!("{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
    let mut str_index_html = format!(
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
        str_date = str_date,
    );
    str_index_html += "<table>";
    let n_games_total = vecgamewithdesc.len();
    let mut n_games_done = 0;
    let mut n_games_non_stock = 0;
    let mut n_games_findings = 0;
    for gamewithdesc in vecgamewithdesc {
        if let Ok(gameresult) = gamewithdesc.resgameresult {
            match gameresult.stockorgame {
                VStockOrT::Stock(_) => {
                    if b_include_no_findings {
                        unwrap!(write!(
                            str_index_html,
                            r#"<tr>
                                <td>
                                    Stock: {}/{}/{}/{}
                                </td>
                            </tr>"#,
                            gameresult.an_payout[EPlayerIndex::EPI0],
                            gameresult.an_payout[EPlayerIndex::EPI1],
                            gameresult.an_payout[EPlayerIndex::EPI2],
                            gameresult.an_payout[EPlayerIndex::EPI3],
                        ));
                    }
                },
                VStockOrT::OrT(game) => {
                    n_games_non_stock += 1;
                    let str_rules = format!("{}", game.rules);
                    let path_analysis_game = path_analysis.join(gamewithdesc.str_description.replace('/', "_").replace('.', "_"));
                    create_dir_if_not_existent(&path_analysis_game)?;
                    let path = path_analysis_game.join("analysis.html");
                    let gameanalysis = analyze_game(
                        &gamewithdesc.str_description,
                        &fn_link(&gamewithdesc.str_description),
                        game,
                        n_max_remaining_cards,
                        b_simulate_all_hands,
                    );
                    let path = write_html(path, &gameanalysis.str_html)?;
                    let n_findings_simulating = gameanalysis.n_findings_simulating;
                    let n_findings_cheating = gameanalysis.n_findings_cheating;
                    assert!(n_findings_simulating <= n_findings_cheating);
                    if 0<n_findings_cheating {
                        n_games_findings += 1;
                    }
                    if b_include_no_findings || 0 < n_findings_cheating {
                        unwrap!(write!(
                            str_index_html,
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
                                let n_secs = gameanalysis.duration.as_secs();
                                if 0==n_secs {
                                    "&lt;1s".to_owned()
                                } else {
                                    format!("{}s", n_secs)
                                }
                            },
                        ));
                    }
                },
            }
        } else {
            unwrap!(write!(str_index_html, "<tr><td>Fehler ({})</td></tr>", gamewithdesc.str_description));
        }
        n_games_done += 1;
        println!("Total: {n_games_total}. Done: {n_games_done}. Non-stock: {n_games_non_stock}. With findings: {n_games_findings}");
    }
    str_index_html += "</table>";
    str_index_html += "</body></html>";
    write_html(path_analysis.join(format!("{}.html", str_date)), &str_index_html)
}
