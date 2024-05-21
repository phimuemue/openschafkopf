mod utils;

use wasm_bindgen::prelude::*;
use openschafkopf_util::*;
use openschafkopf_lib::{
    ai::{determine_best_card, gametree::{SAlphaBetaPrunerNone, SGenericMinReachablePayout, SNoVisualization, SMaxSelfishMinStrategyHigherKinded}, stichoracle::SFilterByOracle},
    game::SGameResultGeneric,
    game_analysis::{append_html_payout_table, parser::{internal_analyze_sauspiel_html, TSauspielHtmlDocument, TSauspielHtmlNode, VSauspielHtmlData}},
    rules::{TRules, ruleset::VStockOrT},
};
use crate::utils::*;
use std::fmt::Debug;

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

impl<'node> TSauspielHtmlNode<'node> for SWebsysElement {
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

#[wasm_bindgen(start)]
pub fn greet() {
    set_panic_hook();

    let document = unwrap!(unwrap!(web_sys::window()).document());
    let mut vecahandstichseqcardepielement = Vec::new();
    match internal_analyze_sauspiel_html(
        SWebsysDocument(document.clone()),
        |game, card, epi, element_played_card| {
            if game.stichseq.remaining_cards_per_hand()[epi] <= if_dbg_else!({3}{5}) {
                vecahandstichseqcardepielement.push((
                    (game.ahand.clone(), game.stichseq.clone()),
                    card,
                    epi,
                    element_played_card.0
                ));
            }
        },
    ) {
        Ok(SGameResultGeneric{stockorgame: VStockOrT::OrT(game_finished), an_payout:_}) => {
            let rules = &game_finished.rules;
            for ((ahand, stichseq), card_played, epi, element_played_card) in vecahandstichseqcardepielement {
                let determinebestcardresult = unwrap!(determine_best_card::<SFilterByOracle,_,_,_,_,_,_,_,_>(
                    &stichseq,
                    Box::new(std::iter::once(ahand.clone())) as Box<_>,
                    /*fn_filter*/|stichseq, ahand| {
                        SFilterByOracle::new(rules, ahand, stichseq)
                    },
                    &|_stichseq, _ahand| SGenericMinReachablePayout::<SMaxSelfishMinStrategyHigherKinded, SAlphaBetaPrunerNone>::new(
                        rules,
                        verify_eq!(epi, unwrap!(stichseq.current_playable_stich().current_playerindex())),
                        game_finished.expensifiers.clone(),
                    ),
                    /*fn_snapshotcache*/|rulestatecache| {
                        rules.snapshot_cache::<SMaxSelfishMinStrategyHigherKinded>(rulestatecache)
                    },
                    /*fn_visualizer*/SNoVisualization::factory(),
                    /*fn_inspect*/&|_b_before, _i_ahand, _ahand, _card| {},
                    /*fn_payout*/&|_stichseq, _ahand, n_payout| (n_payout, ()),
                ));
                let div_table = unwrap!(document.create_element("div"));
                div_table.set_inner_html(&{
                    let mut str_table = String::new();
                    append_html_payout_table::<SMaxSelfishMinStrategyHigherKinded>(
                        &mut str_table,
                        rules,
                        &ahand,
                        &stichseq,
                        &determinebestcardresult,
                        card_played,
                        /*fn_output_card*/&|card, b_highlight| {
                            let str_card = format!("{}", card).replace('Z', "X"); // TODO proper card formatter
                            format!(r#"<span class="card-icon card-icon-by card-icon-{str_card}" title="{str_card}" {str_style}>{str_card}</span>"#,
                                str_style = if b_highlight {r#"style="box-shadow: inset 0px 0px 5px black;border-radius: 5px;""#} else {""},
                            )
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
                        },
                        /*str_openschafkopf_executable*/"openschafkopf",
                    );
                    str_table
                });
                unwrap!(
                    unwrap!(element_played_card.parent_element())
                        .append_child(&div_table)
                );
            }
        },
        Ok(SGameResultGeneric{stockorgame: VStockOrT::Stock(_), an_payout:_}) => {
            // Nothing to analyze for Stock.
        },
        Err(err) => { // TODO we should distinguish between "Game not visible/found/accessible" and "HTML not understood"
            dbg_alert!(&format!("Error parsing document: {:?}", err));
        },
    };
}


