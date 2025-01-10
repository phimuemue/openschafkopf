use crate::game_analysis::*;
use crate::rules::{
    SStossParams,
    ruleset::{TRuleSet, VStockOrT},
    parser::parse_rule_description,
};
use crate::primitives::cardvector::*;
use itertools::Itertools;
use combine::{char::*, *};
use std::fmt::Debug;
use serde::Deserialize;
use serde_repr::Deserialize_repr;

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

fn iter_to_arr<T>(it: impl IntoIterator<Item=T>) -> Result<[T; EPlayerIndex::SIZE], failure::Error> {
    let (card0, card1, card2, card3) = it.into_iter()
        .collect_tuple()
        .ok_or_else(|| format_err!("Wrong number of elements"))?;
    Ok([card0, card1, card2, card3])
}

pub trait TSauspielHtmlDocument : Debug {
    type HtmlNode<'node>: TSauspielHtmlNode<'node> + 'node
        where Self: 'node;
    fn find_class<'slf>(&'slf self, str_class: &'static str) -> impl Debug+Iterator<Item=Self::HtmlNode<'slf>>+'slf;
    fn find_name(&self, str_name: &'static str) -> impl Debug+Iterator<Item=Self::HtmlNode<'_>> + '_;
    fn find_inner_html<'slf>(&'slf self, str_inner_html: &str) -> impl Debug+Iterator<Item=Self::HtmlNode<'slf>>;
}

use select::{document::Document, predicate::{Class, Name}};
impl TSauspielHtmlDocument for Document {
    type HtmlNode<'node> = select::node::Node<'node>;
    fn find_class<'slf>(&'slf self, str_class: &'static str) -> impl Debug+Iterator<Item=Self::HtmlNode<'slf>>+'slf {
        self.find(Class(str_class))
    }
    fn find_name(&self, str_name: &'static str) -> impl Debug+Iterator<Item=Self::HtmlNode<'_>> + '_ {
        self.find(Name(str_name))
    }
    fn find_inner_html<'slf>(&'slf self, str_inner_html: &str) -> impl Debug+Iterator<Item=Self::HtmlNode<'slf>> {
        self.find(move |node: &select::node::Node| node.inner_html()==str_inner_html)
    }
}

