use crate::ai::{handiterators::*, suspicion::*, *};
use crate::game::*;
use crate::primitives::*;
use crate::rules::{payoutdecider::*, ruleset::VStockOrT, rulessolo::*, *};
use crate::util::*;
use crate::game_analysis::determine_best_card_table::{table, SOutputLine, SFormatInfo};
use itertools::Itertools;
use std::{
    io::Write,
    time::{Instant, Duration},
};

pub mod determine_best_card_table;

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
pub struct SAnalysisCardAndPayout {
    pub veccard: Vec<SCard>,
    pub n_payout: isize,
}

#[derive(Clone)]
pub struct SAnalysisPerCard {
    i_stich: usize,
    epi: EPlayerIndex,
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
    let game = unwrap!(SGame::new_finished(
        game_in.rules.clone(),
        game_in.doublings.clone(),
        game_in.ostossparams.clone(),
        game_in.vecstoss.clone(),
        game_in.n_stock,
        SStichSequenceGameFinished::new(&game_in.stichseq),
        /*fn_before_zugeben*/|game, i_stich, epi, card| {
            if remaining_cards_per_hand(&game.stichseq)[epi] <= n_max_remaining_cards {
                let determinebestcard = SDetermineBestCard::new_from_game(game);
                macro_rules! look_for_mistakes{($itahand: expr,) => {{
                    let determinebestcardresult = determine_best_card(
                        &determinebestcard,
                        $itahand,
                        equivalent_cards_filter(
                            /*n_until_remaining_cards, determined heuristically*/7,
                            epi,
                            game.rules.equivalent_when_on_same_hand(),
                        ),
                        &SMinReachablePayout::new_from_game(game),
                        /*fn_visualizer*/|_,_,_| SNoVisualization,
                    );
                    let (veccard, minmax) = determinebestcardresult.cards_with_maximum_value();
                    (
                        determinebestcardresult.clone(),
                        if 
                            !veccard.contains(&card) // TODO can we improve this?
                            && an_payout[epi]<minmax.t_min.min()
                        {
                            Some(SAnalysisCardAndPayout{
                                veccard,
                                n_payout: minmax.t_selfish_min.min(),
                            })
                        } else {
                            // The decisive mistake must occur in subsequent stichs.
                            // TODO assert that it actually occurs
                            None
                        }
                    )
                }}}
                let look_for_mistakes_simulating = || {
                    if !b_simulate_all_hands {
                        VImprovementSimulating::NotRequested
                    } else if let Some(cardandpayout_simulating) = look_for_mistakes!(
                        all_possible_hands(
                            &game.stichseq,
                            game.ahand[determinebestcard.epi_fixed].clone(),
                            determinebestcard.epi_fixed,
                            game.rules.as_ref(),
                        ),
                    ).1 {
                        VImprovementSimulating::Found(cardandpayout_simulating)
                    } else {
                        VImprovementSimulating::NothingFound
                    }
                };
                let (determinebestcardresult_cheating, ocardandpayout_cheating) = look_for_mistakes!(
                    std::iter::once(game.ahand.clone()),
                );
                vecanalysispercard.push(SAnalysisPerCard {
                    determinebestcardresult_cheating,
                    i_stich,
                    epi,
                    oanalysisimpr: /*TODO? if_then_some*/if let Some(analysisimpr) = ocardandpayout_cheating
                            .map(|cardandpayout_cheating| {
                                SAnalysisImprovement {
                                    cardandpayout_cheating,
                                    improvementsimulating: look_for_mistakes_simulating(),
                                }
                            })
                        {
                            Some(analysisimpr)
                        } else {
                            debug_assert!(matches!(look_for_mistakes_simulating(), VImprovementSimulating::NothingFound));
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
        ),
        n_findings_cheating: itanalysisimpr.clone().count(),
        n_findings_simulating: itanalysisimpr.clone()
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

pub fn generate_analysis_html(
    game: &crate::game::SGame,
    str_description: &str,
    str_link: &str,
    str_rules: &str,
    mapepin_payout: &EnumMap<EPlayerIndex, isize>,
    slcanalysispercard: &[SAnalysisPerCard],
) -> String {
    use crate::game::*;
    assert!(game.which_player_can_do_something().is_none()); // TODO use SGameResult (see comment in SGameResult)
    let ahand = EPlayerIndex::map_from_fn(|epi| {
        SHand::new_from_iter(game.stichseq.completed_stichs().iter().map(|stich| stich[epi]))
    });
    let epi_self = EPlayerIndex::EPI0;
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
        str_rules=str_rules,
    )
    + &crate::ai::suspicion::player_table(epi_self, |epi| {
        let mut veccard = ahand[epi].cards().to_vec();
        game.rules.sort_cards_first_trumpf_then_farbe(&mut veccard);
        Some(veccard.into_iter().map(|card|
            crate::ai::suspicion::output_card(card, /*b_border*/false)
        ).format(""))
    })
    + "<table><tr>"
    + &format!("{}", game.stichseq.completed_stichs().iter().map(|stich| {
        format!("<td>{}</td>", crate::ai::suspicion::player_table(epi_self, |epi| {
            Some(crate::ai::suspicion::output_card(stich[epi], /*b_border*/epi==stich.first_playerindex()))
        }))
    }).format("\n"))
    + "</tr></table>"
    + "<ul>"
    + &format!("{}", slcanalysispercard.iter()
        .filter_map(|analysispercard| analysispercard.oanalysisimpr.as_ref().map(|analysisimpr|
            (analysisimpr, analysispercard.i_stich, analysispercard.epi)
        ))
        .map(|(analysisimpr, i_stich, epi)| {
            let mut str_analysisimpr = format!(
                r###"<li>
                    Stich {i_stich}, Spieler {epi}:
                    Bei gegebener Kartenverteilung: {str_card_suggested_cheating} garantierter Mindestgewinn: {n_payout_cheating} (statt {n_payout_real}).
                </li>"###,
                i_stich = i_stich + 1, // humans start counting at 1
                epi = epi,
                str_card_suggested_cheating = analysisimpr.cardandpayout_cheating.veccard
                    .iter()
                    .map(SCard::to_string)
                    .join(", "),
                n_payout_cheating = analysisimpr.cardandpayout_cheating.n_payout,
                n_payout_real = mapepin_payout[epi],
            );
            if let VImprovementSimulating::Found(ref cardandpayout) = analysisimpr.improvementsimulating {
                str_analysisimpr += &format!(
                    r###"
                        <ul><li>
                        Bei unbekannter Kartenverteilung: {str_card_suggested} garantierter Mindestgewinn {n_payout} (statt {n_payout_real}).
                        </ul></li>
                    </li>"###,
                    str_card_suggested = cardandpayout.veccard
                        .iter()
                        .map(SCard::to_string)
                        .join(", "),
                    n_payout = cardandpayout.n_payout,
                    n_payout_real = mapepin_payout[epi],
                );
            }
            str_analysisimpr += "</li>";
            str_analysisimpr
        }).format(""))
    + "</ul>"
    + "<h2>Details</h2>"
    + &format!("{}", slcanalysispercard.iter()
        .map(|analysispercard| {
            let (vecoutputline, aformatinfo) = table(
                &analysispercard.determinebestcardresult_cheating,
                /*fn_human_readable_payout*/&|f_payout| f_payout,
            );
            let mut str_per_card = format!(
                r###"<h3>Stich {i_stich}, Spieler {epi}</h3>"###,
                i_stich = analysispercard.i_stich + 1, // humans start counting at 1
                epi = analysispercard.epi,
            );
            str_per_card += "<table>";
            for SOutputLine{card, atplstrf} in vecoutputline.iter() {
                str_per_card += "<tr>";
                str_per_card += "<td>";
                str_per_card += &crate::ai::suspicion::output_card(*card, /*b_border*/false);
                str_per_card += "</td>";
                for ((str_num, _f), SFormatInfo{f_min:_, f_max:_, n_width}) in atplstrf.iter().zip_eq(aformatinfo.iter()) {
                    str_per_card += "<td>";
                    // TODO colored output as in suggest_card
                    str_per_card += &format!("{:>width$}", str_num, width=n_width);
                    str_per_card += "</td>";
                }
                str_per_card += "</tr>";
            }
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

pub fn analyze_games(path_analysis: &std::path::Path, fn_link: impl Fn(&str)->String, itgamewithdesc: impl Iterator<Item=SGameWithDesc>, b_include_no_findings: bool, n_max_remaining_cards: usize, b_simulate_all_hands: bool) -> Result<(), failure::Error> {
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
    for gamewithdesc in itgamewithdesc {
        if let Ok(gameresult) = gamewithdesc.resgameresult {
            match gameresult.stockorgame {
                VStockOrT::Stock(_) => {
                    if b_include_no_findings {
                        str_index_html += &format!(
                            r#"<tr>
                                <td>
                                    Stock: {}/{}/{}/{}
                                </td>
                            </tr>"#,
                            gameresult.an_payout[EPlayerIndex::EPI0],
                            gameresult.an_payout[EPlayerIndex::EPI1],
                            gameresult.an_payout[EPlayerIndex::EPI2],
                            gameresult.an_payout[EPlayerIndex::EPI3],
                        );
                    }
                },
                VStockOrT::OrT(game) => {
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
                    if b_include_no_findings || 0 < n_findings_cheating {
                        str_index_html += &format!(
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
                        );
                    }
                },
            }
        } else {
            str_index_html += &format!("<tr><td>Fehler ({})</td></tr>", gamewithdesc.str_description);
        }
    }
    str_index_html += "</table>";
    str_index_html += "</body></html>";
    write_html(path_analysis.join(format!("{}.html", str_date)), &str_index_html)?;
    Ok(())
}
