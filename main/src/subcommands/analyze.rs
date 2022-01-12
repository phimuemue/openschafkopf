use crate::game_analysis::*;
use crate::game::*;
use crate::rules::ruleset::{SStossParams, VStockOrT};
use crate::primitives::*;
use crate::primitives::cardvector::*;
use crate::util::{*, parser::*};
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

#[derive(Debug)]
pub struct SSauspielAllowedRules {
    // Sauspiel, Solo, Wenz: implicitly allowed
    b_farbwenz: bool,
    b_geier: bool,
    b_ramsch: bool,
}

#[derive(Debug)]
pub enum VSauspielAllowedRules {
    Turnier(String),
    AllowedRules(SSauspielAllowedRules),
}

#[derive(Debug)]
pub struct SSauspielRuleset {
    allowedrules: VSauspielAllowedRules,
    n_tarif_extra: isize,
    n_tarif_ruf: isize,
    n_tarif_solo: isize,
    // TODO store ekurzlang explicitly?
}

#[derive(Debug)]
pub struct SGameAnnouncementAnonymous;

pub fn analyze_sauspiel_html(str_html: &str) -> Result<SGameResultGeneric<SSauspielRuleset, SGameAnnouncementsGeneric<SGameAnnouncementAnonymous>, Vec<(EPlayerIndex, &'static str)>>, failure::Error> {
    // TODO acknowledge timeouts
    use combine::{char::*, *};
    use select::{document::Document, node::Node, predicate::*};
    let doc = Document::from(str_html);
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
    // TODO ensure that "key_figure_table" looks exactly as we expect
    let scrape_from_key_figure_table = |str_key| -> Result<_, failure::Error> {
        doc.find(Name("th").and(|node: &Node| node.inner_html()==str_key))
            .exactly_one().map_err(|it| format_err!("Error with {}: no single <th>{}</th>: {} elements", str_key, str_key, it.count()))? // TODO could it implement Debug?
            .parent().ok_or_else(|| format_err!("Error with {}: {} has no parent", str_key, str_key))?
            .find(Name("td"))
            .exactly_one().map_err(|it| format_err!("Error with {}: no single <td> containing {}: {} elements", str_key, str_key, it.count())) // TODO could it implement Debug?
    };
    let ruleset = if let Ok(node_tarif) = scrape_from_key_figure_table("Tarif") {
        let (n_tarif_extra, n_tarif_ruf, n_tarif_solo) = {
            let str_tarif = node_tarif.inner_html();
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
        SSauspielRuleset{
            n_tarif_extra,
            n_tarif_ruf,
            n_tarif_solo,
            allowedrules: VSauspielAllowedRules::AllowedRules(scrape_from_key_figure_table("Sonderregeln")?
                .children()
                .filter(|node|
                    !matches!(node.data(), select::node::Data::Text(str_text) if str_text.trim().is_empty() || str_text.trim()!="-")
                )
                .try_fold(
                    SSauspielAllowedRules{
                        b_farbwenz: false,
                        b_geier: false,
                        b_ramsch: false,
                    },
                    |mut ruleset, node| {
                        if !matches!(node.data(), select::node::Data::Element(_,_)) {
                            return Err(format_err!("Unexpected data {:?} in Sonderregeln", node.data()));
                        } else if node.name()!=Some("img") {
                            return Err(format_err!("Unexpected name {:?} in Sonderregeln", node.name()));
                        } else if node.attr("class")!=Some("rules__rule") {
                            return Err(format_err!("Unexpected class {:?} in Sonderregeln", node.attr("class")));
                        } else if node.attr("alt")!=node.attr("title") {
                            return Err(format_err!("alt {:?} differs from title {:?} in Sonderregeln", node.attr("alt"), node.attr("title")));
                        } else {
                            match node.attr("title") {
                                Some("Kurze Karte") => {/* TODO assert/check consistency */},
                                Some("Farbwenz") => ruleset.b_farbwenz = true,
                                Some("Geier") => ruleset.b_geier = true,
                                Some("Ramsch") => ruleset.b_ramsch = true,
                                _ => {
                                    return Err(format_err!("Unknown Sonderregeln: {:?}", node.attr("title")));
                                }
                            }
                        }
                        Ok(ruleset)
                    },
                )?
            )
        }
    } else if let Ok(node_turnier) = scrape_from_key_figure_table("Turnier") {
        SSauspielRuleset{
            // TODO tarif is tricky, as it might also be paid.
            n_tarif_extra: 1,
            n_tarif_ruf: 1,
            n_tarif_solo: 2,
            allowedrules: VSauspielAllowedRules::Turnier(node_turnier.inner_html()),
        }
    } else {
        return Err(format_err!("Ruleset"));
    };
    let orules = doc.find(Class("title-supertext"))
        .exactly_one()
        .map_err(|it| format_err!("title-supertext single failed {} elements", it.count()))? // TODO could it implement Debug?
        .parent().ok_or_else(|| format_err!("title-supertext has no parent"))?
        .find(Name("h1"))
        .exactly_one()
        .map_err(|it| format_err!("h1 is not single: {} elements", it.count())) // TODO could it implement Debug?
        .and_then(|node_rules| {
            if let Ok(rules) = crate::rules::parser::parse_rule_description(
                &node_rules.text(),
                (ruleset.n_tarif_extra, ruleset.n_tarif_ruf, ruleset.n_tarif_solo),
                /*fn_player_to_epi*/username_to_epi,
            ) {
                Ok(Some(rules))
            } else if node_rules.text()=="Zamgworfen" {
                Ok(None)
            } else {
                Err(format_err!("Could not parse rules"))
            }
        })?;
    fn get_cards<T>(
        node: &Node,
        fn_card_highlight: impl Fn(SCard, Option<&str>)->T
    ) -> Result<Vec<T>, failure::Error> {
        node
            .find(Class("card-image"))
            .map(|node_card| -> Result<T, _> {
                let str_class = unwrap!(node_card.attr("class")); // "class" must be present
                (
                    string("card-image "),
                    choice!(string("by"), string("fn")),
                    string(" g"),
                    digit(),
                    space(),
                )
                .with((
                    card_parser(),
                    optional(string(" highlight")),
                ))
                .skip(eof())
                    // end of parser
                    .parse(str_class)
                    .map_err(|err| format_err!("Card parsing: {:?} on {}", err, str_class))
                    .map(|((card, ostr_highlight), _str)| fn_card_highlight(card, ostr_highlight))
            })
            .collect::<Result<Vec<_>,_>>()
    }
    let aveccard = vec_to_arr(
        doc.find(|node: &Node| node.inner_html()=="Karten von:")
            .try_fold(Vec::new(), |mut vecveccard, node| -> Result<_, failure::Error> {
                let mut veccardb = get_cards(
                    &node
                        .parent().ok_or_else(|| format_err!(r#""Karten von:" has no parent"#))?
                        .parent().ok_or_else(|| format_err!("walking html failed"))?,
                    /*fn_card_highlight*/|card, ostr_highlight| (card, ostr_highlight.is_some()),
                )?;
                veccardb.sort_unstable_by_key(|&(_card, b_highlight)| !b_highlight);
                vecveccard.push(veccardb.into_iter().map(|(card, _b_highlight)| card).collect());
                Ok(vecveccard)
        })?
    ).map(EPlayerIndex::map_from_raw)?;
    let get_doublings_stoss = |str_key| -> Result<_, failure::Error> {
        Ok(scrape_from_key_figure_table(str_key)?
            .find(Name("a"))
            .map(|node| username_to_epi(&node.inner_html())))
    };
    let doublings = {
        let vecepi_doubling = get_doublings_stoss("Klopfer")?.collect::<Result<Vec<_>, _>>()?;
        SDoublings::new_full(
            SStaticEPI0{},
            EPlayerIndex::map_from_fn(|epi| 
                vecepi_doubling.contains(&epi)
            ).into_raw(),
        )
    };
    let username_parser = |epi| {
        combine::tokens2(|l,r|l==r, mapepistr_username[epi].chars()) // TODO? can we use combine::char::string?
            .map(move |mut str_username| verify_eq!(epi, unwrap!(username_to_epi(&str_username.join("")))))
    };
    let mut itnode_gameannouncement = ((((doc.find(Name("h4").and(|node: &Node| node.inner_html()=="Spielermittlung"))
        .exactly_one()
        .map_err(|it| format_err!("error on single: {} elements", it.count())))? // TODO could it implement Debug?
        .parent().ok_or_else(|| format_err!("Spielermittlung has no parent")))?
        .parent().ok_or_else(|| format_err!("Spielermittlung parent has no parent")))?
        .find(Class("card-rows"))
        .exactly_one()
        .map_err(|it| format_err!("error on single: {} elements", it.count())))? // TODO could it implement Debug?
        .find(Class("card-row"));
    let gameannouncements = SGameAnnouncementsGeneric::new_full(
        SStaticEPI0{},
        vec_to_arr(
            EPlayerIndex::values().zip(itnode_gameannouncement.by_ref())
                .map(|(epi, node_gameannouncement)| -> Result<_, _> {
                    parse_trimmed(
                        node_gameannouncement.inner_html().trim(), // trim to avoid newlines // TODO move newlines into parser
                        "gameannouncement 1",
                        (
                            username_parser(epi),
                            newline(),
                            spaces(),
                        )
                            .with(choice!(
                                (string("sagt weiter."), optional((newline(), spaces(), string("(timeout)"))))
                                    .map(|_| None),
                                (string("dad gern."))
                                    .map(|_| Some(SGameAnnouncementAnonymous))
                            ))
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
        )?
    );
    let vecvectplepistr_determinerules = itnode_gameannouncement
        .map(|node_gameannouncement| {
            parse_trimmed(
                node_gameannouncement.inner_html().trim(), // TODO move newlines to parser
                "gameannouncement 2",
                choice(EPlayerIndex::map_from_fn(
                    |epi| attempt((
                        username_parser(epi),
                        choice((
                            attempt(string(" h\u{00E4}tt a Sauspiel")),
                            attempt(string(" h\u{00E4}tt a Solo-Tout")),
                            attempt(string(" h\u{00E4}tt a Solo")),
                            attempt(string(" h\u{00E4}tt an Wenz-Tout")),
                            attempt(string(" h\u{00E4}tt an Wenz")),
                            attempt(string(" h\u{00E4}tt an Farbwenz-Tout")),
                            attempt(string(" h\u{00E4}tt an Farbwenz")),
                            attempt(string(" h\u{00E4}tt an Geier-Tout")),
                            attempt(string(" h\u{00E4}tt an Geier")),
                            attempt(string(" spielt auf die Alte")),
                            attempt(string(" spielt auf die Blaue")),
                            attempt(string(" spielt auf die Hundsgfickte")),
                            attempt(string(" spielt Eichel")),
                            attempt(string(" spielt Gras")),
                            attempt(string(" spielt Herz")),
                            attempt(string(" spielt Schelle")),
                            attempt(string(" l\u{00E4}sst den Vortritt.")),
                        ))
                            .skip(optional(string(" (timeout)"))),
                    ))
                ).into_raw()),
            )
        })
        .collect::<Result<Vec<(EPlayerIndex, &'static str)>, _>>()?;
    if let Some(rules) = orules {
        let vecstich = doc.find(|node: &Node| node.inner_html()=="Stich von")
            .try_fold((EPlayerIndex::EPI0, Vec::new()), |(epi_first, mut vecstich), node| -> Result<_, _> {
                vec_to_arr(get_cards(
                    &node.parent().ok_or_else(|| format_err!(r#""Stich von" has no parent"#))?
                        .parent().ok_or_else(|| format_err!("walking html failed"))?,
                    /*fn_card_highlight*/|card, _ostr_highlight| card,
                )?).map(|acard| {
                    let stich = SStich::new_full(epi_first, acard);
                    let epi_winner = rules.winner_index(&stich);
                    vecstich.push(stich);
                    (epi_winner, vecstich)
                })
            })?
            .1;
        let mut game = SGameGeneric::new_with(
            aveccard,
            doublings,
            /*ostossparams*/Some(SStossParams::new(/*n_stoss_max*/4)), // TODO? is this correct
            rules,
            /*n_stock*/0, // Sauspiel does not support stock
            ruleset,
            gameannouncements,
            vecvectplepistr_determinerules,
        );
        for resepi in get_doublings_stoss("Kontra und Retour")? {
            let () = game.stoss(resepi?)?;
        }
        for stich in vecstich.into_iter() {
            for (epi, card) in stich.iter() {
                let () = game.zugeben(*card, epi)?;
            }
        }
        game.finish().map_err(|_game| format_err!("Could not game.finish"))
    } else {
        // TODO assert that there are actually no stichs in doc
        Ok(SGameResultGeneric {
            an_payout: EPlayerIndex::map_from_fn(|_epi| /*Sauspiel does not know stock*/0),
            stockorgame: VStockOrT::Stock(()),
        })
    }
}

pub fn analyze_plain(str_lines: &str) -> impl Iterator<Item=Result<SGame, failure::Error>> + std::fmt::Debug + '_ {
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
                if_then_some!(veccard.len()%EPlayerIndex::SIZE==0, veccard.len()/EPlayerIndex::SIZE)
                    .and_then(EKurzLang::checked_from_cards_per_player)
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

#[test]
fn test_analyze_plain() {
    fn internal_test(str_in: &str) {
        unwrap!(unwrap!(analyze_plain(str_in).exactly_one()));
    }
    internal_test("Rufspiel Blaue von 3: so h7 go eo ho hz hk eu gu h9 su g8 g9 ga gk e9 ea ek ez e7 g7 ha s7 gz sa s9 h8 sz e8 sk hu s8");
    internal_test("Schelln-Wenz von 2: ea ek e7 ez gz g7 ga go eu e9 so s9 gu h7 sa hu su h8 e8 sz s8 ha eo g9 s7 h9 hk g8 sk hz ho gk");
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let mut vecgame = Vec::new();
    super::glob_files(
        unwrap!(clapmatches.values_of("sauspiel-files")),
        |path, str_input| {
            println!("Opened {:?}", path);
            let mut b_found = false;
            let mut push_game = |str_description, resgameresult: Result<_, _>| {
                b_found = b_found || resgameresult.is_ok();
                vecgame.push(SGameWithDesc{
                    str_description,
                    str_link: format!("file://{}", path.to_string_lossy()),
                    resgameresult,
                });
            };
            if let resgameresult@Ok(_) = analyze_sauspiel_html(&str_input) {
                push_game(
                    path.to_string_lossy().into_owned(),
                    resgameresult.map(|game| game.map(|_|(), |_|(), |_|()))
                )
            } else {
                let mut b_found_plain = false;
                for (i, resgame) in analyze_plain(&str_input).filter(|res| res.is_ok()).enumerate() {
                    b_found_plain = true;
                    push_game(
                        format!("{}_{}", path.to_string_lossy(), i),
                        resgame.and_then(|game| game.finish().map_err(|_game| format_err!("Could not game.finish")))
                    )
                }
                if !b_found_plain {
                    push_game(path.to_string_lossy().into_owned(), Err(format_err!("Nothing found in {:?}: Trying to continue.", path)));
                }
            }
            if !b_found {
                eprintln!("Nothing found in {:?}: Trying to continue.", path);
            }
        },
    )?;
    analyze_games(
        std::path::Path::new("./analyze"), // TODO make customizable
        /*fn_link*/|str_description: &str| str_description.to_string(),
        vecgame.into_iter(),
    )
}
