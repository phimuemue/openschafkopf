use openschafkopf_lib::{
    ai::{
        SAi,
        handiterators::*,
    },
    primitives::*,
    rules::{
        ruleset::VStockOrT,
        SDoublings,
        SExpensifiers,
        SStoss,
        SRules,
        TRules,
        TRulesPlayerIndex,
        SDisplayRules,
        parser::parse_rule_description_simple,
        VTrumpfOrFarbe,
    },
    game::{SGame, SExpensifiersNoStoss},
};
use openschafkopf_util::*;
use itertools::Itertools;
use plain_enum::{EnumMap, PlainEnum};
use as_num::*;
use std::sync::{Arc, Mutex};

pub use super::handconstraint::*;

enum VChooseItAhand {
    All,
    Sample(/*n_samples*/usize, /*on_pool*/Option<usize>),
}

#[derive(Clone)]
enum VUserSuppliedPosition {
	CurrentPlayer,
	Concrete(EPlayerIndex),
	RulesAnnouncer,
}
impl VUserSuppliedPosition {
    fn with_concrete_playerindex<R>(&self, rules: &SRules, fn_with_concrete_playerindex: impl FnOnce(Option<EPlayerIndex>)->Result<R, Error>) -> Result<R, Error> {
        fn_with_concrete_playerindex(match self {
            VUserSuppliedPosition::CurrentPlayer => None, // To be determined with stichseq
            VUserSuppliedPosition::Concrete(epi_position_concrete) => {
                Some(*epi_position_concrete)
            }
            VUserSuppliedPosition::RulesAnnouncer => {
                Some(
                    rules.playerindex()
                        .ok_or_else(||format_err!("Rules are not 'announced'."))?
                )
            },
        })
    }
}

pub fn subcommand_given_game(str_subcommand: &'static str, str_about: &'static str) -> clap::Command<'static> {
    clap::Command::new(str_subcommand)
        .about(str_about)
        .help_heading("Game description")
        .arg(openschafkopf_shared_args::ruleset_arg())
        .arg( // "overrides" ruleset // TODO? make ruleset optional
            clap::Arg::new("rules")
                .long("rules")
                .takes_value(true)
                .required(false)
                .multiple_occurrences(true)
                .help("Rules as plain text")
                .long_help("Rules, given in plain text. The program tries to be lenient in the input format, so that all of the following should be accepted: \"gras wenz von 1\", \"farbwenz gras von 1\", \"BlauWenz von 1\". Players are numbere from 0 to 3, where 0 is the player to open the first stich (1, 2, 3 follow accordingly).")
        )
        .arg(clap::Arg::new("position")
            .long("position")
            .help("Position of the player")
			.value_parser(|str_position: &str| -> Result<VUserSuppliedPosition, String> {
				match str_position {
					"0" => Ok(VUserSuppliedPosition::Concrete(EPlayerIndex::EPI0)),
					"1" => Ok(VUserSuppliedPosition::Concrete(EPlayerIndex::EPI1)),
					"2" => Ok(VUserSuppliedPosition::Concrete(EPlayerIndex::EPI2)),
					"3" => Ok(VUserSuppliedPosition::Concrete(EPlayerIndex::EPI3)),
					"current" => Ok(VUserSuppliedPosition::CurrentPlayer),
					"rulesannouncer" => Ok(VUserSuppliedPosition::RulesAnnouncer),
					_ => Err(format!("{str_position} not recognized. Supported values: 0, 1, 2, 3, current, rulesannouncer."))
				}
			})
            .default_value("current")
        )
        .arg(clap::Arg::new("hand")
            .long("hand")
            .takes_value(true)
            .multiple_occurrences(true)
            .help("The cards on someone's hand")
            .long_help("The cards on the current player's hand (simply separated by spaces, such as \"eo go ho so eu gu hu su\" for a Sie), or the hands of all players. Specifying all player's hands works by first listing cards of player 0, then player 1, then player 2, then player 3 (Example: \"ea ez  ga gz  ha hz  sa sz\" means player 0 has Eichel-Ass and Eichel-Zehn, player 1 has Gras-Ass and Gras-Zehn, and so forth). You can use underscore to leave \"holes\" in other players' hands (Example: \"ea __  ga __  ha __  sa __\" means player 0 has Eichel-Ass and another unknown card, player 1 has Gras-Ass and unknown card, and so forth).")
        )
        .arg(clap::Arg::new("cards_on_table")
            .long("played-cards")
            .takes_value(true)
            .help("Cards played so far")
            .long_help("Cards played so far in the order they have been played. The software matches the cards to the respective player.")
        )
        .arg(clap::Arg::new("stoss")
            .long("stoss")
            .takes_value(true)
            .help("Stosses given")
            .long_help("Stosses given so far. Enumerate the respective player indices one after another, separated by a space.")
        )
        .arg(super::shared_args::glob_files_arg()
            .long("file")
        )
        .help_heading("Generating hands")
        .arg(clap::Arg::new("simulate_hands")
            .long("simulate-hands")
            .takes_value(true)
            .help("Number of hands to simulate")
            .long_help("Number of unknown hands to simulate. Can either be a number or \"all\", causing the software to generate all possible combinations.")
        )
        .arg(clap::Arg::new("constrain_hands")
            .long("constrain-hands")
            .takes_value(true)
            .multiple_occurrences(true)
            .help("Constrain simulated hands")
            .long_help("Constrain simulated hands so that certain criteria are fulfilled. Example: \"4<ctx.trumpf(0) && ctx.ea(1)\" only considers card distributions where player 0 has more than 4 Trumpf and player 1 has Eichel-Ass. (Players are numbere from 0 to 3, where 0 is the player to open the first stich (1, 2, 3 follow accordingly).)") // TODO improve docs
        )
        .arg(clap::Arg::new("repeat_hands")
            .long("repeat-hands")
            .takes_value(true)
            .help("Repeat each simulated card distribution")
        )
        .help_heading(None)
        .arg(clap::Arg::new("verbose")
            .long("verbose")
            .short('v')
            .help("Show more output")
        )
}

