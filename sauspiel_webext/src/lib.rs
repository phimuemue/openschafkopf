mod utils;

use wasm_bindgen::prelude::*;
use openschafkopf_util::*;
use openschafkopf_lib::{
    game::SGameResultGeneric,
    game_analysis::{analyze_game, parser::{internal_analyze_sauspiel_html, TSauspielHtmlDocument, TSauspielHtmlNode, VSauspielHtmlData}},
    rules::ruleset::VStockOrT,
};
use crate::utils::*;
use std::fmt::Debug;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

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
    match internal_analyze_sauspiel_html(SWebsysDocument(document.clone())) {
        Ok(SGameResultGeneric{stockorgame: VStockOrT::OrT(game), an_payout:_}) => {
            let str_html = analyze_game(
                game.map(
                    /*fn_announcements*/|_| (),
                    /*fn_determinerules*/|_| (),
                    /*fn_ruleset*/|_| (),
                    /*fn_rules*/|rules| rules,
                ),
                /*n_max_remaining_cards*/5,
                /*b_simulate_all_hands*/false,
            )
                .generate_analysis_html(
                    /*str_description*/"",
                    /*str_link*/"", // TODO get URL
                    /*str_openschafkopf_executable*/"openschafkopf",
                    /*fn_output_card*/&|card, b_highlight| {
                        let str_card = format!("{}", card).replace('Z', "X"); // TODO proper card formatter
                        format!(r#"<span class="card-icon card-icon-by card-icon-{str_card}" title="{str_card}" {str_style}>{str_card}</span>"#,
                            str_style = if b_highlight {r#"style="box-shadow: inset 0px 0px 5px black;border-radius: 5px;""#} else {""},
                        )
                        /* // TODO Show table directly below played card, similar to this:
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
                );
            let htmlcol = document.get_elements_by_class_name("container content");
            assert_eq!(htmlcol.length(), 1);
            let div_container_content = unwrap!(htmlcol.item(0));
            let div_analysis = unwrap!(document.create_element("div"));
            div_analysis.set_inner_html(&str_html);
            unwrap!(div_container_content.append_child(&div_analysis));
        },
        Ok(SGameResultGeneric{stockorgame: VStockOrT::Stock(_), an_payout:_}) => {
            alert("Game does not contain any played cards.");
        },
        Err(err) => {
            alert(&format!("Error parsing document: {:?}", err));
        },
    };
}


