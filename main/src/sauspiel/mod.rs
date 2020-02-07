use crate::util::*;
use crate::primitives::cardvector::*;
use crate::primitives::*;
use select::{
    document::Document,
    predicate::*,
    node::Node,
};
use crate::game_analysis::*;
use combine::{*, char::*};

pub fn analyze_html(str_html: &str) -> Result<SAnalyzeParams, failure::Error> {
    let doc = Document::from(&str_html as &str);
    fn vec_to_arr<T: std::fmt::Debug+Clone/*TODO can we avoid clone?*/>(vect: Vec<T>) -> Result<[T; EPlayerIndex::SIZE], failure::Error> {
        if_then_some!(
            EPlayerIndex::SIZE==vect.len(),
            [vect[0].clone(), vect[1].clone(), vect[2].clone(), vect[3].clone()]
        ).ok_or_else(|| format_err!("Wrong number of elements ({}) in {:?}", vect.len(), vect))
    }
    fn vec_to_enummap<T: std::fmt::Debug+Clone/*TODO can we avoid clone?*/>(vect: Vec<T>) -> Result<EnumMap<EPlayerIndex, T>, failure::Error> {
        vec_to_arr(vect).map(EPlayerIndex::map_from_raw)
    }
    let mapepistr_username = vec_to_enummap(
        doc.find(Class("game-participants"))
            .single()
            .map_err(|err| format_err!("error on single: {:?}", err))?
            .find(Attr("data-username", ()))
            .map(|node_username| debug_verify!(node_username.attr("data-username")).unwrap())
            .collect()
    )?;
    let username_to_epi = |str_username: &str| {
        EPlayerIndex::values()
            .find(|epi| mapepistr_username[*epi]==str_username)
            .ok_or_else(|| format_err!("username {} not part of mapepistr_username {:?}", str_username, mapepistr_username))
    };
    let find_cards = |node: &Node| -> Result<Vec<SCard>, failure::Error> {
        node.find(Class("card-image"))
            .map(|node_card| -> Result<SCard, _> {
                let str_class = debug_verify!(node_card.attr("class")).unwrap(); // "class" must be present
                (
                    string("card-image by g"),
                    digit(),
                    space(),
                )
                .with(card_parser())
                .skip(optional(string(" highlight")))
                .skip(eof())
                    // end of parser
                    .parse(str_class)
                    .map_err(|err| format_err!("Card parsing: {:?} on {}", err, str_class))
                    .map(|(card, _str)| card)
            })
            .collect::<Result<Vec<_>,_>>()
    };
    let scrape_from_key_figure_table = |str_key| -> Result<_, failure::Error> {
        doc.find(Name("th").and(|node: &Node| node.inner_html()==str_key))
            .single().map_err(|err| format_err!("Error with {}: no single <th>{}</th>: {:?}", str_key, str_key, err))?
            .parent().ok_or_else(|| format_err!("Error with {}: {} has no parent", str_key, str_key))?
            .find(Name("td"))
            .single().map_err(|err| format_err!("Error with {}: no single <td> containing {}: {:?}", str_key, str_key, err))
    };
    let (n_tarif_extra, n_tarif_ruf, n_tarif_solo) = {
        let str_tarif = scrape_from_key_figure_table("Tarif")?.inner_html();
        let parser_digits = many1::<String,_>(digit())
            .map(|str_digits| str_digits.parse::<isize>());
        macro_rules! parser_tarif(($parser_currency: expr, $parser_digits: expr) => {
            $parser_currency.with((
                $parser_digits.clone(),
                string(" / ").with($parser_digits.clone()),
                string(" / ").with($parser_digits.clone()),
            )).map(|(resn_extra, resn_ruf, resn_solo)| -> Result<_, failure::Error> {
                Ok((resn_extra?, resn_ruf?, resn_solo?))
            })
        });
        spaces().with(
            choice!(
                parser_tarif!(string("P "), parser_digits),
                parser_tarif!(
                    choice!(string("€ "), string("$ ")), // Note: I could not find a game from Vereinsheim, but I suspect they use $
                    (parser_digits.clone(), char(','), count_min_max::<String,_>(2, 2, digit()))
                        .map(|(resn_before_comma, _str_comma, str_2_digits_after_comma)| -> Result<_, failure::Error> {
                            let n_before_comma : isize = resn_before_comma?;
                            let n_after_comma : isize = str_2_digits_after_comma.parse::<isize>()?;
                            Ok(n_before_comma * 100 + n_after_comma)
                        })
                )
            )
        )
            .skip((spaces(), eof()))
            // end of parser
            .parse(&str_tarif as &str)
            .map_err(|err| format_err!("Error in tarif parsing: {:?} on {}", err, str_tarif))
            .map(|(resnnn, _str)| resnnn)
                ? // unpack result of combine::parse call
                ? // unpack parsed result
    };
    let rules = doc.find(Class("title-supertext"))
        .single()
        .map_err(|err| format_err!("title-supertext single failed {:?}", err))?
        .parent().ok_or_else(|| format_err!("title-supertext has no parent"))?
        .find(Name("h1"))
        .single()
        .map_err(|err| format_err!("h1 is not single: {:?}", err))
        .and_then(|node_rules| {
            crate::rules::parser::parse_rule_description(
                &node_rules.text(),
                (n_tarif_extra, n_tarif_ruf, n_tarif_solo),
                /*fn_player_to_epi*/username_to_epi,
            )
        })?;
    let vecstich = doc.find(|node: &Node| node.inner_html()=="Stich von")
        .try_fold((EPlayerIndex::EPI0, Vec::new()), |(epi_first, mut vecstich), node| -> Result<_, _> {
            vec_to_arr(find_cards(
                &node.parent().ok_or_else(|| format_err!(r#""Stich von" has no parent"#))?
                    .parent().ok_or_else(|| format_err!("walking html failed"))?
            )?).map(|acard| {
                let stich = SStich::new_full(epi_first, acard);
                let epi_winner = rules.winner_index(&stich);
                vecstich.push(stich);
                (epi_winner, vecstich)
            })
        })?
        .1;
    let ahand = vec_to_enummap(
        doc.find(|node: &Node| node.inner_html()=="Karten von:")
            .map(|node| -> Result<SHand, failure::Error> {
                let node_parent = node.parent().ok_or_else(|| format_err!(r#""Karten von" has no parent"#))?;
                let node_hand = node_parent.parent().ok_or_else(|| format_err!("walking html failed"))?;
                let veccard_hand = find_cards(&node_hand)?;
                EKurzLang::values().find(|ekurzlang| ekurzlang.cards_per_player()==veccard_hand.len())
                    .ok_or_else(|| format_err!("invalid hand size: {}", veccard_hand.len()))
                    .map(move |_ekurzlang| {
                        SHand::new_from_vec(veccard_hand.into_iter().collect())
                    })
            })
            .collect::<Result<Vec<_>, _>>()?
    )?;
    let get_doublings_stoss = |str_key| -> Result<_, failure::Error> {
        scrape_from_key_figure_table(str_key)?
            .find(Name("a"))
            .map(|node| username_to_epi(&node.inner_html()).map(|epi| epi.to_usize()))
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(SAnalyzeParams {
        rules,
        ahand,
        vecn_doubling: get_doublings_stoss("Klopfer")?,
        vecn_stoss: get_doublings_stoss("Kontra und Retour")?,
        n_stock: 0, // Sauspiel does not support stock
        vecstich,
    })
}
