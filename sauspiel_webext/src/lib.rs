#![allow(
    clippy::blocks_in_conditions,
)]
mod utils;

use wasm_bindgen::prelude::*;
use openschafkopf_util::*;
use openschafkopf_lib::{
    ai::{SPayoutStats, determine_best_card, gametree::{SMaxSelfishMinStrategy, SAlphaBetaPrunerNone, SGenericMinReachablePayout, SNoVisualization, SMaxSelfishMinStrategyHigherKinded}, stichoracle::SFilterByOracle},
    game::{first_hand_for, SGameResultGeneric},
    game_analysis::{html_payout_table, html_copy_button, parser::{SGameAnnouncementAnonymous, internal_analyze_sauspiel_html, TSauspielHtmlDocument, TSauspielHtmlNode, VSauspielHtmlData}},
    rules::{TRules, ruleset::VStockOrT, SExpensifiers, trumpfdecider::STrumpfDecider, VTrumpfOrFarbe, card_points::points_stich},
    primitives::{ECard, EFarbe, ESchlag, EPlayerIndex, SHand, SStichSequence, TCardSorter},
};
use crate::utils::*;
use std::fmt::Debug;
use std::cmp::Ordering;
use plain_enum::*;
use itertools::EitherOrBoth;

#[cfg(feature="sauspiel_webext_use_json")]
use openschafkopf_lib::{
    game_analysis::parser::analyze_sauspiel_json,
    ai::gametree::{player_table_ahand, player_table_stichseq},
};
use itertools::Itertools;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

macro_rules! dbg_alert{($expr:expr) => {
    if_dbg_else!({alert($expr)} {web_sys::console::log_1(&$expr.into())});
}}

#[derive(Debug)]
struct SWebsysDocument(web_sys::Document);
#[derive(Debug)]
struct SWebsysElement(web_sys::Element);
#[derive(Debug)]
struct SHtmlCollectionIterator{
    i: u32, // index type for HtmlCollection::item
    htmlcol: web_sys::HtmlCollection,
}

impl SHtmlCollectionIterator {
    fn new(htmlcol: web_sys::HtmlCollection) -> Self {
        Self {
            i: 0,
            htmlcol,
        }
    }
}

impl Iterator for SHtmlCollectionIterator {
    type Item = SWebsysElement;
    fn next(&mut self) -> Option<Self::Item> {
        let oelement = self.htmlcol.item(self.i);
        if oelement.is_some() {
            self.i += 1;
        }
        oelement.map(SWebsysElement)
    }
}

impl TSauspielHtmlDocument for SWebsysDocument {
    type HtmlNode<'node> = SWebsysElement;
    fn find_class<'slf>(&'slf self, str_class: &'static str) -> impl Debug+Iterator<Item=Self::HtmlNode<'slf>>+'slf {
        SHtmlCollectionIterator::new(self.0.get_elements_by_class_name(str_class))
    }
    fn find_name(&self, str_name: &'static str) -> impl Debug+Iterator<Item=Self::HtmlNode<'_>> + '_ {
        SHtmlCollectionIterator::new(self.0.get_elements_by_tag_name(str_name))
    }
    fn find_inner_html<'slf>(&'slf self, str_inner_html: &str) -> impl Debug+Iterator<Item=Self::HtmlNode<'slf>> {
        self.find_name("*") // all elements, see https://developer.mozilla.org/en-US/docs/Web/API/Document/getElementsByTagName
            .filter(move |element| element.0.inner_html()==str_inner_html)
    }
}

impl TSauspielHtmlNode<'_> for SWebsysElement {
    fn find_name(&self, str_name: &'static str) -> impl Debug+Iterator<Item=Self> {
        SHtmlCollectionIterator::new(self.0.get_elements_by_tag_name(str_name))
    }
    fn find_attr(&self, str_attr: &str, attr: ()) -> impl Debug+Iterator<Item=Self> {
        verify_is_unit!(attr); // otherwise has_attribute not meaningful
        SHtmlCollectionIterator::new(self.0.get_elements_by_tag_name("*"))
            .filter(move |element| element.0.has_attribute(str_attr))
    }
    fn find_class(&self, str_class: &'static str) -> impl Debug+Iterator<Item=Self> {
        SHtmlCollectionIterator::new(self.0.get_elements_by_class_name(str_class))
    }
    fn attr(&self, str_attr: &str) -> Option<String> {
        self.0.get_attribute(str_attr)
    }
    fn parent(&self) -> Option<Self> {
        self.0.parent_element().map(SWebsysElement)
    }
    fn inner_html(&self) -> String {
        self.0.inner_html()
    }
    fn children(&self) -> impl Debug+Iterator<Item=Self> {
        SHtmlCollectionIterator::new(self.0.children())
    }
    fn data(&self) -> VSauspielHtmlData {
        match self.0.node_type() {
            3 => VSauspielHtmlData::Text(self.text()),
            1 => VSauspielHtmlData::Element,
            _ => VSauspielHtmlData::Comment, // TODO unsupported types
        }
    }
    fn name(&self) -> Option<String> {
        Some(self.0.tag_name())
    }
    fn text(&self) -> String {
        unwrap!(self.0.text_content())
    }
}