pub trait TSauspielHtmlNode<'node> : Debug + Sized + 'node {
    fn find_name(&self, str_name: &'static str) -> impl Debug+Iterator<Item=Self>;
    fn find_attr(&self, str_attr: &str, attr: ()) -> impl Debug+Iterator<Item=Self>;
    fn find_class(&self, str_class: &'static str) -> impl Debug+Iterator<Item=Self>;
    fn attr(&self, str_attr: &str) -> Option<String>;
    fn parent(&self) -> Option<Self>;
    fn inner_html(&self) -> String;
    fn children(&self) -> impl Debug+Iterator<Item=Self>;
    fn data(&self) -> VSauspielHtmlData;
    fn name(&self) -> Option<String>;
    fn text(&self) -> String;
}

#[derive(Debug)]
pub enum VSauspielHtmlData {
    Element,
    Text(String),
    Comment,
}

impl<'node> TSauspielHtmlNode<'node> for select::node::Node<'node> {
    fn find_name(&self, str_name: &'static str) -> impl Debug+Iterator<Item=Self> {
        self.find(Name(str_name))
    }
    fn find_attr(&self, str_attr: &str, attr: ()) -> impl Debug+Iterator<Item=Self> {
        self.find(select::predicate::Attr(str_attr, attr))
    }
    fn find_class(&self, str_class: &'static str) -> impl Debug+Iterator<Item=Self> {
        self.find(Class(str_class))
    }
    fn attr(&self, str_attr: &str) -> Option<String> {
        self.attr(str_attr).map(String::from)
    }
    fn parent(&self) -> Option<Self> {
        self.parent()
    }
    fn inner_html(&self) -> String {
        self.inner_html()
    }
    fn children(&self) -> impl Debug+Iterator<Item=Self> {
        self.children()
    }
    fn data(&self) -> VSauspielHtmlData {
        match self.data() {
            select::node::Data::Text(str_text) => VSauspielHtmlData::Text(str_text.to_string()),
            select::node::Data::Element(_,_) => VSauspielHtmlData::Element,
            select::node::Data::Comment(_) => VSauspielHtmlData::Comment,
        }
    }
    fn name(&self) -> Option<String> {
        self.name().map(String::from)
    }
    fn text(&self) -> String {
        self.text()
    }
}

pub fn analyze_sauspiel_html(str_html: &str) -> Result<SGameResultGeneric<SSauspielRuleset, Option<SGameAnnouncementAnonymous>, Vec<()>>, failure::Error> {
    internal_analyze_sauspiel_html(
        Document::from(str_html),
        /*fn_determinerules_step*/|_,_,_| (),
        /*fn_before_play_card*/|_,_,_,_| (),
    )
}

pub fn internal_analyze_sauspiel_html<Document: TSauspielHtmlDocument, DetermineRulesStep, FnDetermineRulesStep, FnBeforePlayCard>(
    doc: Document,
    mut fn_determinerules_step: FnDetermineRulesStep,
    mut fn_before_play_card: FnBeforePlayCard,
) -> Result<SGameResultGeneric<SSauspielRuleset, Option<SGameAnnouncementAnonymous>, Vec<DetermineRulesStep>>, failure::Error>
    where
        for <'card> FnDetermineRulesStep: FnMut(EPlayerIndex, &str, Document::HtmlNode<'card>)->DetermineRulesStep,
        for <'card> FnBeforePlayCard: FnMut(&SGameGeneric<SSauspielRuleset, Option<SGameAnnouncementAnonymous>, Vec<DetermineRulesStep>>, ECard, EPlayerIndex, Document::HtmlNode<'card>),
{
    // TODO acknowledge timeouts
    let mapepistr_username = iter_to_arr(
        doc.find_class("game-participants")
            .exactly_one()
            .map_err(|it| format_err!("{:?}", it))?
            .find_attr("data-username", ())
            .map(|node_username| unwrap!(node_username.attr("data-username")))
    ).map(EPlayerIndex::map_from_raw)?;
    let username_to_epi = |str_username: &str| -> Result<EPlayerIndex, failure::Error> {
        EPlayerIndex::values()
            .find(|epi| mapepistr_username[*epi]==str_username)
            .ok_or_else(|| format_err!("username {} not part of mapepistr_username {:?}", str_username, mapepistr_username))
    };
    // TODO ensure that "key_figure_table" looks exactly as we expect
    let scrape_from_key_figure_table = |str_key| -> Result<_, failure::Error> {
        doc.find_name("th")
            .filter(|node| node.inner_html()==str_key)
            .exactly_one().map_err(|it| format_err!("{:?}", it))?
            .parent().ok_or_else(|| format_err!("Error with {}: {} has no parent", str_key, str_key))?
            .find_name("td")
            .exactly_one().map_err(|it| format_err!("{:?}", it))
    };
    fn get_cards<'node, HtmlNode: TSauspielHtmlNode<'node>>(
        node: &HtmlNode,
    ) -> Result<Vec<(ECard, bool/*b_highlight*/, HtmlNode)>, failure::Error> {
        node
            .find_class("card-image")
            .map(|node_card| -> Result<_, _> {
                let str_class = unwrap!(node_card.attr("class")); // "class" must be present
                let res = (
                    string("card-image "),
                    choice!(string("by"), string("fn")),
                    string(" g"),
                    digit(),
                    space(),
                )
                .with((
                    card_parser(),
                    optional(string(" highlight"))
                        .map(|ostr_highlight| ostr_highlight.is_some()),
                ))
                .skip(eof())
                    // end of parser
                    .parse(str_class.as_str())
                    .map_err(|err| format_err!("Card parsing: {:?} on {}", err, str_class))
                    .map(|((card, b_highlight), _str)| (card, b_highlight, node_card));
                res
            })
            .collect::<Result<Vec<_>,_>>()
    }
    let aveccard = iter_to_arr(
        doc.find_inner_html("Karten von:")
            .try_fold(Vec::new(), |mut vecveccard, node| -> Result<_, failure::Error> {
                let mut veccardbnode = get_cards(
                    &node
                        .parent().ok_or_else(|| format_err!(r#""Karten von:" has no parent"#))?
                        .parent().ok_or_else(|| format_err!("walking html failed"))?,
                )?;
                veccardbnode.sort_unstable_by_key(|&(_card, b_highlight, ref _node_card)| !b_highlight);
                vecveccard.push(SHandVector::try_from(
                    veccardbnode.into_iter()
                        .map(|(card, _b_highlight, _node_card)| card)
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
                    !matches!(node.data(), VSauspielHtmlData::Text(str_text) if str_text.trim().is_empty() || str_text.trim()!="-")
                )
                .try_fold(
                    SSauspielAllowedRules{
                        b_farbwenz: false,
                        b_geier: false,
                        b_ramsch: false,
                    },
                    |mut ruleset, node| {
                        if !matches!(node.data(), VSauspielHtmlData::Element) {
                            return Err(format_err!("Unexpected data {:?} in Sonderregeln", node.data()));
                        } else if node.name()!=Some("img".into()) && node.name()!=Some("IMG".into()) {
                            return Err(format_err!("Unexpected name {:?} in Sonderregeln", node.name()));
                        } else if node.attr("class")!=Some("rules__rule".into()) {
                            return Err(format_err!("Unexpected class {:?} in Sonderregeln", node.attr("class")));
                        } else if node.attr("alt")!=node.attr("title") {
                            return Err(format_err!("alt {:?} differs from title {:?} in Sonderregeln", node.attr("alt"), node.attr("title")));
                        } else {
                            match node.attr("title").as_deref() {
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
    let orules = doc.find_class("title-supertext")
        .exactly_one()
        .map_err(|it| format_err!("{:?}", it))?
        .parent().ok_or_else(|| format_err!("title-supertext has no parent"))?
        .find_name("h1")
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
    let get_doublings_stoss = |str_key: &'static str| -> Result<_, failure::Error> {
        scrape_from_key_figure_table(str_key)?
            .find_name("a")
            .map(|node| username_to_epi(&node.inner_html()))
            .collect::<Result<Vec<_>, _>>() // TODO avoid this
    };
    let doublings = {
        let vecepi_doubling = get_doublings_stoss("Klopfer")?;
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
    let node_card_rows = ((((doc.find_name("h4")
        .filter(|node| node.inner_html()=="Spielermittlung")
        .exactly_one()
        .map_err(|it| format_err!("{:?}", it)))?
        .parent().ok_or_else(|| format_err!("Spielermittlung has no parent")))?
        .parent().ok_or_else(|| format_err!("Spielermittlung parent has no parent")))?
        .find_class("card-rows")
        .exactly_one()
        .map_err(|it| format_err!("{:?}", it)))?;
    let mut itnode_gameannouncement = node_card_rows
        .find_class("card-row");
    let mapepigameannouncement = iter_to_arr(
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
    ).map(EPlayerIndex::map_from_raw)?;
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
            .map(|(epi, str_determinerules)| fn_determinerules_step(epi, str_determinerules, node_gameannouncement))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(rules) = orules {
        let mut game = SGameGeneric::new_with(
            aveccard,
            SExpensifiersNoStoss::new_with_doublings(/*n_stock: Sauspiel does not support Stock*/0, doublings),
            rules,
            ruleset,
            mapepigameannouncement,
            vecvectplepistr_determinerules,
        );
        for epi in get_doublings_stoss("Kontra und Retour")? {
            verify_is_unit!(game.stoss(epi)?);
        }
        for node_stich in doc.find_inner_html("Stich von") {
            for (card, _b_highlight, node_card) in get_cards(
                &node_stich.parent().ok_or_else(|| format_err!(r#""Stich von" has no parent"#))?
                    .parent().ok_or_else(|| format_err!("walking html failed"))?,
            )? {
                let epi_zugeben = game.which_player_can_do_something()
                    .map(|(epi_zugeben, _vecepi_stoss)| epi_zugeben)
                    .ok_or_else(|| format_err!("which_player_can_do_something"))?;
                fn_before_play_card(&game, card, epi_zugeben, node_card);
                verify_is_unit!(game.zugeben(card, epi_zugeben)?);
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

plain_enum_mod!(modesauspielposition, derive(Deserialize_repr,), map_derive(), ESauspielPosition {_0, _1, _2, _3,});

pub fn analyze_sauspiel_json(
    str_json: &str,
    fn_before_zugeben: impl FnMut(&SGameGeneric<EKurzLang, (), ()>, /*i_stich*/usize, EPlayerIndex, ECard),
) -> Result<SGameResultGeneric</*Ruleset*/EKurzLang, (), ()>, failure::Error> {
    #[derive(Deserialize, Debug)]
    #[serde(tag = "type")]
    #[allow(non_camel_case_types, non_snake_case)] // to match Sauspiel JSON
    #[allow(dead_code)] // to find unexpected errors in matching
    enum VSauspielJSONEvent {
        yourAuthenticationSucceeded {
            // position: ESauspielPosition,
            // userName: String,
            // playInstant: bool,
            // userID: usize,
            // balancePlay: isize,
            // avatar: SAvatar,
        },
        joinedTable {
            // position: ESauspielPosition,
            // userName: String,
            // playInstant: bool,
            // userID: usize,
            // balancePlay: isize,
            // avatar: SAvatar,
        },
        gameStarted {
            // gameID: usize,
            // playInstant: bool,
        },
        youGotCards {
            cards: Vec<ECard>,
        },
        hasKnocked {
            position: ESauspielPosition,
        },
        playsTheGame {
            suit: Option<EFarbe>,
            gameType: usize,
            position: Option<ESauspielPosition>,
            announcement: usize
        },
        hasContra {
            position: ESauspielPosition,
        },
        playedACard {
            position: ESauspielPosition,
            cardID: ECard,
        },
        wonTheTrick {
            // position: ESauspielPosition,
            // cards: [ECard; EPlayerIndex::SIZE],
            // playInstant: bool,
        },
        gameResult {
            won: bool,
            points: isize,
            amount: isize,
            balanceType: isize,
            gameType: String,
            gameRate: String,
            baseRate: String,
            runners: isize,
            result: isize,
            knockings: usize,
            contras: usize,
            announcement: usize,
            // players: [EPlayerRole; EPlayerIndex::SIZE]
        },
    }

    #[derive(Debug)]
    struct SMissing;
    let mut resstr_rules_no_playerindex = Err(SMissing);
    let mut resoposition_active = Err(SMissing);
    let mut resoefarbe = Err(SMissing);
    let mut vectplpositioncard_played = Vec::new();
    let mut vecerr = Vec::new();

    for jsonval_sauspieljsonevent in serde_json::from_str::<Vec<serde_json::Value>>(str_json)? {
        match serde_json::value::from_value(jsonval_sauspieljsonevent) {
            Err(err) => {
                vecerr.push(err);
            },
            Ok(
                VSauspielJSONEvent::yourAuthenticationSucceeded{..}
                | VSauspielJSONEvent::joinedTable{..}
                | VSauspielJSONEvent::gameStarted{..}
                | VSauspielJSONEvent::youGotCards{..} // TODO? derive EKurzLang from this
                | VSauspielJSONEvent::hasKnocked{..} // TODO collect doublings
                | VSauspielJSONEvent::hasContra{..} // TODO collect stoss
                | VSauspielJSONEvent::wonTheTrick{..} // TODO? consistency checks
            )
            => {
            },
            Ok(VSauspielJSONEvent::playsTheGame{suit, position, ..}) => {
                resoefarbe = Ok(suit);
                resoposition_active = Ok(position);
            },
            Ok(VSauspielJSONEvent::playedACard{position, cardID}) => {
                vectplpositioncard_played.push((position, cardID));
            },
            Ok(VSauspielJSONEvent::gameResult{gameType, ..}) => {
                resstr_rules_no_playerindex = Ok(gameType);
                // TODO? consistency check gameresult
            },
        }
    }
    move || -> Result<_,_> {
        let position_corresponding_to_epi0 = vectplpositioncard_played.first()
            .ok_or_else(|| format_err!("Cannot determine position_corresponding_to_epi0"))
            .map(|(position, _card)| *position)?;
        let position_to_epi = move |position: ESauspielPosition| {
            unwrap!(EPlayerIndex::checked_from_usize(position.wrapped_difference(position_corresponding_to_epi0).0.to_usize()))
        };
        let rules = crate::rules::parser::parse_rule_description_simple(&{
            // TODO? good to go through parse_rule_description_simple?
            let mut str_rules = "".to_string();
            if let Some(efarbe) = resoefarbe.map_err(|err| format_err!("oefarbe not found: {:?}", err))? {
                str_rules += &format!("{} ", efarbe);
            }
            str_rules += &resstr_rules_no_playerindex.map_err(|err| format_err!("str_rules_no_playerindex not found: {:?}", err))?;
            if let Some(position_active)=resoposition_active.map_err(|err| format_err!("oepi_active not found: {:?}", err))? {
                str_rules += &format!(" von {}", position_to_epi(position_active));
            }
            str_rules
        })?;
        let ekurzlang = EKurzLang::values()
            .find(|ekurzlang| ekurzlang.cards_per_player()*EPlayerIndex::SIZE==vectplpositioncard_played.len())
            .ok_or(format_err!("Could not determine ekurzlang"))?;
        let stichseq = SStichSequence::new_from_cards(
            ekurzlang,
            vectplpositioncard_played.iter().map(|tplpositioncard| tplpositioncard.1),
            &rules
        ).map_err(|SDuplicateCard(card)| format_err!("Duplicate card: {}", card))?;
        SGameGeneric::</*Ruleset*/EKurzLang, (), ()>::new_finished_with_ruleset(
            rules,
            SExpensifiers::new_no_stock_doublings_stoss(), // TODO
            SStichSequenceGameFinished::new(&stichseq),
            ekurzlang,
            fn_before_zugeben,
        ).and_then(|game| game.finish()
            .map_err(|err| format_err!("Could not finish game: {:?}", err))
        )
    }().map_err(|err| format_err!("{}: {:?}", err, vecerr))
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
            ).map_err(|SDuplicateCard(card)| format_err!("Duplicate card: {}", card))?;
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
        .chunk_by(|str_line| str_line.trim().is_empty())
        .into_iter()
        .filter(|(b_is_empty, _grpstr_line)| !b_is_empty)
        .map(|(_b_is_empty, grpstr_line)| -> Result<_, _> {
            let mut grpstr_line = grpstr_line.peekable();
            grpstr_line.next()
                .filter(|str_geber| str_geber.starts_with("Geber: ")) // TODO be more precise?
                .ok_or_else(|| format_err!("Expected 'Geber: '"))?;
            if Some(&"Spielart: Schieber")==grpstr_line.peek() {
                grpstr_line.next();
            }
            let mut vecstr_player_name = Vec::<String>::new();
            let mut oekurzlang = None;
            for _epi in EPlayerIndex::values() {
                let (str_player_name, resekurzlang_epi) = grpstr_line.next()
                    .ok_or_else(|| format_err!("Expected description of player's hand"))
                    .and_then(|str_player_hand| {
                        parse_trimmed(
                            str_player_hand,
                            (
                                attempt(many1::<String,_>(alpha_num())), // TODO allow more characters for player names
                                string(" hat: "),
                                sep_by::<Vec<_>,_,_>(
                                    choice!(card_parser().map(Some), string("DU").map(|_|None)), // TODO? be more precise
                                    char(' ')
                                ),
                            ).map(|(str_player_name, _, vecocard): (_, _, Vec<Option<ECard>>)| (
                                str_player_name,
                                EKurzLang::from_cards_per_player(vecocard.len())
                                    .ok_or_else(|| format_err!("Incorrect number of cards: {}", vecocard.len())),
                            ))
                        ).map_err(|err| format_err!("Failed to parse <player> hat <hand>: {:?}", err))
                    })?;
                vecstr_player_name.push(str_player_name);
                match (oekurzlang, resekurzlang_epi?) {
                    (None, ekurzlang_epi) => oekurzlang=Some(ekurzlang_epi),
                    (Some(ekurzlang), ekurzlang_epi) => {
                        if ekurzlang!=ekurzlang_epi {
                            Err(format_err!("Mismatched ekurzlang values"))?;
                        }
                    },
                }
            }
            let ekurzlang = unwrap!(oekurzlang);
            let mapepistr_player = EPlayerIndex::map_from_raw(unwrap!(iter_to_arr(vecstr_player_name)));
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
    test_internal(include_bytes!("Schafkopfprotokoll_vom_04.08.24.txt"));
}

#[test]
fn test_analyze_plain() {
    fn internal_test(str_in: &str) {
        unwrap!(unwrap!(analyze_plain(str_in).exactly_one()));
    }
    internal_test("Rufspiel Blaue von 3: so h7 go eo ho hz hk eu gu h9 su g8 g9 ga gk e9 ea ek ez e7 g7 ha s7 gz sa s9 h8 sz e8 sk hu s8");
    internal_test("Schelln-Wenz von 2: ea ek e7 ez gz g7 ga go eu e9 so s9 gu h7 sa hu su h8 e8 sz s8 ha eo g9 s7 h9 hk g8 sk hz ho gk");
}