fn for_each_game_situation(
    clapmatches: &clap::ArgMatches,
    b_verbose: bool,
    mut fn_with_game_situation: impl FnMut(
        (&EnumMap<EPlayerIndex, SHand>, &str/*str_ahand*/, bool/*b_explicitly_given_single_ahand*/),
        (&SRules, bool/*b_explicitly_given_single_rules*/),
        &SStichSequence,
        Option<ECard>/*ocard_played*/,
        EPlayerIndex/*epi_position*/,
        &SExpensifiers,
    ) -> Result<(), Error>,
) -> Result<(), Error> {
    let usersuppliedposition : &VUserSuppliedPosition = unwrap!(clapmatches.get_one("position"));
    super::glob_files(clapmatches, |opath, str_input, _i_input| {// short-circuits when user requested input from files but something bad happened
        super::analyze::for_each_gameresult(opath.as_ref(), &str_input, b_verbose, |gamewithdesc| {
            match gamewithdesc.resgameresult {
                Ok(gameresult) => {
                    if b_verbose {
                        println!("{}", gamewithdesc.str_description);
                    }
                    match gameresult.stockorgame {
                        VStockOrT::Stock(_) => {
                            if b_verbose {
                                println!("Ignoring {}.", gamewithdesc.str_description);
                            }
                        },
                        VStockOrT::OrT(game_in) => {
                            let _result_used_by_verify_or_println = verify_or_println!(usersuppliedposition.with_concrete_playerindex(&game_in.rules, |oepi_position_concrete| {
                                verify!(SGame::new(
                                    game_in.aveccard.clone(),
                                    SExpensifiersNoStoss::new_with_doublings(
                                        game_in.expensifiers.n_stock,
                                        game_in.expensifiers.doublings.clone(),
                                    ),
                                    game_in.rules.clone()
                                ).play_cards_and_stoss(
                                    &game_in.expensifiers.vecstoss,
                                    game_in.stichseq.visible_cards(),
                                    /*fn_before_zugeben*/|game, _i_stich, epi_zugeben, card_played| {
                                        unwrap!/*assume that fn_with_game_situation can work with a pre-checked game*/(fn_with_game_situation(
                                            (
                                                &game.ahand,
                                                /*str_ahand*/&game.ahand.iter()
                                                    .map(|hand|
                                                        SDisplayCardSlice::new(hand.cards().clone(), &game.rules).to_string()
                                                    )
                                                    .join(" | "),
                                                /*b_explicitly_given_single_ahand*/false
                                            ),
                                            (&game.rules, /*b_explicitly_given_single_rules*/false),
                                            &game.stichseq,
                                            Some(card_played),
                                            oepi_position_concrete.unwrap_or(epi_zugeben),
                                            &game.expensifiers,
                                        ));
                                    },
                                ))
                            }));
                        },
                    }
                },
                Err(err) => println!("Error on {}: {}", gamewithdesc.str_description, err),
            }
        });
    })?;
    { // "Classical" invocation
        let vectplvecocardstr_ahand = match clapmatches.values_of("hand") {
            Some(values_hand) => {
                values_hand.map(|str_ahand| 
                    cardvector::parse_optional_cards::<Vec<_>>(str_ahand)
                        .ok_or_else(||format_err!("Could not parse hand: {}", str_ahand))
                        .map(|vecocard| (vecocard, str_ahand))
                )
                .collect::<Result<Vec<_>, _>>()?
            },
            None => {
                Vec::new()
            },
        };
        let veccard_stichseq = match clapmatches.value_of("cards_on_table") { // TODO allow multiple stichseq (in particular something like "ea | ez ek e9  sa sz | sk s9" so that the user can query intermittent game states).
            None => Vec::new(),
            Some(str_cards_on_table) => cardvector::parse_cards(str_cards_on_table)
                .ok_or_else(||format_err!("Could not parse played cards"))?,
        };
        let b_explicitly_given_single_ahand = vectplvecocardstr_ahand.len()==1;
        let vecstoss = match clapmatches.value_of("stoss")
            .map(|str_stoss| {
                if str_stoss.trim().is_empty() {
                    Ok(Vec::new())
                } else {
                    str_stoss
                        .split(' ')
                        .filter(|str_epi| !str_epi.is_empty())
                        .map(|str_epi| str_epi.parse::<EPlayerIndex>()
                            .map(|epi| SStoss {
                                epi,
                                n_cards_played: 0, // TODO? make adjustable
                            })
                        )
                        .collect::<Result<Vec<_>, _>>()
                }
            })
        {
            Some(Ok(vecstoss)) => vecstoss,
            None => Vec::new(),
            Some(Err(e)) => return Err(format_err!("Could not parse stoss: {}", e)),
        };
        let expensifiers = SExpensifiers::new(
            /*n_stock*/0, // TODO? make adjustable
            /*doublings*/SDoublings::new_full( // TODO? make adjustable
                SStaticEPI0{},
                [false; EPlayerIndex::SIZE],
            ),
            vecstoss,
        );
        for (vecocard_hand, str_ahand) in vectplvecocardstr_ahand.iter() {
            let veccard_duplicate = veccard_stichseq.iter()
                .chain(vecocard_hand.iter().filter_map(|ocard| ocard.as_ref()))
                .duplicates()
                .collect::<Vec<_>>();
            if !veccard_duplicate.is_empty() {
                return Err(format_err!("Cards are used more than once: {}", veccard_duplicate.iter().join(", ")));
            }
            let (itrules, b_explicitly_given_single_rules) = match clapmatches.values_of("rules")
                .map(|values| values.map(parse_rule_description_simple))
                .into_iter()
                .flatten()
                .collect::<Result<Vec<_>,_>>()
            {
                Ok(vecrules) => {
                    if vecrules.is_empty() {
                        let ruleset = openschafkopf_shared_args::get_ruleset(clapmatches)?;
                        (
                            Box::new(ruleset
                                .avecrulegroup.into_raw().into_iter()
                                .flat_map(|vecrulegroup|
                                    vecrulegroup.into_iter().flat_map(|rulegroup| {
                                        rulegroup.vecorules.into_iter()
                                            .filter_map(|orules|
                                                orules.as_ref().map(|rules|
                                                    SRules::from(rules.clone())
                                                )
                                            )
                                    })
                                )
                                .chain(match ruleset.stockorramsch {
                                    VStockOrT::Stock(_) => None,
                                    VStockOrT::OrT(rules) => Some(rules.into())
                                })
                            ) as Box<dyn Iterator<Item=SRules>>,
                            /*b_explicitly_given_single_rules*/false,
                        )
                    } else {
                        let b_explicitly_given_single_rules = vecrules.len()==1;
                        (Box::new(vecrules.into_iter()) as Box<dyn Iterator<Item=SRules>>, b_explicitly_given_single_rules)
                    }
                },
                Err(err) => {
                    return Err(format_err!("Could not parse rules: {}", err));
                },
            };
            for rules in itrules {
                let rules = &rules;
                let (stichseq, ahand_with_holes, epi_position) = usersuppliedposition.with_concrete_playerindex(rules, |oepi_position_concrete| {
                    EKurzLang::values()
                        .filter_map(|ekurzlang| {
                            let mut stichseq = SStichSequence::new(ekurzlang);
                            for &card in veccard_stichseq.iter() {
                                if !ekurzlang.supports_card(card)
                                    || stichseq.current_playerindex().is_none()
                                {
                                    return None; // TODO? distinguish error
                                }
                                stichseq.zugeben(card, rules);
                            }
                            let epi_position = oepi_position_concrete.unwrap_or_else(||
                                unwrap!(stichseq.current_stich().current_playerindex())
                            );
                            if_then_some!(
                                stichseq.remaining_cards_per_hand()[epi_position]==vecocard_hand.len(),
                                (SHand::new_from_iter(vecocard_hand.iter().flatten()), epi_position)
                                    .to_ahand()
                            ).or_else(|| {
                                let n_cards_total = stichseq.kurzlang().cards_per_player()*EPlayerIndex::SIZE;
                                if_then_some!(stichseq.visible_cards().count()+vecocard_hand.len()==n_cards_total, {
                                    let mut i_card_lo = 0;
                                    EPlayerIndex::map_from_raw(stichseq.remaining_cards_per_hand().as_raw().map(|n_remaining| {
                                        // Note: This function is called for each index in order (https://doc.rust-lang.org/std/primitive.array.html#method.map)
                                        let hand = SHand::new_from_iter(
                                            vecocard_hand[i_card_lo..i_card_lo+n_remaining].iter()
                                                .flatten()
                                        );
                                        i_card_lo += n_remaining;
                                        assert!(hand.cards().len() <= n_remaining);
                                        hand
                                    }))
                                })
                            })
                            .map(|ahand| (stichseq, ahand, epi_position))
                        })
                        .exactly_one_2()
                        .map_err(|err| format_err!("Could not determine ekurzlang: {:?}", err))
                })?;
                // TODO check that everything is ok (no duplicate cards, cards are allowed, current stich not full, etc.)
                if let Some(epi_active) = rules.playerindex() {
                    let veccard_hand_active = stichseq.cards_from_player(&ahand_with_holes[epi_active], epi_active)
                        .collect::<Vec<_>>();
                    if veccard_hand_active.len()==stichseq.kurzlang().cards_per_player() {
                        if !rules.can_be_played(SFullHand::new(&veccard_hand_active, stichseq.kurzlang())) {
                            if b_explicitly_given_single_rules {
                                return Err(format_err!("Rules {} cannot be played given these cards.", SDisplayRules::new(rules, /*b_include_playerindex*/true)));
                            } else {
                                if b_verbose {
                                    println!("Rules {} cannot be played given these cards.", SDisplayRules::new(rules, /*b_include_playerindex*/true));
                                }
                                continue;
                            }
                        }
                    } else {
                        // let hand iterators try to generate valid hands.
                    }
                }
                fn_with_game_situation(
                    (&ahand_with_holes, str_ahand, b_explicitly_given_single_ahand),
                    (rules, b_explicitly_given_single_rules),
                    &stichseq,
                    /*ocard_played*/None,
                    epi_position,
                    &expensifiers,
                )?;
            } // itrules
        } // vecocard_hand, str_ahand)
    }
    Ok(())
}

