use crate::ai::{handiterators::*, suspicion::*, *};
use crate::game::*;
use crate::primitives::*;
use crate::rules::{payoutdecider::*, ruleset::*, rulessolo::*, *};
use crate::util::*;
use itertools::Itertools;
use std::{
    io::Write,
    time::{Instant, Duration},
};

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
pub fn make_stich_vector(vecpairepiacard_stich: &[(EPlayerIndex, [SCard; 4])]) -> Vec<SStich> {
    vecpairepiacard_stich.iter()
        .map(|&(epi, acard)| {
            SStich::new_full(epi, acard)
        })
        .collect()
}

pub fn analyze_game_internal(
    analyzeparams: SAnalyzeParams,
    mut fn_before_zugeben: impl FnMut(&SGame, /*i_stich*/usize, EPlayerIndex, SCard),
) -> SGame { // TODO return SGameResult
    let doublings = SDoublings::new_full(
        SStaticEPI0{},
        EPlayerIndex::map_from_fn(|epi| 
            analyzeparams.vecn_doubling.contains(&epi.to_usize())
        ).into_raw()
    );
    let mut game = SGame::new(
        analyzeparams.ahand,
        doublings,
        Some(SStossParams::new(
            /*n_stoss_max*/4,
        )),
        analyzeparams.rules.box_clone(),
        analyzeparams.n_stock,
    );
    for n_epi_stoss in analyzeparams.vecn_stoss.iter() {
        debug_verify!(game.stoss(/*TODO could this be EPlayerIndex?*/debug_verify!(EPlayerIndex::checked_from_usize(*n_epi_stoss)).unwrap())).unwrap();
    }
    for (i_stich, stich) in analyzeparams.vecstich.iter().enumerate() {
        assert_eq!(Some(stich.first_playerindex()), game.which_player_can_do_something().map(|gameaction| gameaction.0));
        for (epi, card) in stich.iter() {
            assert_eq!(Some(epi), game.which_player_can_do_something().map(|gameaction| gameaction.0));
            fn_before_zugeben(&game, i_stich, epi, *card);
            debug_verify!(game.zugeben(*card, epi)).unwrap();
        }
    }
    for (i_stich, stich) in game.stichseq.visible_stichs().iter().enumerate() {
        assert_eq!(stich, &analyzeparams.vecstich[i_stich]);
    }
    game
}

#[derive(Clone)]
pub struct SAnalysisCardAndPayout {
    pub veccard: Vec<SCard>,
    pub n_payout: isize,
}

#[derive(Clone)]
pub struct SAnalysisImprovement {
    pub i_stich: usize,
    pub epi: EPlayerIndex,
    pub cardandpayout_cheating: SAnalysisCardAndPayout,
    pub ocardandpayout_simulating: Option<SAnalysisCardAndPayout>,
}

#[derive(Debug, Clone)]
pub struct SAnalyzeParams {
    pub rules: Box<dyn TRules>,
    pub ahand: EnumMap<EPlayerIndex, SHand>,
    pub vecn_doubling: Vec<usize>,
    pub vecn_stoss: Vec<usize>,
    pub n_stock: isize,
    pub vecstich: Vec<SStich>,
}

pub struct SGameAnalysis {
    pub str_html: String,
    pub n_findings_cheating: usize,
    pub n_findings_simulating: usize,
    pub duration: Duration,
}