fn internal_output_card_sauspiel_img(card: ECard, str_style: String) -> html_generator::HtmlElement<impl html_generator::AttributeOrChild> {
    let str_card = format!("{card}").replace('Z', "X"); // TODO proper card formatter
    use html_generator::*;
    elements::span((
        class(format!("card-icon card-icon-by card-icon-{str_card}")),
        attributes::title(str_card.clone()),
        attributes::style(str_style),
        str_card
    ))
    /* // TODO This would look better:
    <div class="game-protocol-trick-card position-1  " style="/*! text-align: center; */justify-content: center;">
        <a data-userid="119592" data-username="TiltBoi" class="profile-link" href="/profile/TiltBoi" style="margin: 0 auto;">TiltBoi</a>
        <span class="card-image by g2 HO" title="Der Rote" style="margin: 0 auto;">Der Rote</span>
        <table><tbody>
            <tr>
                <td style="padding: 5px;text-align: center;"><span class="card-icon card-icon-by card-icon-HA" title="HA" style="box-shadow: inset 0px 0px 5px black;border-radius: 5px;">HA</span><span class="card-icon card-icon-by card-icon-HX" title="HX">HX</span><span class="card-icon card-icon-by card-icon-SA" title="SA">SA</span><span class="card-icon card-icon-by card-icon-SO" title="SO">SO</span></td><td style="padding: 5px;text-align: center;"><span class="card-icon card-icon-by card-icon-EK" title="EK">EK</span></td>
            </tr>
            <tr>
                <td style="padding: 5px;text-align: center;">100</td><td style="padding: 5px;text-align: center;">-100 </td>
            </tr>
        </tbody></table>
    </div>
    */
}

fn output_card_sauspiel_img(card: ECard, b_highlight: bool) -> impl html_generator::AttributeOrChild {
    internal_output_card_sauspiel_img(
        card,
        /*str_style*/if b_highlight { "box-shadow: inset 0px 0px 5px black;border-radius: 4px;" } else { "" }.into(),
    )
}

