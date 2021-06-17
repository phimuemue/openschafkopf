use crate::game_analysis::*;
use crate::game::*;
use crate::rules::ruleset::SStossParams;
use crate::primitives::*;
use crate::primitives::cardvector::*;
use crate::util::{*, parser::*};
use std::io::Read;
use itertools::Itertools;

pub fn subcommand(str_subcommand: &str) -> clap::App {
    clap::SubCommand::with_name(str_subcommand)
        .about("Analyze played games and spot suboptimal decisions")
        .arg(clap::Arg::with_name("sauspiel-files")
            .required(true)
            .takes_value(true)
            .multiple(true)
        )
}

pub fn analyze_sauspiel_html(str_html: &str) -> Result<SGame, failure::Error> {
    use combine::{char::*, *};
    use select::{document::Document, node::Node, predicate::*};
    let doc = Document::from(&str_html as &str);
    fn vec_to_arr<T: std::fmt::Debug>(vect: Vec<T>) -> Result<[T; EPlayerIndex::SIZE], failure::Error> {
        let (card0, card1, card2, card3) = vect.into_iter()
            .collect_tuple()
            .ok_or_else(|| format_err!("Wrong number of elements"))?;
        Ok([card0, card1, card2, card3])
    }
    let mapepistr_username = vec_to_arr(
        doc.find(Class("game-participants"))
            .exactly_one()
            .map_err(|it| format_err!("error on single: {} elements", it.count()))? // TODO could it implement Debug?
            .find(Attr("data-username", ()))
            .map(|node_username| unwrap!(node_username.attr("data-username")))
            .collect()
    ).map(EPlayerIndex::map_from_raw)?;
    let username_to_epi = |str_username: &str| {
        EPlayerIndex::values()
            .find(|epi| mapepistr_username[*epi]==str_username)
            .ok_or_else(|| format_err!("username {} not part of mapepistr_username {:?}", str_username, mapepistr_username))
    };
    let scrape_from_key_figure_table = |str_key| -> Result<_, failure::Error> {
        doc.find(Name("th").and(|node: &Node| node.inner_html()==str_key))
            .exactly_one().map_err(|it| format_err!("Error with {}: no single <th>{}</th>: {} elements", str_key, str_key, it.count()))? // TODO could it implement Debug?
            .parent().ok_or_else(|| format_err!("Error with {}: {} has no parent", str_key, str_key))?
            .find(Name("td"))
            .exactly_one().map_err(|it| format_err!("Error with {}: no single <td> containing {}: {} elements", str_key, str_key, it.count())) // TODO could it implement Debug?
    };
    let (n_tarif_extra, n_tarif_ruf, n_tarif_solo) = {
        let str_tarif = scrape_from_key_figure_table("Tarif")?.inner_html();
        let parser_digits = many1::<String,_>(digit())
            .map(|str_digits| str_digits.parse::<isize>());
        macro_rules! parser_tarif(($parser_currency: expr, $parser_digits: expr) => {
            $parser_currency.with((
                $parser_digits.clone(),
                string(" / ").with($parser_digits.clone()),
                optional(string(" / ").with($parser_digits.clone())),
            )).map(|(resn_1, resn_2, oresn_3)| -> Result<_, failure::Error> {
                Ok(if let Some(resn_3)=oresn_3 {
                    (resn_1?, resn_2?, resn_3?)
                } else {
                    let n_2 = resn_2?;
                    (resn_1?, n_2, n_2)
                })
            })
        });
        parse_trimmed(
            &str_tarif,
            "tarif",
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
            ? // unpack result of combine::parse call
            ? // unpack parsed result
    };
    let rules = doc.find(Class("title-supertext"))
        .exactly_one()
        .map_err(|it| format_err!("title-supertext single failed {} elements", it.count()))? // TODO could it implement Debug?
        .parent().ok_or_else(|| format_err!("title-supertext has no parent"))?
        .find(Name("h1"))
        .exactly_one()
        .map_err(|it| format_err!("h1 is not single: {} elements", it.count())) // TODO could it implement Debug?
        .and_then(|node_rules| {
            crate::rules::parser::parse_rule_description(
                &node_rules.text(),
                (n_tarif_extra, n_tarif_ruf, n_tarif_solo),
                /*fn_player_to_epi*/username_to_epi,
            )
        })?;
    let vecstich = doc.find(|node: &Node| node.inner_html()=="Stich von")
        .try_fold((EPlayerIndex::EPI0, Vec::new()), |(epi_first, mut vecstich), node| -> Result<_, _> {
            vec_to_arr(
                node.parent().ok_or_else(|| format_err!(r#""Stich von" has no parent"#))?
                    .parent().ok_or_else(|| format_err!("walking html failed"))?
                    .find(Class("card-image"))
                    .map(|node_card| -> Result<SCard, _> {
                        let str_class = unwrap!(node_card.attr("class")); // "class" must be present
                        (
                            string("card-image "),
                            choice!(string("by"), string("fn")),
                            string(" g"),
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
                    .collect::<Result<Vec<_>,_>>()?
                
            ).map(|acard| {
                let stich = SStich::new_full(epi_first, acard);
                let epi_winner = rules.winner_index(&stich);
                vecstich.push(stich);
                (epi_winner, vecstich)
            })
        })?
        .1;
    let get_doublings_stoss = |str_key| -> Result<_, failure::Error> {
        Ok(scrape_from_key_figure_table(str_key)?
            .find(Name("a"))
            .map(|node| username_to_epi(&node.inner_html())))
    };
    let mut game = SGame::new(
        /*ahand*/EPlayerIndex::map_from_fn(|epi|
            SHand::new_from_iter(
                vecstich
                    .iter()
                    .map(|stich| stich[epi])
            )
        ),
        /*doublings*/{
            let vecepi_doubling = get_doublings_stoss("Klopfer")?.collect::<Result<Vec<_>, _>>()?;
            SDoublings::new_full(
                SStaticEPI0{},
                EPlayerIndex::map_from_fn(|epi| 
                    vecepi_doubling.contains(&epi)
                ).into_raw(),
            )
        },
        /*ostossparams*/Some(SStossParams::new(/*n_stoss_max*/4)),
        rules,
        /*n_stock*/0, // Sauspiel does not support stock
    );
    for resepi in get_doublings_stoss("Kontra und Retour")? {
        let () = game.stoss(resepi?)?;
    }
    for stich in vecstich.into_iter() {
        for (epi, card) in stich.iter() {
            let () = game.zugeben(*card, epi)?;
        }
    }
    Ok(game)
}

fn analyze_plain(str_lines: &str) -> impl Iterator<Item=Result<SGame, failure::Error>> + '_ {
    str_lines
        .lines()
        .map(|str_plain| {
            let (str_rules, str_cards) = str_plain
                .split(':')
                .collect_tuple()
                .ok_or_else(|| format_err!("':' does not separate rules from stichs."))?;
            let str_cards = str_cards.trim();
            let rules = crate::rules::parser::parse_rule_description_simple(str_rules)?;
            let veccard = parse_cards::<Vec<_>>(str_cards)
                .ok_or_else(|| format_err!("Could not parse cards: {}", str_cards))?;
            let stichseq = SStichSequence::new_from_cards(
                /*ekurzlang*/EKurzLang::values()
                    .find(|ekurzlang| ekurzlang.cards_per_player()*EPlayerIndex::SIZE==veccard.len())
                    .ok_or_else(|| format_err!("Incorrect number of cards: {}", veccard.len()))?,
                veccard.iter().copied(),
                rules.as_ref(),
            );
            SGame::new_finished(
                rules,
                SDoublings::new_full(
                    SStaticEPI0{},
                    EPlayerIndex::map_from_fn(|_epi| false).into_raw(),
                ),
                /*ostossparams*/None,
                /*vecstoss*/vec![],
                /*n_stock*/0,
                SStichSequenceGameFinished::new(&stichseq),
                /*fn_before_zugeben*/|_game, _i_stich, _epi, _card| {},
            )
        })
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let mut vecgame = Vec::new();
    if let Some(itstr_sauspiel_html_file) = clapmatches.values_of("sauspiel-files") {
        for str_file_sauspiel_html in itstr_sauspiel_html_file {
            for globresult in glob::glob(str_file_sauspiel_html)? {
                match globresult {
                    Ok(path) => {
                        println!("Opening {:?}", path);
                        let str_input = &via_out_param_result(|str_html|
                            std::fs::File::open(&path)?.read_to_string(str_html)
                        )?.0;
                        let mut b_found = false;
                        let mut push_game = |str_description, resgame: Result<_, _>| {
                            b_found = b_found || resgame.is_ok();
                            vecgame.push(SGameWithDesc{
                                str_description,
                                str_link: format!("file://{}", path.to_string_lossy()),
                                resgame,
                            });
                        };
                        if let resgame@Ok(_) = analyze_sauspiel_html(&str_input) {
                            push_game(path.to_string_lossy().into_owned(), resgame)
                        } else {
                            let mut b_found_plain = false;
                            for (i, resgame) in analyze_plain(&str_input).filter(|res| res.is_ok()).enumerate() {
                                b_found_plain = true;
                                push_game(format!("{}_{}", path.to_string_lossy(), i), resgame)
                            }
                            if !b_found_plain {
                                push_game(path.to_string_lossy().into_owned(), Err(format_err!("Nothing found in {:?}: Trying to continue.", path)));
                            }
                        }
                        if !b_found {
                            println!("Nothing found in {:?}: Trying to continue.", path);
                        }
                    },
                    Err(e) => {
                        println!("Error: {:?}. Trying to continue.", e);
                    },
                }
            }
        }
    }
    analyze_games(
        &std::path::Path::new("./analyze"), // TODO make customizable
        /*fn_link*/|str_description: &str| str_description.to_string(),
        vecgame.into_iter(),
    )
}
