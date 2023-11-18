use crate::game_analysis::*;
use crate::rules::{
    SStossParams,
    ruleset::{TRuleSet, VStockOrT},
    parser::parse_rule_description,
};
use crate::primitives::cardvector::*;
use itertools::Itertools;
use combine::{char::*, *};

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
pub struct SSauspielRuleset { // TODO can we represent this as a plain SRuleSet?
    ekurzlang: EKurzLang,
    #[allow(dead_code)] // TODO
    allowedrules: VSauspielAllowedRules,
    n_tarif_extra: isize,
    n_tarif_ruf: isize,
    n_tarif_solo: isize,
}

impl TRuleSet for SSauspielRuleset {
    fn kurzlang(&self) -> EKurzLang {
        self.ekurzlang
    }
}

#[derive(Debug)]
pub struct SGameAnnouncementAnonymous;

fn vec_to_arr<T: std::fmt::Debug>(vect: Vec<T>) -> Result<[T; EPlayerIndex::SIZE], failure::Error> {
    let (card0, card1, card2, card3) = vect.into_iter()
        .collect_tuple()
        .ok_or_else(|| format_err!("Wrong number of elements"))?;
    Ok([card0, card1, card2, card3])
}

pub fn analyze_sauspiel_html(str_html: &str) -> Result<SGameResultGeneric<SSauspielRuleset, SGameAnnouncementsGeneric<SGameAnnouncementAnonymous>, Vec<(EPlayerIndex, &'static str)>>, failure::Error> {
    // TODO acknowledge timeouts
    use select::{document::Document, node::Node, predicate::*};
    let doc = Document::from(str_html);
    let mapepistr_username = vec_to_arr(
        doc.find(Class("game-participants"))
            .exactly_one()
            .map_err(|it| format_err!("{:?}", it))?
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
            .exactly_one().map_err(|it| format_err!("{:?}", it))?
            .parent().ok_or_else(|| format_err!("Error with {}: {} has no parent", str_key, str_key))?
            .find(Name("td"))
            .exactly_one().map_err(|it| format_err!("{:?}", it))
    };
    fn get_cards<T>(
        node: &Node,
        fn_card_highlight: impl Fn(ECard, Option<&str>)->T
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
                vecveccard.push(SHandVector::try_from(
                    veccardb.into_iter()
                        .map(|(card, _b_highlight)| card)
                        .collect::<Vec<_>>()
                        .as_slice()
                )?);
                Ok(vecveccard)
        })?
    ).map(EPlayerIndex::map_from_raw)?;
    let ekurzlang = aveccard.iter()
        .map(|veccard| EKurzLang::from_cards_per_player(veccard.len()))
        .all_equal_value()
        .map_err(|e| format_err!("Not all players have the same number of cards: {:?}", e))?
        .ok_or(format_err!("Could not determine ekurzlang"))?;
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
                .map_err(|err| format_err!("Failed to parse tarif: {:?}", err))? // unpack result of combine::parse call
                ? // unpack parsed result
        };
        SSauspielRuleset{
            ekurzlang,
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
                                Some("Kurze Karte") => {
                                    if ekurzlang!=EKurzLang::Kurz {
                                        return Err(format_err!("Contradicting kurz/lang values."));
                                    }
                                },
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
            ekurzlang,
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
        .map_err(|it| format_err!("{:?}", it))?
        .parent().ok_or_else(|| format_err!("title-supertext has no parent"))?
        .find(Name("h1"))
        .exactly_one()
        .map_err(|it| format_err!("{:?}", it))
        .and_then(|node_rules| {
            if let Ok(rules) = parse_rule_description(
                &node_rules.text(),
                (ruleset.n_tarif_extra, ruleset.n_tarif_ruf, ruleset.n_tarif_solo),
                SStossParams::new(/*n_stoss_max*/4), // TODO? is this correct
                /*fn_player_to_epi*/username_to_epi,
            ) {
                Ok(Some(rules))
            } else if node_rules.text()=="Zamgworfen" {
                Ok(None)
            } else {
                Err(format_err!("Could not parse rules"))
            }
        })?;
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
        tokens2(|l,r|l==r, mapepistr_username[epi].chars()) // TODO? can we use combine::char::string?
            .map(move |mut str_username| verify_eq!(epi, unwrap!(username_to_epi(&str_username.join("")))))
    };
    let mut itnode_gameannouncement = ((((doc.find(Name("h4").and(|node: &Node| node.inner_html()=="Spielermittlung"))
        .exactly_one()
        .map_err(|it| format_err!("{:?}", it)))?
        .parent().ok_or_else(|| format_err!("Spielermittlung has no parent")))?
        .parent().ok_or_else(|| format_err!("Spielermittlung parent has no parent")))?
        .find(Class("card-rows"))
        .exactly_one()
        .map_err(|it| format_err!("{:?}", it)))?
        .find(Class("card-row"));
    let gameannouncements = SGameAnnouncementsGeneric::new_full(
        SStaticEPI0{},
        vec_to_arr(
            EPlayerIndex::values().zip(itnode_gameannouncement.by_ref())
                .map(|(epi, node_gameannouncement)| -> Result<_, _> {
                    parse_trimmed(
                        node_gameannouncement.inner_html().trim(), // trim to avoid newlines // TODO move newlines into parser
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
                    ).map_err(|err| format_err!("Failed to parse game announcement 1: {:1}", err))
                })
                .collect::<Result<Vec<_>, _>>()?
        )?
    );
    let vecvectplepistr_determinerules = itnode_gameannouncement
        .map(|node_gameannouncement| {
            parse_trimmed(
                node_gameannouncement.inner_html().trim(), // TODO move newlines to parser
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
            ).map_err(|err| format_err!("Failed to parse game announcement 2: {:?}", err))
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
                    let stich = SFullStich::new(SStich::new_full(epi_first, acard));
                    let epi_winner = rules.winner_index(stich.as_ref());
                    vecstich.push(stich);
                    (epi_winner, vecstich)
                })
            })?
            .1;
        if vecstich.len()!=ekurzlang.cards_per_player() {
            return Err(format_err!("Contradicting kurz/lang values."));
        }
        let mut game = SGameGeneric::new_with(
            aveccard,
            SExpensifiersNoStoss::new_with_doublings(/*n_stock: Sauspiel does not support Stock*/0, doublings),
            rules,
            ruleset,
            gameannouncements,
            vecvectplepistr_determinerules,
        );
        for resepi in get_doublings_stoss("Kontra und Retour")? {
            verify_is_unit!(game.stoss(resepi?)?);
        }
        for stich in vecstich.into_iter() {
            for (epi, card) in stich.iter() {
                verify_is_unit!(game.zugeben(*card, epi)?);
            }
        }
        game.finish().map_err(|_game| format_err!("Could not game.finish"))
    } else {
        // TODO assert that there are actually no stichs in doc
        Ok(SGameResultGeneric {
            an_payout: EPlayerIndex::map_from_fn(|_epi| /*Sauspiel does not know stock*/0),
            stockorgame: VStockOrT::Stock(ruleset),
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
                    .and_then(EKurzLang::from_cards_per_player)
                    .ok_or_else(|| format_err!("Incorrect number of cards: {}", veccard.len()))?,
                veccard.iter().copied(),
                &rules,
            );
            SGame::new_finished(
                rules,
                SExpensifiers::new_no_stock_doublings_stoss(),
                SStichSequenceGameFinished::new(&stichseq),
            )
        })
}