pub fn with_common_args<FnWithArgs>(
    clapmatches: &clap::ArgMatches,
    mut fn_with_args: FnWithArgs,
) -> Result<(), Error>
    where
        for<'rules> FnWithArgs: FnMut(
            Box<dyn Iterator<Item=EnumMap<EPlayerIndex, SHand>>+Send+'rules>,
            &'rules SRules,
            &SStichSequence,
            Option<ECard>/*ocard_played*/,
            &EnumMap<EPlayerIndex, SHand>, // TODO? Good idea? Could this simply given by itahand?
            EPlayerIndex/*epi_position*/,
            &SExpensifiers,
            bool/*b_verbose*/,
            Arc<Mutex<EnumMap<ECard, EnumSet<EPlayerIndex>>>>,
        ) -> Result<(), Error>,
{
    let iteratehands = if_then_some!(let Some(str_itahand)=clapmatches.value_of("simulate_hands"),
        if "all"==str_itahand.to_lowercase() { // TODO replace this case by simply "0"?
            VChooseItAhand::All
        } else {
            match str_itahand
                .split('/')
                .map(|str_n| str_n.parse().ok())
                .collect::<Option<Vec<_>>>()
                .as_deref()
            {
                Some(&[n_samples]) => VChooseItAhand::Sample(n_samples, /*on_pool*/None),
                Some(&[n_samples, n_pool]) => VChooseItAhand::Sample(n_samples, Some(n_pool)),
                _ => return Err(format_err!("Failed to parse simulate_hands")),
            }
        }
    ).unwrap_or_else(|| {
        VChooseItAhand::All
    });
    let vecotplconstraintstr = clapmatches.values_of("constrain_hands")
        .map(|values_constrain_hands| -> Result<Vec<(SConstraint, &str)>, _> {
            values_constrain_hands
                .map(|str_constrain_hands|
                    str_constrain_hands.parse::<SConstraint>()
                        .map_err(|err| format_err!("Cannot parse hand constraints: {:?}", err))
                        .map(|constraint| (
                            constraint,
                            str_constrain_hands,
                        )),
                )
                .collect::<Result<Vec<_>,_>>()
        })
        .transpose()?
        .map(|vectplconstraintstr: Vec<(SConstraint, &str)>| -> Vec<Option<(SConstraint, &str)>> {
            vectplconstraintstr.into_iter().map(Some).collect()
        })
        .unwrap_or_else(|| vec!(None));
    assert!(!vecotplconstraintstr.is_empty());
    assert!(vecotplconstraintstr.iter().map(Option::is_some).all_equal());
    let b_verbose = clapmatches.is_present("verbose");
    let n_repeat_hand = clapmatches.value_of("repeat_hands").unwrap_or("1").parse()?;
    for_each_game_situation( clapmatches, b_verbose, |
        (ahand_with_holes, str_ahand, b_explicitly_given_single_ahand),
        (rules, b_explicitly_given_single_rules),
        stichseq,
        ocard_played,
        epi_position,
        expensifiers,
    | {
        for otplconstraintstr in &vecotplconstraintstr {
            let mapepin_cards_per_hand = stichseq.remaining_cards_per_hand();
            for epi in EPlayerIndex::values() {
                assert!(ahand_with_holes[epi].cards().len() <= mapepin_cards_per_hand[epi]);
            }
            macro_rules! forward{($n_ahand_total: expr, $itahand_factory: expr, $fn_take: expr) => {{ // TODORUST generic closures
                let mut n_ahand_seen = 0;
                let mut n_ahand_valid = 0;
                if b_verbose || !b_explicitly_given_single_rules {
                    println!("Rules: {}", SDisplayRules::new(rules, /*b_include_playerindex*/true));
                }
                let mapcardsetepi_distribution = Arc::new(Mutex::new(ECard::map_from_fn(|_card| EnumSet::<EPlayerIndex>::new_empty())));
                if b_verbose
                    || !b_explicitly_given_single_ahand
                    || 1<vecotplconstraintstr.len()
                {
                    println!("Hand(s): {} {}",
                        str_ahand,
                        match otplconstraintstr {
                            Some((_constraint, str_constraint)) if 1<vecotplconstraintstr.len() => {
                                format!("[{}]", str_constraint)
                            },
                            _ => "".to_string(),
                        },
                    );
                }
                fn_with_args(
                    Box::new(
                        #[allow(clippy::redundant_closure_call)]
                        $fn_take($itahand_factory(
                            &stichseq,
                            ahand_with_holes.clone(),
                            rules,
                            &expensifiers.vecstoss,
                            /*fn_inspect*/|b_valid_so_far, ahand| {
                                n_ahand_seen += 1;
                                let b_valid = b_valid_so_far
                                    && otplconstraintstr.as_ref().map_or(true, |(constraint, _str_constraint)|
                                        constraint.eval(&stichseq, ahand, rules.clone())
                                    );
                                if b_valid {
                                    n_ahand_valid += 1;
                                }
                                if b_verbose {
                                    println!("{} {}/{}/{} {}",
                                        if b_valid {
                                            '>'
                                        } else {
                                            '|'
                                        },
                                        n_ahand_valid,
                                        n_ahand_seen,
                                        $n_ahand_total,
                                        display_card_slices(&ahand, rules, " | "),
                                    )
                                }
                                b_valid
                            }
                        ))
                        .inspect(|ahand| {
                            let mut mapcardsetepi_distribution = unwrap!(mapcardsetepi_distribution.lock());
                            for epi in EPlayerIndex::values() {
                                for &card in ahand[epi].cards() {
                                    mapcardsetepi_distribution[card].insert(epi);
                                }
                            }
                        })
                        .flat_map(|ahand| {
                            std::iter::repeat_n(
                                ahand,
                                n_repeat_hand,
                            )
                        })
                    ),
                    rules,
                    &stichseq,
                    ocard_played,
                    &ahand_with_holes,
                    epi_position,
                    &expensifiers,
                    b_verbose,
                    mapcardsetepi_distribution.clone(), // Only to be used after fn_with_args drove the iterator to completion
                )
            }}}
            match (&iteratehands, rules.playerindex()) {
                (VChooseItAhand::All, _oepi_active) => {
                    let mut n_cards_unknown = mapepin_cards_per_hand.iter().sum::<usize>()
                        - ahand_with_holes.iter().map(|hand| hand.cards().len()).sum::<usize>();
                    let n_ahand_total = EPlayerIndex::values()
                        .fold(1u64, |n_ahand_total, epi| {
                            let n_cards_sampled = mapepin_cards_per_hand[epi]-ahand_with_holes[epi].cards().len();
                            let n_binom = num_integer::binomial(
                                n_cards_unknown.as_num::<u64>(),
                                n_cards_sampled.as_num::<u64>(),
                            );
                            n_cards_unknown -= n_cards_sampled;
                            n_ahand_total*n_binom
                        });
                    forward!(n_ahand_total, internal_all_possible_hands, |itahand| itahand)
                },
                (VChooseItAhand::Sample(n_samples, None), _oepi_active) => {
                    forward!(n_samples, internal_forever_rand_hands, |itahand| Iterator::take(itahand, *n_samples))
                },
                (VChooseItAhand::Sample(n_samples, Some(_n_pool)), None) => {
                    forward!(n_samples, internal_forever_rand_hands, |itahand| Iterator::take(itahand, *n_samples))
                },
                (VChooseItAhand::Sample(n_samples, Some(n_pool)), Some(epi_active)) => {
                    forward!(
                        *n_samples,
                        internal_forever_rand_hands,
                        |itahand_pool| {
                            Iterator::take(itahand_pool, *n_pool)
                                .map(|ahand: EnumMap<EPlayerIndex, SHand>| {
                                    let payout = SAi::new_simulating(
                                        /*n_rank_rules_samples*/100,
                                        /*n_suggest_card_branches*/1,
                                        /*n_suggest_card_samples*/0,
                                    ).rank_rules(
                                        SFullHand::new(
                                            &stichseq.cards_from_player(
                                                &ahand[epi_active],
                                                epi_active,
                                            ).collect::<Vec<_>>(),
                                            stichseq.kurzlang(),
                                        ),
                                        epi_active,
                                        rules,
                                        expensifiers,
                                    ).omaxselfishmin.as_ref().unwrap_static_some().avg();
                                    (ahand, payout)
                                })
                                .k_largest_by(*n_samples, |tplahandpayout_lhs, tplahandpayout_rhs| unwrap!(tplahandpayout_lhs.1.partial_cmp(&tplahandpayout_rhs.1)))
                                .map(|(ahand, _payout)| ahand)
                        }
                    )
                },
            }?;
        }
        Ok(())
    })
}