#[wasm_bindgen(start)]
pub fn greet() {
    set_panic_hook();

    let determine_best_card_sauspiel = |ahand: EnumMap<EPlayerIndex, SHand>, stichseq: &SStichSequence, epi, rules, expensifiers: &SExpensifiers| {
        unwrap!(determine_best_card::<SFilterByOracle,_,_,_,_,_,_,_,_>(
            stichseq,
            Box::new(std::iter::once(ahand)) as Box<_>,
            /*fn_filter*/|stichseq, ahand| {
                SFilterByOracle::new(rules, ahand, stichseq)
            },
            &|_stichseq, _ahand| SGenericMinReachablePayout::<SMaxSelfishMinStrategyHigherKinded, SAlphaBetaPrunerNone>::new(
                rules,
                verify_eq!(epi, unwrap!(stichseq.current_playable_stich().current_playerindex())),
                expensifiers.clone(),
            ),
            /*fn_snapshotcache*/|rulestatecache| {
                rules.snapshot_cache::<SMaxSelfishMinStrategyHigherKinded>(rulestatecache)
            },
            /*fn_visualizer*/SNoVisualization::factory(),
            /*fn_inspect*/&|_inspectionpoint, _i_ahand, _ahand| {},
            /*fn_payout*/&|_stichseq, _ahand, n_payout| (n_payout, ()),
        ))
    };
    fn append_sibling(node_existing: &web_sys::Node, node_new: &web_sys::Node) {
        unwrap!(
            unwrap!(node_existing.parent_element())
                .append_child(node_new)
        );
    }
    let document = unwrap!(unwrap!(web_sys::window()).document());
    let mut vecahandstichseqcardepielement = Vec::new();
    #[derive(Clone)]
    struct SDetermineRulesStep {
        epi: EPlayerIndex,
        node_determinerules: web_sys::Element,
        resslcschlag_trumpf: Result<&'static [ESchlag], ()>,
        resoefarbe_trumpf: Result<Option<EFarbe>, ()>,
    }
    match internal_analyze_sauspiel_html(
        SWebsysDocument(document.clone()),
        /*fn_gameannouncement*/|_, ogameannouncement, node_gameannouncement| (ogameannouncement.clone(), node_gameannouncement),
        /*fn_determinerules_step*/|epi, oeobslcschlagoefarbe_trumpf, SWebsysElement(node_determinerules)| SDetermineRulesStep {
            epi,
            resslcschlag_trumpf: oeobslcschlagoefarbe_trumpf.clone().and_then(EitherOrBoth::left).ok_or(()),
            resoefarbe_trumpf: oeobslcschlagoefarbe_trumpf.clone().and_then(EitherOrBoth::right).ok_or(()),
            node_determinerules,
        },
        /*fn_before_play_card*/|game, card, epi, element_played_card| {
            vecahandstichseqcardepielement.push((
                (game.ahand.clone(), game.stichseq.clone()),
                card,
                epi,
                element_played_card.0
            ));
        },
    ) {
        Ok((SGameResultGeneric{stockorgame: VStockOrT::OrT(game_finished), an_payout}, mapepistr_username)) => {
            let (_ogameannouncement, SWebsysElement(node_gameannouncement_epi0)) = &game_finished.mapepigameannouncement[EPlayerIndex::EPI0];
            fn epi_to_sauspiel_position(epi: EPlayerIndex) -> usize {
                epi.to_usize() + 1
            }
            fn output_position_and_cards_sauspiel_img(epi: EPlayerIndex, slccard: &mut [ECard], trumpfdecider: &STrumpfDecider) -> String {
                trumpfdecider.sort_cards(slccard);
                let mut str_position_and_cards = format!("({})", epi_to_sauspiel_position(epi));
                for card in slccard {
                    str_position_and_cards += &internal_output_card_sauspiel_img(*card, "".into()).to_string();
                }
                str_position_and_cards
            }
            for epi in EPlayerIndex::values() {
                let prepend_cards = |slcschlag_trumpf: &'static [ESchlag], oefarbe_trumpf: Option<EFarbe>, node: &web_sys::Element| {
                    node.set_inner_html(&format!("{} {}",
                        output_position_and_cards_sauspiel_img(epi, &mut game_finished.aveccard[epi].clone(), &STrumpfDecider::new(slcschlag_trumpf, oefarbe_trumpf)),
                        node.inner_html()),
                    );
                };
                let (ogameannouncement, SWebsysElement(node_gameannouncement)) = &game_finished.mapepigameannouncement[epi];
                let mut itdeterminerulesstep_epi = game_finished.determinerules.iter()
                    .filter(|determinerulesstep| determinerulesstep.epi==epi);
                let (slcschlag_trumpf_gameannouncement, oefarbe_trumpf_gameannouncement) = if let Some(SGameAnnouncementAnonymous) = ogameannouncement {
                    let mut vecdeterminerulesstep = itdeterminerulesstep_epi.cloned().collect::<Vec<_>>(); // TODO can we work directly on game.determinerules?
                    assert!(!vecdeterminerulesstep.is_empty());
                    { // Sauspiel reveals slcschlag_trumpf first, so we propagate that forwards
                        if vecdeterminerulesstep[0].resslcschlag_trumpf.is_err() {
                            vecdeterminerulesstep[0].resslcschlag_trumpf = Ok(&[ESchlag::Ober, ESchlag::Unter]);
                        }
                        for i_determinerulesstep in 1..vecdeterminerulesstep.len() {
                            if vecdeterminerulesstep[i_determinerulesstep].resslcschlag_trumpf.is_err() {
                                vecdeterminerulesstep[i_determinerulesstep].resslcschlag_trumpf = verify!(vecdeterminerulesstep[i_determinerulesstep-1].resslcschlag_trumpf);
                            }
                        }
                    }
                    { // Sauspiel reveals oefarbe_trumpf potentially last, so we propagate that backwards
                        let determinerulesstep_last = unwrap!(vecdeterminerulesstep.last_mut());
                        assert!(determinerulesstep_last.resslcschlag_trumpf.is_ok());
                        if determinerulesstep_last.resoefarbe_trumpf.is_err() {
                            // determine oefarbe_trumpf heuristically
                            let trumpfdecider_no_farbe = STrumpfDecider::new(
                                unwrap!(determinerulesstep_last.resslcschlag_trumpf),
                                /*ofearbe*/None
                            );
                            let mut mapefarben_count = EFarbe::map_from_fn(|_efarbe| 0);
                            for &card in &game_finished.aveccard[epi] {
                                match trumpfdecider_no_farbe.trumpforfarbe(card) {
                                    VTrumpfOrFarbe::Trumpf => {},
                                    VTrumpfOrFarbe::Farbe(efarbe) => {
                                        mapefarben_count[efarbe] += 1;
                                    },
                                }
                            }
                            determinerulesstep_last.resoefarbe_trumpf = Ok(EFarbe::values().max_by(|&efarbe_lhs, &efarbe_rhs|
                                mapefarben_count[efarbe_lhs].cmp(&mapefarben_count[efarbe_rhs])
                                    .then_with(|| {
                                        let has_ass = |efarbe| {
                                            game_finished.aveccard[epi].contains(&ECard::new(efarbe, ESchlag::Ass))
                                        };
                                        has_ass(efarbe_rhs).cmp(&has_ass(efarbe_lhs))
                                    })
                            ))
                        }
                        for i_determinerulesstep in (1..vecdeterminerulesstep.len()).rev() {
                            if vecdeterminerulesstep[i_determinerulesstep-1].resoefarbe_trumpf.is_err() {
                                vecdeterminerulesstep[i_determinerulesstep-1].resoefarbe_trumpf = verify!(vecdeterminerulesstep[i_determinerulesstep].resoefarbe_trumpf);
                            }
                        }
                    }
                    // vecdeterminerulesstep is prepared, so we can improve determinerules section
                    for SDetermineRulesStep{epi:_, node_determinerules, resslcschlag_trumpf, resoefarbe_trumpf} in &vecdeterminerulesstep {
                        prepend_cards(unwrap!(resslcschlag_trumpf), unwrap!(resoefarbe_trumpf), node_determinerules);
                    }
                    let determinerulesstep_first = unwrap!(vecdeterminerulesstep.first());
                    (unwrap!(determinerulesstep_first.resslcschlag_trumpf), unwrap!(determinerulesstep_first.resoefarbe_trumpf))
                } else {
                    assert!(itdeterminerulesstep_epi.next().is_none());
                    (type_inference!(&'static[ESchlag], &[ESchlag::Ober, ESchlag::Unter]), Some(EFarbe::Herz))
                };
                prepend_cards(slcschlag_trumpf_gameannouncement, oefarbe_trumpf_gameannouncement, node_gameannouncement);
            }
            let rules = &game_finished.rules;
            let node_gameannouncements = unwrap!(node_gameannouncement_epi0.parent_element()); // TODO? Find common predecessor of all nodes in game_finished.mapepigameannouncement?
            { // Add doublings and stoss to protocol
                let node_doubling_or_stoss = |epi, mut veccard: Vec<ECard>, str_explanation, trumpfdecider| {
                    let node_doubling_or_stoss = unwrap!(
                        unwrap!(node_gameannouncement_epi0.clone_node())
                            .dyn_into::<web_sys::Element>() // TODO can we avoid this?
                    );
                    node_doubling_or_stoss.set_inner_html(&format!("{} {} {}",
                        output_position_and_cards_sauspiel_img(epi, &mut veccard, trumpfdecider),
                        mapepistr_username[epi],
                        str_explanation,
                    ));
                    node_doubling_or_stoss
                };
                // Add doublings to protocol
                let trumpfdecider_doubling = STrumpfDecider::new(&[ESchlag::Ober, ESchlag::Unter], Some(EFarbe::Herz));
                for epi in EPlayerIndex::values().rev(/*so that we can repeatedly prepend*/) {
                    unwrap!(node_gameannouncements.prepend_with_node_1(&node_doubling_or_stoss(
                        epi,
                        first_hand_for(&game_finished.aveccard[epi], game_finished.kurzlang()).to_vec(),
                        if *unwrap!(game_finished.expensifiers.doublings.get(epi)) {
                            "klopft"
                        } else {
                            "klopft nicht"
                        },
                        &trumpfdecider_doubling,
                    )));
                }
                // Add stoss to protocol
                for (stoss, str_explanation_stoss) in std::iter::zip(
                    &game_finished.expensifiers.vecstoss,
                    itertools::chain(
                        ["gibt Kontra", "gibt Retour"],
                        std::iter::repeat("gibt Stoss"),
                    )
                ) {
                    let epi = stoss.epi;
                    unwrap!(node_gameannouncements.append_with_node_1(&node_doubling_or_stoss(
                        epi,
                        game_finished.aveccard[epi].to_vec(),
                        str_explanation_stoss,
                        rules.trumpfdecider(),
                    )));
                }
            }
            #[derive(PartialOrd, Ord, PartialEq, Eq, Clone)]
            enum EPlayedCardSeverity {
                Optimal,
                Suboptimal(bool/*b_loss_realized*/),
            }
            fn suboptimal_quality_to_html_color(b_loss_realized: bool) -> &'static str {
                if b_loss_realized {
                    "#d70000" // red
                } else {
                    "#ffef00" // yellow
                }
            }
            let mut vecepicardocardseverity = Vec::new();
            for ((ahand, stichseq), card_played, epi, element_played_card) in vecahandstichseqcardepielement {
                let ocardseverity = if_then_some!(stichseq.remaining_cards_per_hand()[epi] <= if_dbg_else!({3}{5}), {
                    let determinebestcardresult = determine_best_card_sauspiel(
                        ahand.clone(),
                        &stichseq,
                        epi,
                        rules,
                        &game_finished.expensifiers,
                    );
                    let div_table = unwrap!(document.create_element("div"));
                    div_table.set_inner_html(&{
                        html_display_children(html_payout_table::<_, SMaxSelfishMinStrategyHigherKinded>(
                            rules,
                            &ahand,
                            &stichseq,
                            &determinebestcardresult,
                            card_played,
                            &output_card_sauspiel_img,
                        )).to_string()
                    });
                    append_sibling(&element_played_card, &div_table);
                    let get_min_max_eq = |payoutstats: &SMaxSelfishMinStrategy<SPayoutStats<()>>| {
                        let payoutstats = &payoutstats.maxselfishmin.0;
                        verify_eq!(payoutstats.min(), payoutstats.max())
                    };
                    let (veccard_optimal, payoutstats_optimal) = determinebestcardresult.cards_with_maximum_value(
                        |lhs: &SMaxSelfishMinStrategy<SPayoutStats<()>>, rhs| {
                            get_min_max_eq(lhs).cmp(&get_min_max_eq(rhs))
                        },
                    );
                    if !veccard_optimal.contains(&card_played) {
                        match an_payout[epi].cmp(&get_min_max_eq(payoutstats_optimal)) {
                            Ordering::Greater | Ordering::Equal => EPlayedCardSeverity::Suboptimal(/*b_loss_realized*/false),
                            Ordering::Less => EPlayedCardSeverity::Suboptimal(/*b_loss_realized*/true),
                        }
                    } else {
                        EPlayedCardSeverity::Optimal
                    }
                });
                vecepicardocardseverity.push((epi, card_played, ocardseverity));
                let div_button = unwrap!(document.create_element("div"));
                div_button.set_inner_html(&html_copy_button(
                    rules,
                    &ahand,
                    &stichseq,
                    /*str_openschafkopf_executable*/"openschafkopf",
                ));
                append_sibling(&element_played_card, &div_button);
            }
            use html_generator::*; // TODO narrower scope?
            fn table_cell_with_background(str_tag_name: &'static str, str_text: impl AttributeOrChild, ocardseverity: &Option<EPlayedCardSeverity>) -> HtmlElement<impl AttributeOrChild> {
                let str_color = match ocardseverity {
                    None | Some(EPlayedCardSeverity::Optimal) => "", // Do not indicate "unchecked" or "optimal" in overview cells
                    Some(EPlayedCardSeverity::Suboptimal(b_loss_realized)) => suboptimal_quality_to_html_color(*b_loss_realized),
                };
                HtmlElement::new(str_tag_name, (attributes::style(format!("background-color: {str_color};")), str_text))
            }
            let itepi_cycled_twice = itertools::chain(
                EPlayerIndex::values(),
                EPlayerIndex::values().take(EPlayerIndex::SIZE - 1),
            );
            /*TODO const*/let html_table_gap_cell = th(attributes::style("width: 10px; background: none;"));
            let node_whole_game = unwrap!(
                unwrap!(node_gameannouncement_epi0.clone_node())
                    .dyn_into::<web_sys::Element>() // TODO can we avoid this?
            );
            let mut mapepiocardseverity = EPlayerIndex::map_from_fn(|_epi| None);
            for (epi, _card, ocardseverity) in vecepicardocardseverity.iter() {
                assign_gt(&mut mapepiocardseverity[*epi], ocardseverity); // exploits that Option::None is smaller than any Option::Some(_) // TODO Good idea?
            }
            node_whole_game.set_inner_html(&table(tbody((
                tr((
                    th(()), // empty cell to match subsequent rows // TODO merge with next row's cell?
                    html_table_gap_cell.clone(),
                    th((
                        colspan(format!("{}", itepi_cycled_twice.clone().count())),
                        "Karten",
                    )),
                    html_table_gap_cell.clone(),
                    th((
                        colspan(format!("{}", EPlayerIndex::SIZE)),
                        "Augen", // "Augen" as used by sauspiel.de
                    )),
                )),
                tr((
                    th(()), // empty cell to match subsequent rows
                    html_table_gap_cell.clone(),
                    html_iter(itepi_cycled_twice.clone().map(|epi_header| {
                        table_cell_with_background("th", format!("{}", epi_to_sauspiel_position(epi_header)), &mapepiocardseverity[epi_header])
                    })),
                    html_table_gap_cell.clone(),
                    html_iter(EPlayerIndex::values().map(|epi_points|
                        th(format!("{}", epi_to_sauspiel_position(epi_points)))
                    )),
                )),
                vecepicardocardseverity.chunks(EPlayerIndex::SIZE).zip_eq(game_finished.stichseq.completed_stichs_winner_index(&game_finished.rules)).enumerate().map(|(i_stich, (slcepicardocardseverity_stich, (stich, epi_winner)))| {
                    tr((
                        table_cell_with_background(
                            "td",
                            /*str_text*/format!("{}. Stich", i_stich+1),
                            unwrap!(slcepicardocardseverity_stich.iter().map(|(_epi, _card, ocardseverity)| ocardseverity).max()),
                        ),
                        html_table_gap_cell.clone(),
                        html_iter(itertools::merge_join_by(
                            itepi_cycled_twice.clone(),
                            slcepicardocardseverity_stich,
                            |epi_running_index, (epi_card, _card, _ocardseverity)| {
                                epi_running_index.cmp(epi_card)
                            },
                        ).map(move |eitherorboth| td(match eitherorboth {
                            EitherOrBoth::Left(_epi_running_index) => None,
                            EitherOrBoth::Both(epi_running_index, (epi_card, card, ocardseverity)) => {
                                assert_eq!(epi_running_index, *epi_card);
                                let str_color = match ocardseverity {
                                    Some(EPlayedCardSeverity::Optimal) => "#78db00", // green
                                    Some(EPlayedCardSeverity::Suboptimal(b_loss_realized)) => suboptimal_quality_to_html_color(*b_loss_realized),
                                    None => "lightgrey",
                                };
                                Some(internal_output_card_sauspiel_img(*card, format!("border-top: 5px solid {str_color};box-sizing: content-box;")))
                            },
                            EitherOrBoth::Right(_) => panic!(),
                        }))),
                        html_table_gap_cell.clone(),
                        html_iter(EPlayerIndex::values().map(move |epi_points| 
                            td((
                                attributes::style("padding: 5px;"),
                                if_then_some!(epi_points==epi_winner, format!("{}", points_stich(stich))),
                            ))
                        )),
                    ))
                })
                .collect::<Vec<_>>(), // TODO avoid
                tr((
                    td(colspan(format!("{}", // TODO(html_generator) support format_args
                        1 // Column "i-th Stich"
                        + 1 // html_table_gap_cell
                        + itepi_cycled_twice.clone().count()
                        + 1 // html_table_gap_cell
                    ))),
                    html_iter(EPlayerIndex::values().map(|epi_points| {
                        let n_points = game_finished.stichseq.completed_stichs_winner_index(&game_finished.rules)
                            .filter_map(|(stich, epi_winner)|
                                if_then_some!(epi_points==epi_winner, points_stich(stich))
                            )
                            .sum::<isize>();
                        td((
                            attributes::style("padding: 5px; border-top: 1px solid black;"),
                            format!("{}", n_points),
                        ))

                    })),
                ))
            ))).to_string());
            unwrap!(node_gameannouncements.append_with_node_1(&node_whole_game));
        },
        Ok((SGameResultGeneric{stockorgame: VStockOrT::Stock(_), an_payout:_}, _mapepistr_username)) => {
            // Nothing to analyze for Stock.
        },
        Err(err_html) => { // TODO we should distinguish between "Game not visible/found/accessible" and "HTML not understood"
            #[cfg(not(feature="sauspiel_webext_use_json"))] {
                dbg_alert!(&format!("Error parsing document:\n{err_html:?}"));
            }
            #[cfg(feature="sauspiel_webext_use_json")] {
                let mut vecahandstichseqcardepi = Vec::new();
                match analyze_sauspiel_json(
                    &{
                        let xmlhttprequest = unwrap!(web_sys::XmlHttpRequest::new());
                        let str_json_url = format!("{}.json", unwrap!(unwrap!(web_sys::window()).location().href()));
                        dbg_alert!(&str_json_url);
                        unwrap!(xmlhttprequest.open_with_async(
                            "GET",
                            &str_json_url,
                            /*async*/false,
                        ));
                        unwrap!(xmlhttprequest.send());
                        let str_json = unwrap!( // unwrap Option
                            unwrap!(xmlhttprequest.response_text()) // unwrap Result
                        );
                        assert!(xmlhttprequest.status().is_ok());
                        dbg_alert!(&str_json);
                        str_json
                    },
                    /*fn_before_zugeben*/|game, _i_stich, epi, card| {
                        vecahandstichseqcardepi.push((
                            (game.ahand.clone(), game.stichseq.clone()),
                            card,
                            epi,
                        ));
                    },
                ) {
                    Ok(SGameResultGeneric{stockorgame: VStockOrT::OrT(game_finished), an_payout:_}) => {
                        use html_generator::*;
                        let str_html_out = html_display_children(html_iter(vecahandstichseqcardepi.into_iter().map(|((ahand, stichseq), card_played, epi)| {
                            (
                                table(tr((
                                    html_display_children(player_table_stichseq(/*epi_self*/EPlayerIndex::EPI0, &stichseq, &output_card_sauspiel_img)).to_string(),
                                    html_display_children(player_table_ahand(
                                        /*epi_self*/EPlayerIndex::EPI0,
                                        &ahand,
                                        &game_finished.rules,
                                        /*fn_border*/move |card| card==card_played,
                                        &output_card_sauspiel_img,
                                    )).to_string(),
                                ))),
                                if_then_some!(stichseq.remaining_cards_per_hand()[epi] <= if_dbg_else!({3}{5}), {
                                    html_payout_table::<_, SMaxSelfishMinStrategyHigherKinded>(
                                        &game_finished.rules,
                                        &ahand,
                                        &stichseq,
                                        &determine_best_card_sauspiel(
                                            ahand.clone(),
                                            &stichseq,
                                            epi,
                                            &game_finished.rules,
                                            &game_finished.expensifiers,
                                        ),
                                        card_played,
                                        &output_card_sauspiel_img,
                                    )
                                }),
                                div(html_copy_button(
                                    &game_finished.rules,
                                    &ahand,
                                    &stichseq,
                                    /*str_openschafkopf_executable*/"openschafkopf",
                                )),
                            )
                        }))).to_string();
                        let div_openschafkopf_overview = unwrap!(document.create_element("div"));
                        div_openschafkopf_overview.set_inner_html(&str_html_out);
                        let document = SWebsysDocument(document);
                        append_sibling(
                            &unwrap!(
                                document
                                    .find_class("game-overview")
                                    .exactly_one()
                            ).0,
                            &div_openschafkopf_overview,
                        );
                    },
                    Ok(SGameResultGeneric{stockorgame: VStockOrT::Stock(_), an_payout:_}) => {
                        // Nothing to analyze for Stock.
                    },
                    Err(err_json) => {
                        dbg_alert!(&format!("Error parsing document:\n{err_html:?}\n{err_json:?}"));
                    },
                }
            }
        },
    };
}