pub fn analyze_netschafkopf(str_lines: &str) -> Result<Vec<Result<SGameResult</*Ruleset*/()>, failure::Error>>, failure::Error> {
    let mut itstr_line = str_lines.lines();
    itstr_line.next().ok_or_else(|| format_err!("First line should contain rules"))?;
    itstr_line.next()
        .filter(|str_gespielt_von| str_gespielt_von.starts_with("gespielt von")) // TODO be more precise?
        .ok_or_else(|| format_err!("Expected 'gespielt von'."))?;
    Ok(itstr_line
        .group_by(|str_line| str_line.trim().is_empty())
        .into_iter()
        .filter(|(b_is_empty, _grpstr_line)| !b_is_empty)
        .map(|(_b_is_empty, grpstr_line)| -> Result<_, _> {
            let mut grpstr_line = grpstr_line.peekable();
            grpstr_line.next()
                .filter(|str_geber| str_geber.starts_with("Geber: ")) // TODO be more precise?
                .ok_or_else(|| format_err!("Expected 'Geber: '"))?;
            if Some(&"Spielart: Schieber")==dbg!(grpstr_line.peek()) {
                grpstr_line.next();
            }
            let mut vecstr_player_name = Vec::<String>::new();
            for _epi in EPlayerIndex::values() {
                vecstr_player_name.push(
                    grpstr_line.next()
                        .ok_or_else(|| format_err!("Expected description of player's hand"))
                        .and_then(|str_player_hand| {
                            parse_trimmed(
                                str_player_hand,
                                attempt(many1::<String,_>(alpha_num())) // TODO allow more characters for player names
                                    .skip((
                                        string(" hat: "),
                                        sep_by::<Vec<_>,_,_>(
                                            choice!(card_parser().map(|_|()), string("DU").map(|_|())), // TODO be more precise
                                            char(' ')
                                        ), // TODO determine ekurzlang?
                                    ))
                            ).map_err(|err| format_err!("Failed to parse <player> hat <hand>: {:?}", err))
                        })?
                        .to_string()
                );
            }
            let ekurzlang = EKurzLang::Lang; // TODO does NetSchafkopf support EKurzLang::Kurz?
            let mapepistr_player = EPlayerIndex::map_from_raw(unwrap!(vec_to_arr(vecstr_player_name)));
            let player_to_epi = |str_player: &str| {
                EPlayerIndex::values()
                    .find(|epi| mapepistr_player[*epi]==str_player)
                    .ok_or_else(|| format_err!("player {} not part of mapepistr_player {:?}", str_player, mapepistr_player))
            };
            let username_parser = |epi: EPlayerIndex| {
                tokens2(|l,r|l==r, mapepistr_player[epi].chars()) // TODO? can we use combine::char::string?
                    .map(move |mut str_player| verify_eq!(epi, unwrap!(player_to_epi(&str_player.join("")))))
            };
            if Some(&"Es wurde zusammengeworfen.")==grpstr_line.peek() {
                Ok(SGameResultGeneric {
                    an_payout: EPlayerIndex::map_from_fn(|_epi| 0), // TODO could there be stock?
                    stockorgame: VStockOrT::Stock(()),
                })
            } else {
                let rules = parse_rule_description(
                    grpstr_line.next().ok_or_else(|| format_err!("Expected rules"))?,
                    (/*n_tarif_extra*/10, /*n_tarif_ruf*/20, /*n_tarif_solo*/50), // TODO? make adjustable
                    SStossParams::new(/*n_stoss_max*/4), // TODO? support
                    /*fn_player_to_epi*/player_to_epi,
                )?;
                let mut stichseq = SStichSequence::new(ekurzlang);
                for _i_stich in 0..ekurzlang.cards_per_player() {
                    let (_epi, veccard) = parse_trimmed(
                        grpstr_line.next().ok_or_else(||format_err!("Expected stich"))?,
                        choice::<[_; EPlayerIndex::SIZE]>(EPlayerIndex::map_from_fn(|epi|
                            attempt((
                                username_parser(epi).skip(string(" spielt aus: ")),
                                sep_by::<Vec<_>,_,_>(card_parser(), char(' ')),
                            ))
                        ).into_raw()),
                    ).map_err(|err| format_err!("Failed to parse stich {:?}", err))?;
                    for card in veccard {
                        stichseq.zugeben(card, &rules);
                    }
                }
                SGame::new_finished(
                    rules,
                    SExpensifiers::new_no_stock_doublings_stoss(), // TODO? support
                    SStichSequenceGameFinished::new(&stichseq),
                )
                    .and_then(|game| game.finish()
                        .map_err(|err| format_err!("Could not finish game: {:?}", err))
                    )
            }
        })
        // TODO is the following needed?
        .collect::<Vec<_>>()
    )
}

#[test]
fn test_parse_netschafkopf() {
    fn test_internal(slcu8_netschafkopf: &[u8]) {
        for resgame in unwrap!(analyze_netschafkopf(&String::from_utf8_lossy(slcu8_netschafkopf))) {
            unwrap!(resgame);
        }
    }
    test_internal(include_bytes!("Schafkopfprotokoll_vom_14.12.22.txt"));
    test_internal(include_bytes!("Schafkopfprotokoll_vom_16.05.20.txt"));
    test_internal(include_bytes!("Schafkopfprotokoll_vom_20.03.23.txt"));
}

#[test]
fn test_analyze_plain() {
    fn internal_test(str_in: &str) {
        unwrap!(unwrap!(analyze_plain(str_in).exactly_one()));
    }
    internal_test("Rufspiel Blaue von 3: so h7 go eo ho hz hk eu gu h9 su g8 g9 ga gk e9 ea ek ez e7 g7 ha s7 gz sa s9 h8 sz e8 sk hu s8");
    internal_test("Schelln-Wenz von 2: ea ek e7 ez gz g7 ga go eu e9 so s9 gu h7 sa hu su h8 e8 sz s8 ha eo g9 s7 h9 hk g8 sk hz ho gk");
}
