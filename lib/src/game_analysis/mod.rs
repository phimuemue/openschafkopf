use crate::ai::{gametree::*, *};
use crate::game::*;
use crate::primitives::*;
use crate::rules::*;
use crate::util::*;
use crate::game_analysis::determine_best_card_table::N_COLUMNS;
use itertools::Itertools;
use std::{
    io::Write,
};
use std::fmt::Write as _;

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

pub fn append_html_payout_table<HtmlAttributeOrChildCard: html_generator::AttributeOrChild, MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded>(
    str_per_card: &mut String,
    rules: &SRules,
    ahand: &EnumMap<EPlayerIndex, SHand>,
    stichseq: &SStichSequence,
    determinebestcardresult: &SDetermineBestCardResult<MinMaxStrategiesHK::Type<SPayoutStats<()>>>,
    card_played: ECard,
    fn_output_card: &dyn Fn(ECard, bool/*b_highlight*/)->HtmlAttributeOrChildCard,
)
    where
        MinMaxStrategiesHK::Type<SPayoutStats<()>>: std::fmt::Debug,
        MinMaxStrategiesHK::Type<[(String, f32); N_COLUMNS]>: PartialEq+Clone,
{
    use html_generator::*;
    *str_per_card += "<table>";
    fn condensed_cheating_columns(atplstrf: &[(String, f32); N_COLUMNS]) -> impl Iterator<Item=&(String, f32)> {
        let otplstrf_first = verify!(atplstrf.first());
        assert_eq!(
            verify_eq!(unwrap!(otplstrf_first), &atplstrf[2]).1,
            atplstrf[1].1,
        );
        otplstrf_first.into_iter()
    }
    let vecoutputline_cheating = determine_best_card_table::table::</*TODO type annotations needed?*/MinMaxStrategiesHK, _>(
        determinebestcardresult,
        rules,
        /*fn_loss_or_win*/&|n_payout, ()| n_payout.cmp(&0),
    ).into_output_lines();
    for outputline in vecoutputline_cheating.iter() {
        *str_per_card += "<tr>";
        *str_per_card += r#"<td style="padding: 5px;">"#;
        for &card in outputline.vect.iter() {
            *str_per_card += &html_display_children(fn_output_card(card, /*b_border*/card==card_played)).to_string();
        }
        *str_per_card += "</td>";
        for (_emmstrategy, atplstrf) in outputline.perminmaxstrategyatplstrf.via_accessors() {
            // TODO simplify to one item per emmstrategy
            for (str_num, _f) in condensed_cheating_columns(atplstrf) {
                *str_per_card += r#"<td style="padding: 5px;">"#;
                // TODO colored output as in suggest_card
                *str_per_card += str_num;
                *str_per_card += "</td>";
            }
        }
        *str_per_card += "</tr>";
    }
    *str_per_card += "<tr>";
    // TODO? should veccard_non_allowed be a separate row in determine_best_card_table::table?
    let hand_current_player = &ahand[unwrap!(stichseq.current_stich().current_playerindex())];
    let veccard_allowed = rules.all_allowed_cards(
        stichseq,
        hand_current_player,
    );
    let mut veccard_non_allowed = hand_current_player.cards().iter()
        .filter_map(|card| if_then_some!(!veccard_allowed.contains(card), *card))
        .collect::<Vec<_>>();
    if !veccard_non_allowed.is_empty() {
        rules.sort_cards(&mut veccard_non_allowed);
        *str_per_card += r#"<td style="padding: 5px;">"#;
        for card in veccard_non_allowed {
            *str_per_card += &html_display_children(fn_output_card(card, /*b_border*/false)).to_string();
        }
        *str_per_card += "</td>";
        unwrap!(write!(
            *str_per_card,
            r#"<td colspan="{}" style="padding: 5px;">N.A.</td>"#,
            unwrap!(vecoutputline_cheating.iter()
                .map(|outputline|
                    outputline.perminmaxstrategyatplstrf.via_accessors().into_iter().flat_map(|(_emmstrategy, atplstrf)| condensed_cheating_columns(atplstrf)).count()
                )
                .all_equal_value()),
        ));
    }
    *str_per_card += "</tr>";
    *str_per_card += "</table>";
}

pub fn append_html_copy_button(
    str_per_card: &mut String,
    rules: &SRules,
    ahand: &EnumMap<EPlayerIndex, SHand>,
    stichseq: &SStichSequence,
    str_openschafkopf_executable: &str,
) {
    *str_per_card += &format!(r###"<button onclick='
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
    );
}