pub fn analyze_game(str_description: &str, str_link: &str, analyzeparams: SAnalyzeParams) -> SGameAnalysis {
    let instant_begin = Instant::now();
    let mut vecanalysisimpr = Vec::new();
    let an_payout = debug_verify!(analyze_game_internal(
        analyzeparams.clone(),
        /*fn_before_zugeben*/|_game, _i_stich, _epi, _card| {}
    ).finish()).unwrap().an_payout;
    let str_rules = format!("{}{}",
        analyzeparams.rules,
        if let Some(epi) = analyzeparams.rules.playerindex() {
            format!(" von {}", epi)
        } else {
            "".to_owned()
        },
    );
    let game = analyze_game_internal(
        analyzeparams,
        /*fn_before_zugeben*/|game, i_stich, epi, card| {
            if remaining_cards_per_hand(&game.stichseq)[epi] <= if_dbg_else!({2}{4}) {
                let determinebestcard = SDetermineBestCard::new_from_game(game);
                macro_rules! look_for_mistakes{($itahand: expr,) => {{
                    if determinebestcard.single_allowed_card().is_none() { // there is an actual choice
                        let determinebestcardresult = determine_best_card(
                            &determinebestcard,
                            $itahand,
                            &|_,_| (/*no filtering*/),
                            &SMinReachablePayout::new_from_game(game),
                            /*ostr_file_out*/None,
                        );
                        let (veccard, minmax) = determinebestcardresult.best_card(|minmax| minmax.values_for(determinebestcard.epi_fixed)[EMinMaxStrategy::OthersMin]);
                        if 
                            !veccard.contains(&card) // TODO can we improve this?
                            && an_payout[epi]<minmax.aan_payout[EMinMaxStrategy::OthersMin][epi]
                        {
                            Some(SAnalysisCardAndPayout{
                                veccard,
                                n_payout: minmax.aan_payout[EMinMaxStrategy::MaxPerEpi][epi]
                            })
                        } else {
                            // The decisive mistake must occur in subsequent stichs.
                            // TODO assert that it actually occurs
                            None
                        }
                    } else {
                        None
                    }
                }}};
                let look_for_mistakes_simulating = || {
                    look_for_mistakes!(
                        all_possible_hands(
                            &game.stichseq,
                            game.ahand[determinebestcard.epi_fixed].clone(),
                            determinebestcard.epi_fixed,
                            game.rules.as_ref(),
                        ),
                    )
                };
                if let Some(analysisimpr) = look_for_mistakes!(
                    std::iter::once(game.ahand.clone()),
                )
                    .map(|cardandpayout_cheating| {
                        SAnalysisImprovement {
                            i_stich,
                            epi,
                            cardandpayout_cheating,
                            ocardandpayout_simulating: look_for_mistakes_simulating(),
                        }
                    })
                {
                    vecanalysisimpr.push(analysisimpr);
                } else {
                    debug_assert!(look_for_mistakes_simulating().is_none());
                }
            }
        },
    );
    SGameAnalysis {
        str_html: generate_analysis_html(
            &game,
            str_description,
            str_link,
            &str_rules,
            &debug_verify!(game.clone().finish()).unwrap().an_payout,
            &vecanalysisimpr,
        ),
        n_findings_cheating: vecanalysisimpr.len(),
        n_findings_simulating: vecanalysisimpr.iter()
            .filter(|analysisimpr| analysisimpr.ocardandpayout_simulating.is_some())
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
    slcanalysisimpr: &[SAnalysisImprovement],
) -> String {
    use crate::game::*;
    assert!(game.which_player_can_do_something().is_none()); // TODO use SGameResult (see comment in SGameResult)
    let ahand = EPlayerIndex::map_from_fn(|epi| {
        SHand::new_from_vec(game.stichseq.completed_stichs().iter().map(|stich| stich[epi]).collect())
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
    + &format!("{}", slcanalysisimpr.iter().map(|analysisimpr| {
        let mut str_analysisimpr = format!(
            r###"<li>
                Stich {i_stich}, Spieler {epi}:
                Bei gegebener Kartenverteilung: {str_card_suggested_cheating} garantierter Mindestgewinn: {n_payout_cheating} (statt {n_payout_real}).
            </li>"###,
            i_stich = analysisimpr.i_stich + 1, // humans start counting at 1
            epi = analysisimpr.epi,
            str_card_suggested_cheating = analysisimpr.cardandpayout_cheating.veccard
                .iter()
                .map(SCard::to_string)
                .join(", "),
            n_payout_cheating = analysisimpr.cardandpayout_cheating.n_payout,
            n_payout_real = mapepin_payout[analysisimpr.epi],
        );
        if let Some(ref cardandpayout) = analysisimpr.ocardandpayout_simulating {
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
                n_payout_real = mapepin_payout[analysisimpr.epi],
            );
        }
        str_analysisimpr += "</li>";
        str_analysisimpr
    }).format(""))
    + "</ul>"
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

pub struct SAnalyzeParamsWithDesc {
    pub str_description: String,
    pub str_link: String,
    pub resanalyzeparams: Result<SAnalyzeParams, failure::Error>,
}

pub fn analyze_games(path_analysis: &std::path::Path, fn_link: impl Fn(&str)->String, itanalyzeparamswithdesc: impl Iterator<Item=SAnalyzeParamsWithDesc>) -> Result<(), failure::Error> {
    create_dir_if_not_existent(&path_analysis)?;
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
    for analyzeparamswithdesc in itanalyzeparamswithdesc {
        if let Ok(analyzeparams) = analyzeparamswithdesc.resanalyzeparams {
            let str_rules = format!("{}", analyzeparams.rules);
            let path_analysis_game = path_analysis.join(analyzeparamswithdesc.str_description.replace("/", "_").replace(".", "_"));
            create_dir_if_not_existent(&path_analysis_game)?;
            let path = path_analysis_game.join("analysis.html");
            let gameanalysis = analyze_game(&analyzeparamswithdesc.str_description, &fn_link(&analyzeparamswithdesc.str_description), analyzeparams);
            let path = write_html(path, &gameanalysis.str_html)?;
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
                str_path = debug_verify!(
                    debug_verify!(path.strip_prefix(path_analysis)).unwrap().to_str()
                ).unwrap(),
                str_rules = str_rules,
                n_findings_simulating = gameanalysis.n_findings_simulating,
                n_findings_cheating = gameanalysis.n_findings_cheating,
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
        } else {
            str_index_html += &format!("<tr><td>Fehler ({})</td></tr>", analyzeparamswithdesc.str_description);
        }
    }
    str_index_html += "</table>";
    str_index_html += "</body></html>";
    write_html(path_analysis.join(format!("{}.html", str_date)), &str_index_html)?;
    Ok(())
}