fn print_table<'tableline>(
    str_indent: &'static str,
    itvecstr: impl Iterator<Item=&'tableline Vec<String>> + Clone, // TODO more generic?
) {
    let vecn_width = (0../*n_columns*/unwrap!(itvecstr.clone().map(Vec::len).all_equal_value()))
        .map(|i_column|
            unwrap!(itvecstr.clone().map(|vecstr| vecstr[i_column].len()).max())
        )
        .collect::<Vec<_>>();
    for vecstr in itvecstr {
        print!("{str_indent}");
        for (str_column, n_width) in itertools::zip_eq(vecstr, &vecn_width) {
            print!("{str_column:<n_width$}");
        }
        println!();
    }
}

pub fn print_card_distribution_statistics(
    stichseq: &SStichSequence,
    rules: &SRules,
    mapcardsetepi_distribution: &EnumMap<ECard, EnumSet<EPlayerIndex>>,
) {
    enum VWithPlayer<MultiplePlayers> {
        OnePlayer(EPlayerIndex),
        MultiplePlayers(MultiplePlayers),
    }
    let ittplcardwithplayer = ECard::values(stichseq.kurzlang()).filter_map(|card| {
        let setepi = &mapcardsetepi_distribution[card];
        match setepi.iter().exactly_one_2() {
            Err(ExactlyOneError::Empty) => {
                assert!(stichseq.visible_cards().find(|(_epi, card_visible)| *card_visible==&card).is_some());
                None // Ignore already played cards
            },
            Ok(epi_exactly_one) => Some(VWithPlayer::OnePlayer(epi_exactly_one)),
            Err(ExactlyOneError::MoreThanOne([_,_],_)) => Some(VWithPlayer::MultiplePlayers(setepi)),
        }.map(|withplayer| (card, withplayer))
    });
    // Collect unplayed cards that occured at exaclty one player vs at multiple players
    let mut mapepisetcard_with_one_player = EPlayerIndex::map_from_fn(|_epi| EnumSet::<ECard>::new_empty());
    let mut maptrumpforfarbeveccard_with_multiple_players = VTrumpfOrFarbe::map_from_fn(|_trumpforfarbe| Vec::new());
    for (card, withplayer) in ittplcardwithplayer.clone() {
        match withplayer {
            VWithPlayer::OnePlayer(epi_exactly_one) => {
                verify!(mapepisetcard_with_one_player[epi_exactly_one].insert(card));
            },
            VWithPlayer::MultiplePlayers(_setepi) => {
                maptrumpforfarbeveccard_with_multiple_players[rules.trumpforfarbe(card)].push(card);
            }
        }
    }
    // Determine which players' hands are considered "completely known"
    let mapepin_card_count = stichseq.remaining_cards_per_hand();
    let setepi_hand_completely_known = EnumSet::<EPlayerIndex>::new_from_fn(|epi|
        mapepin_card_count[epi]==mapepisetcard_with_one_player[epi].iter().count()
    );
    // Find out if one particular player cannot have certain cards
    let mut mapepisetcard_not_with_player = EPlayerIndex::map_from_fn(|_epi| EnumSet::<ECard>::new_empty());
    for (card, withplayer) in ittplcardwithplayer.clone() {
        match withplayer {
            VWithPlayer::OnePlayer(epi_exactly_one) => {
                assert!(mapepisetcard_with_one_player[epi_exactly_one].contains(card));
            },
            VWithPlayer::MultiplePlayers(setepi) => {
                if let Ok(epi_exactly_one) = setepi_hand_completely_known.complement().minus(setepi).iter().exactly_one_2() {
                    verify!(mapepisetcard_not_with_player[epi_exactly_one].insert(card));
                }
            },
        }
    }
    fn columns_for_always_or_never_cards(
        str_first_column: String,
        fn_player_column: impl Fn(EPlayerIndex)->String,
    ) -> Vec<String> {
        std::iter::chain(
            std::iter::once(str_first_column),
            itertools::intersperse(
                EPlayerIndex::values().map(fn_player_column),
                " | ".to_string(),
            ),
        ).collect()
    }
    println!("Card distribution (as per simulation):");
    print_table(
        /*str_indent*/" ",
        std::iter::chain(
            std::iter::once(columns_for_always_or_never_cards("+ ".to_string(), |epi| {
                let mut veccard = mapepisetcard_with_one_player[epi].iter()
                    .collect::<Vec<_>>();
                rules.sort_cards(&mut veccard);
                veccard.iter().map(ECard::to_string)
                    .pad_using(mapepin_card_count[epi], |_| "__".to_string())
                    .join(" ")
            })),
            if_then_some!(mapepisetcard_not_with_player.iter().any(|setcard| !setcard.is_empty()), {
                columns_for_always_or_never_cards("- ".to_string(), |epi| {
                    SDisplayCardSlice::new(
                        mapepisetcard_not_with_player[epi].iter().collect::<Vec<_>>(),
                        rules
                    ).to_string()
                })
            }),
        ).collect::<Vec<_>>().iter()
    );
    let mut vecvecstr_trumpforfarbe = Vec::new(); // We collect "backwards" into this vector. Simplifies unioning (see below).
    for (trumpforfarbe, mut veccard_with_multiple_players) in itertools::zip_eq( // TODO EnumMap iterator instead of zip_eq + manually reversing two iterators
        VTrumpfOrFarbe::values().rev(),
        maptrumpforfarbeveccard_with_multiple_players.into_raw().into_iter().rev(),
    ) {
        rules.sort_cards(&mut veccard_with_multiple_players);
        let mut push_trumpforfarbe_line = |str_trumpforfarbe_heading, slccard_chunk: &[ECard], setepi: &EnumSet<EPlayerIndex>| {
            vecvecstr_trumpforfarbe.push(std::iter::chain(
                [
                    str_trumpforfarbe_heading,
                    slccard_chunk.iter().join(" ").to_string(),
                    " | ".to_string()
                ],
                itertools::intersperse(
                    EPlayerIndex::values().map(|epi| {
                        if setepi.contains(epi) {
                            epi.to_string()
                        } else {
                            "".to_string()
                        }
                    }),
                    " ".to_string(),
                ),
            ).collect::<Vec<_>>())
        };
        let str_trumpforfarbe_heading = format!("? {trumpforfarbe}: ");
        match veccard_with_multiple_players
            .chunk_by(|card_lhs, card_rhs| mapcardsetepi_distribution[*card_lhs]==mapcardsetepi_distribution[*card_rhs]) // TODO? chunk_by_key
            .map(|slccard_chunk| (
                unwrap!(
                    slccard_chunk.iter()
                        .map(|&card| &mapcardsetepi_distribution[card])
                        .all_equal_value()
                ),
                slccard_chunk
            ))
            .rev()
            .exactly_one_2()
        {
            Err(ExactlyOneError::Empty) => {},
            Ok((setepi, slccard_chunk)) => {
                push_trumpforfarbe_line(str_trumpforfarbe_heading, slccard_chunk, setepi);
            },
            Err(ExactlyOneError::MoreThanOne(atplsetepislccard, ittplsetepislccard)) => {
                let mut setepi_union = EnumSet::<EPlayerIndex>::new_empty();
                for (setepi_chunk, slccard_chunk) in std::iter::chain(atplsetepislccard, ittplsetepislccard) {
                    push_trumpforfarbe_line("".to_string(), slccard_chunk, setepi_chunk);
                    for epi_chunk in setepi_chunk.iter() { // TODO EnumSet::union
                        setepi_union.insert(epi_chunk);
                    }
                }
                push_trumpforfarbe_line(
                    str_trumpforfarbe_heading,
                    /*slccard_chunk*/&[], // Done by the "sub-items"
                    &setepi_union,
                );
            },
        }

    }
    if !vecvecstr_trumpforfarbe.is_empty() {
        print_table(/*str_indent*/" ", vecvecstr_trumpforfarbe.iter().rev());
    }
}
