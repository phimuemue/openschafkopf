mod utils;

use wasm_bindgen::prelude::*;
use openschafkopf_util::*;
use openschafkopf_lib::{
    game::SGameResultGeneric,
    game_analysis::{analyze_game, parser::analyze_sauspiel_html},
    rules::ruleset::VStockOrT,
};
use crate::utils::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen(start)]
pub fn greet() {
    set_panic_hook();

    let document = unwrap!(unwrap!(web_sys::window()).document());
    match analyze_sauspiel_html(&unwrap!(document.body()).inner_html()) {
        Ok(SGameResultGeneric{stockorgame: VStockOrT::OrT(game), an_payout:_}) => {
            let gameanalysis = analyze_game(
                /*str_description*/"",
                /*str_link*/"", // TODO get URL
                game.map(
                    /*fn_announcements*/|_| (),
                    /*fn_determinerules*/|_| (),
                    /*fn_ruleset*/|_| (),
                    /*fn_rules*/|rules| rules,
                ),
                /*n_max_remaining_cards*/4,
                /*b_simulate_all_hands*/true,
                /*str_openschafkopf_executable*/"openschafkopf",
                /*fn_output_card*/&|card, b_highlight| {
                    let str_card = format!("{}", card).replace('Z', "X"); // TODO proper card formatter
                    format!(r#"<span class="card-icon card-icon-by card-icon-{str_card}" title="{str_card}" {str_style}>{str_card}</span>"#,
                        str_style = if b_highlight {r#"style="box-shadow: inset 0px 0px 5px black;border-radius: 5px;""#} else {""},
                    )
                },
            );
            let htmlcol = document.get_elements_by_class_name("container content");
            assert_eq!(htmlcol.length(), 1);
            let div_container_content = unwrap!(htmlcol.item(0));
            let div_analysis = unwrap!(document.create_element("div"));
            div_analysis.set_inner_html(&gameanalysis.str_html);
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


