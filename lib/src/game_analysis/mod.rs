use crate::ai::{gametree::*, *};
use crate::game::*;
use crate::primitives::*;
use crate::rules::*;
use crate::util::*;
use crate::game_analysis::determine_best_card_table::{SOutputLine, N_COLUMNS};
use itertools::Itertools;
use std::{
    io::Write,
};

pub mod determine_best_card_table;
pub mod parser;

pub fn rules_to_string(rules: &SRules) -> String {
    format!("{}{}", // TODO unify rule formatting
        rules,
        if let Some(epi) = rules.playerindex() {
            format!(" von {epi}")
        } else {
            "".to_owned()
        },
    )
}


pub fn generate_html_auxiliary_files(path_out_dir: &std::path::Path) -> Result<(), std::io::Error> {
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

pub fn html_payout_table<'a, 'b, HtmlAttributeOrChildCard: html_generator::AttributeOrChild, TplStrategies:TTplStrategies+Clone>(
    rules: &'b SRules,
    ahand: &'b EnumMap<EPlayerIndex, SHand>,
    stichseq: &'b SStichSequence,
    determinebestcardresult: &'b SDetermineBestCardResult<SPerMinMaxStrategyGeneric<SPayoutStats<()>, TplStrategies>>,
    card_played: ECard,
    fn_output_card: &'a dyn Fn(ECard, bool/*b_highlight*/)->HtmlAttributeOrChildCard,
) -> impl html_generator::AttributeOrChild<Attribute=()> + use<'a, HtmlAttributeOrChildCard, TplStrategies> {
    use html_generator::*;
    fn condensed_cheating_columns(atplstrf: &[(String, f32); N_COLUMNS]) -> impl Iterator<Item=&(String, f32)> + Clone {
        let otplstrf_first = verify!(atplstrf.first());
        assert_eq!(
            verify_eq!(unwrap!(otplstrf_first), &atplstrf[2]).1,
            atplstrf[1].1,
        );
        otplstrf_first.into_iter()
    }
    let vecoutputline_cheating = determine_best_card_table::table::</*TODO type annotations needed?*/TplStrategies, _>(
        determinebestcardresult,
        rules,
        /*fn_loss_or_win*/&|n_payout, ()| n_payout.cmp(&0),
    ).into_output_lines();
    // TODO? should veccard_non_allowed be a separate row in determine_best_card_table::table?
    let hand_current_player = &ahand[unwrap!(stichseq.current_stich().current_playerindex())];
    let veccard_allowed = rules.all_allowed_cards(
        stichseq,
        hand_current_player,
    );
    let mut veccard_non_allowed = hand_current_player.cards().iter()
        .filter_map(|card| if_then_some!(!veccard_allowed.contains(card), *card))
        .collect::<Vec<_>>();
    let otr_non_allowed = if_then_some!(!veccard_non_allowed.is_empty(), tr({
        rules.sort_cards(&mut veccard_non_allowed);
        (
            td((
                attributes::style("padding: 5px;"),
                html_iter(veccard_non_allowed.into_iter().map(|card|
                    fn_output_card(card, /*b_border*/false) // TODO This should possibly assert that card!=card_played
                )),
            )),
            td((
                colspan(format!("{}",
                    unwrap!(vecoutputline_cheating.iter()
                        .map(|outputline|
                            outputline.perminmaxstrategyatplstrf.via_accessors().into_iter().flat_map(|(_emmstrategy, atplstrf)| condensed_cheating_columns(atplstrf)).count()
                        )
                        .all_equal_value()),
                )),
                attributes::style("padding: 5px;"),
                "N.A.",
            )),
        )
    }));
    table((
        html_iter(vecoutputline_cheating.into_iter().map(move |SOutputLine{vect, perminmaxstrategyatplstrf}| {
            tr((
                td((
                    attributes::style("padding: 5px;"),
                    html_iter(vect.into_iter().map(move |card| 
                        fn_output_card(card, /*b_border*/card==card_played)
                    )),
                )),
                {
                    html_iter(perminmaxstrategyatplstrf.via_accessors().into_iter()
                        .map(|(_emmstrategy, atplstrf)| atplstrf.to_owned())
                        .collect::<Vec<_>>() // TODO required?
                        .into_iter()
                        .flat_map(|atplstrf| {
                            // TODO simplify to one item per emmstrategy
                            condensed_cheating_columns(&atplstrf)
                                .map(ToOwned::to_owned).collect::<Vec<_>>() // TODO avoid
                        })
                        .map(|(str_num, _f)|
                            td((attributes::style("padding: 5px;"), str_num)) // TODO colored output as in suggest_card
                        ),
                    )
                }
            ))
        })),
        otr_non_allowed,
    ))
}

pub fn html_copy_button(
    rules: &SRules,
    ahand: &EnumMap<EPlayerIndex, SHand>,
    stichseq: &SStichSequence,
    str_openschafkopf_executable: &str,
) -> String { // TODO use html_generator
    format!(r###"<button onclick='
        (function /*copyToClipboard*/(str, btn) {{
            navigator.clipboard.writeText(str).then(
                function() {{
                    btn.innerHTML = "Copied (" + (new Date()).toLocaleString() + ")";
                }},
                function() {{
                    // indicate fail by doing nothing
                }},
            );
        }})("{}", this)
        '>&#128203</button>"###,
        format!("{str_openschafkopf_executable} suggest-card --rules \"{str_rules}\" --cards-on-table \"{str_cards_on_table}\" --hand \"{str_hand_all}\" --hand \"{str_hand_single}\"",
            str_rules=rules_to_string(rules),
            str_cards_on_table=stichseq.visible_stichs().iter()
                .filter_map(|stich| if_then_some!(!stich.is_empty(), stich.iter().map(|(_epi, card)| *card).join(" ")))
                .join("  "),
            str_hand_all=display_card_slices(ahand, rules, "  "),
            str_hand_single=SDisplayCardSlice::new(ahand[unwrap!(stichseq.current_stich().current_playerindex())].cards().to_vec(), rules),
        ).replace('\"', "\\\""),
    )
}

