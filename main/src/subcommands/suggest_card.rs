use openschafkopf_lib::{
    ai::{*, handiterators::*, gametree::*, stichoracle::SFilterByOracle, cardspartition::*},
    rules::{
        SRules,
        SDisplayRules,
        TRules,
        SRuleStateCacheFixed,
        SExpensifiers,
        SDoublings,
        SStoss,
        TRulesPlayerIndex,
        VTrumpfOrFarbe,
        ruleset::VStockOrT,
        parser::parse_rule_description_simple,
    },
    primitives::*,
    game::{SGame, SExpensifiersNoStoss},
    game_analysis::determine_best_card_table::{
        table,
        internal_table,
        SFormatInfo,
        SOutputLine,
        SPayoutStatsTable,
    },
};
use openschafkopf_util::*;
use itertools::Itertools;
use serde::Serialize;
use derive_new::new;
use plain_enum::{PlainEnum, EnumMap};
use super::handconstraint::*;
use std::io::IsTerminal;
use std::sync::{Arc, Mutex};
use as_num::*;

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
            .chunk_by_key(|&card| &mapcardsetepi_distribution[card])
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

// TODO? can we make this a fn of SPayoutStatsTable?
fn print_payoutstatstable<T: std::fmt::Display, TplStrategies: TTplStrategies>(
    payoutstatstable: &SPayoutStatsTable<T, TplStrategies>,
    b_print_table_description_before_table: bool,
    fn_mark_played: impl Fn(&T)->bool,
) {
    let slcoutputline = &payoutstatstable.output_lines();
    if b_print_table_description_before_table { // TODO? only for second-level verbosity
        println!("\nInterpreting a line of the following table (taking the first line as an example):");
        let SOutputLine{vect, perminmaxstrategyatplstrf} = &slcoutputline[0];
        println!("If you play {}, then:", vect.iter().join(" or "));
        for (i_strategy, (emmstrategy, atplstrf)) in perminmaxstrategyatplstrf.via_accessors().into_iter().enumerate() {
            let astr = atplstrf.clone().map(|tplstrf| tplstrf.0);
            let [str_payout_min, str_payout_avg, str_payout_max, str_stats] = &astr;
            println!("* Columns {i_strategy_1_based}.1 to {i_strategy_1_based}.{n_subcolumns} show tell what happens if all other players play {str_play}:",
                i_strategy_1_based = i_strategy + 1,
                n_subcolumns = astr.len(),
                str_play = match emmstrategy {
                    EMinMaxStrategy::MinMin => "adversarially and you play pessimal",
                    EMinMaxStrategy::MaxMin => "adversarially",
                    EMinMaxStrategy::MaxSelfishMin => "optimally for themselves, in disfavor of you in case of doubt",
                    EMinMaxStrategy::MaxSelfishMax => "optimally for themselves, in favor of you in case of doubt",
                    EMinMaxStrategy::Max => "optimally for you",
                },
            );
            println!("  * In the worst case (over all generated card distributions), you can enforce a payout of {str_payout_min}");
            println!("  * On average (over all generated card distributions), you can enforce a payout of {str_payout_avg}");
            println!("  * In the best case (over all generated card distributions), you can enforce a payout of {str_payout_max}");
            println!("  * {str_stats} shows the number of games lost[/zero-payout]/won (percentage not lost)");
        }
        println!();
    }
    // TODO interface should probably output payout interval per card
    let mut vecstr_id = Vec::new();
    let mut n_width_id = 0;
    for outputline in slcoutputline.iter() {
        let str_id = outputline.vect.iter()
            .map(|t| if fn_mark_played(t) {
                format!("[{t}]")
            } else {
                format!("{t}")
            })
            .join(" ");
        assign_gt(&mut n_width_id, str_id.len());
        vecstr_id.push(str_id);
    }
    for (str_id, SOutputLine{vect:_, perminmaxstrategyatplstrf}) in itertools::zip_eq(
        vecstr_id.iter(),
        slcoutputline.iter(),
    ) {
        print!("{str_id:<n_width_id$}: ");
        for ((_emmstrategy_atplstrf, atplstrf), (_emmstrategy_aformatinfo, aformatinfo)) in itertools::zip_eq(
            perminmaxstrategyatplstrf.via_accessors(),
            payoutstatstable.format_infos().via_accessors(),
        ) {
            for ((str_num, f), SFormatInfo{f_min, f_max, n_width}) in itertools::zip_eq(
                atplstrf.iter(),
                aformatinfo.iter(),
            ) {
                use termcolor::*;
                let mut stdout = StandardStream::stdout(if std::io::stdout().is_terminal() {
                    ColorChoice::Auto
                } else {
                    ColorChoice::Never
                });
                #[allow(clippy::float_cmp)]
                if f_min!=f_max {
                    let mut set_color = |color| {
                        unwrap!(stdout.set_color(ColorSpec::new().set_fg(Some(color))));
                    };
                    if f==f_min {
                        set_color(Color::Red);
                    } else if f==f_max {
                        set_color(Color::Green);
                    }
                }
                print!("{str_num:>n_width$}");
                unwrap!(stdout.reset());
            }
            print!("   ");
        }
        println!();
    }
}

pub fn subcommand(str_subcommand: &'static str) -> clap::Command<'static> {
    clap::Command::new(str_subcommand)
        .about("Suggest a card to play given the game so far and compute statistics about a given game")
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
        .help_heading("Game tree exploration")
        .arg(clap::Arg::new("branching")
            .long("branching")
            .takes_value(true)
            .help("Braching strategy for game tree search")
            .long_help("Branching strategy for game tree search. Supported values are either \"equiv<N>\" where <N> is a number or \"<Min>,<Max>\" where <Min> and <Max> are numbers. \"equiv6\" will eliminate equivalent cards in branching up to the 6th stich, after that it will do full exploration; similarily \"equiv3\" will do this up to the 3rd stich. \"2,5\" will limit the branching factor of each game tree's node to a random value between 2 and 5 (exclusively). If you specify a branching limit that is too higher than 8 (e.g. \"99,100\"), the software will not prune the game tree in any way and do a full exploration.")
        )
        .arg(clap::Arg::new("prune")
            .long("prune")
            .takes_value(true)
            .help("Prematurely stop game tree exploration if result is tenatively known")
            .long_help("Prematurely stop game tree exploration if result is tenatively known. Example: If, for a Solo, someone already reached 70 points after the fifth stich, the Solo is surely won, so the exploration can just stop right there (at the expense of a more inaccurate result).")
            .possible_values(["none", "hint"])
        )
        .arg(clap::Arg::new("abprune")
            .long("abprune")
            .help("Use alpha-beta-pruning")
            .long_help("Use alpha-beta-pruning to possibly speed up game tree exploration.")
        )
        .arg(clap::Arg::new("snapshotcache")
            .long("snapshotcache")
            .help("Use snapshot cache")
            .long_help("Use snapshot cache to possibly speed up game tree exploration.")
        )
        .arg(clap::Arg::new("no-gametree")
            .long("no-gametree")
            .help("Do not explore gametree.")
            .long_help("Only distribute the cards across the players, but do not explore the gametree.")
        )
        .help_heading(None)
        .arg(clap::Arg::new("verbose")
            .long("verbose")
            .short('v')
            .help("Show more output")
        )
        .arg(clap::Arg::new("strategy")
            .long("strategy")
            .takes_value(true)
            .help("Restrict to one specific strategy")
            .possible_values(["maxmin", "maxselfishmin"])
        )
        .arg(clap::Arg::new("visualize")
            .long("visualize")
            .takes_value(true)
            .help("Output game trees as HTML")
        )
        .arg(clap::Arg::new("points") // TODO? also support by stichs
            .long("points")
            .help("Use points as criterion")
            .long_help("When applicable (e.g. for Solo, Rufspiel), investigate the points reached instead of the raw payout.")
        )
        .arg(clap::Arg::new("json")
            .long("json")
            .help("Output result as json")
        )
        .arg(clap::Arg::new("inspect")
            .long("inspect")
            .takes_value(true)
            .multiple_occurrences(true)
            .help("Describes inspection target")
            .long_help("Describes what the software will inspect. Example: \"ctx.ea(0)\" checks if player 0 has Eichel-Ass, \"ctx.trumpf(2)\" counts the trumpf cards held by player 2. (Players are numbere from 0 to 3, where 0 is the player to open the first stich (1, 2, 3 follow accordingly).)") // TODO improve docs.
        )
        // TODO support "compute optimal game tree"
}

#[derive(new, Serialize)]
struct SJsonTableLine<TplStrategies: TTplStrategies>
    where
        SPerMinMaxStrategyGeneric<Vec<((isize/*n_payout*/, char/*chr_loss_or_win*/), usize/*n_count*/)>, TplStrategies>: Serialize,
{
    ostr_header: Option<String>,
    perminmaxstrategyvecpayout_histogram: SPerMinMaxStrategyGeneric<Vec<((isize/*n_payout*/, char/*chr_loss_or_win*/), usize/*n_count*/)>, TplStrategies>,
}

#[derive(new, Serialize)]
struct SJson<TplStrategies: TTplStrategies> {
    str_rules: String,
    str_stichseq: String,
    ocard_played: Option<ECard>,
    astr_hand: [String; EPlayerIndex::SIZE],
    vectableline: Vec<SJsonTableLine<TplStrategies>>,
}

fn json_histograms<TplStrategies: TTplStrategies>(payoutstatsperstrategy: &SPerMinMaxStrategyGeneric<SPayoutStats<std::cmp::Ordering>, TplStrategies>)
    -> SPerMinMaxStrategyGeneric<Vec<((isize, char), usize)>, TplStrategies>
{
    payoutstatsperstrategy.map(|payoutstats| 
        payoutstats.histogram().iter()
            .map(|((n_payout, ord_vs_0), n_count)| ( 
                (
                    *n_payout,
                    match ord_vs_0 {
                        std::cmp::Ordering::Less => '-',
                        std::cmp::Ordering::Equal => '\u{00b1}', // plus-minus 0
                        std::cmp::Ordering::Greater => '+',
                    },
                ),
                *n_count,
            ))
            .collect()
    )
}

enum EBranching {
    Branching(usize, usize),
    Equivalent(usize, SCardsPartition),
    Oracle,
    OnePerWinnerIndex(Option<EPlayerIndex>),
}

#[derive(Clone)]
enum ESingleStrategy {
    MaxMin,
    MaxSelfishMin,
}

fn make_snapshot_cache<TplStrategies: TTplStrategies>(rules: &SRules) -> impl Fn(&SRuleStateCacheFixed) -> Box<dyn TSnapshotCache<SPerMinMaxStrategyRawPayout<TplStrategies>>> + '_ {
    move |rulestatecache| rules.snapshot_cache::<TplStrategies>(rulestatecache)
}

#[allow(clippy::extra_unused_type_parameters)]
fn make_snapshot_cache_none<TplStrategies>(_rules: &SRules) -> impl Fn(&SRuleStateCacheFixed)->SSnapshotCacheNone {
    SSnapshotCacheNone::factory()
}

fn run_internal<
    'stichseq,
    'rules,
    FilterAllowedCards: TFilterAllowedCards,
    TplStrategies: TTplStrategies,
    AlphaBetaPruner: TAlphaBetaPruner+Sync,
    Pruner: TPruner+Sync,
    SnapshotCache: TSnapshotCache<<SMinReachablePayoutBase<'rules, Pruner, TplStrategies, AlphaBetaPruner> as TForEachSnapshot>::Output>,
    OSnapshotCache: Into<Option<SnapshotCache>>,
    SnapshotVisualizer: TSnapshotVisualizer<<SMinReachablePayoutBase<'rules, Pruner, TplStrategies, AlphaBetaPruner> as TForEachSnapshot>::Output>,
    OFilterAllowedCards: Into<Option<FilterAllowedCards>>,
>(
    b_verbose: bool,
    clapmatches: &clap::ArgMatches,
    ahand_fixed_with_holes: &EnumMap<EPlayerIndex, SHand>,
    rules: &SRules,
    epi_position: EPlayerIndex,
    expensifiers: &SExpensifiers,

    stichseq: &'stichseq SStichSequence,
    ocard_played: Option<ECard>,
    itahand: Box<dyn Iterator<Item=EnumMap<EPlayerIndex, SHand>> + Send + 'stichseq>,
    fn_make_filter: impl Fn(&SStichSequence, &EnumMap<EPlayerIndex, SHand>)->OFilterAllowedCards + std::marker::Sync,

    fn_alphabetapruner: impl Fn(&SStichSequence, &EnumMap<EPlayerIndex, SHand>)->AlphaBetaPruner + std::marker::Sync,

    fn_snapshotcache: impl Fn(&SRuleStateCacheFixed) -> OSnapshotCache + std::marker::Sync,
    fn_visualizer: impl Fn(usize, &EnumMap<EPlayerIndex, SHand>, Option<ECard>) -> SnapshotVisualizer + std::marker::Sync,
    fn_payout: &(impl Fn(&SStichSequence, &EnumMap<EPlayerIndex, SHand>, isize)->(isize, std::cmp::Ordering) + Sync),
    slcconstraint: &[SConstraint],
) -> Result<(), Error>
{
    let inspectionstatistics = Arc::new(Mutex::new(SInspectionStatistics::new(slcconstraint, stichseq, rules)));
    let fn_loss_or_win = |_n_payout, ord_vs_0| ord_vs_0;
    let ovecinterimres_verbose = if_then_some!(b_verbose, Arc::new(Mutex::new(Vec::<SInterimResult<TplStrategies>>::new())));
    let determinebestcardresult = determine_best_card(
        stichseq,
        itahand,
        fn_make_filter,
        /*fn_make_foreachsnapshot*/&|stichseq, ahand| <SMinReachablePayoutBase<Pruner, TplStrategies, _>>::new_with_pruner(
            rules,
            epi_position,
            expensifiers.clone(),
            fn_alphabetapruner(stichseq, ahand),
        ),
        fn_snapshotcache,
        fn_visualizer,
        /*fn_inspect*/&|inspectionpoint, i_ahand, ahand| {
            match inspectionpoint {
                VInspectionPoint::Card{b_before, card} => {
                    if b_verbose {
                        println!(" {} {} ({}): {}",
                            if *b_before {'>'} else {'<'},
                            i_ahand+1, // TODO use same hand counters as in common_given_game
                            card,
                            display_card_slices(ahand, rules, " | "),
                        );
                    }
                },
                VInspectionPoint::AfterHand(mapcardopayoutstats) => {
                    unwrap!(inspectionstatistics.lock()).update_inspection_statistics(
                        ahand,
                        // TODO respect mapcardopayoutstats
                    );
                    if let Some(ref vecinterimres) = ovecinterimres_verbose {
                        let fn_cmp_interim_result = |lhs: &SInterimResult<TplStrategies>, rhs: &SInterimResult<TplStrategies>| {
                            rhs.payoutstats.compare_canonical(&lhs.payoutstats, fn_loss_or_win)
                        };
                        let mut vecinterimres = unwrap!(vecinterimres.lock());
                        let n_count_before = vecinterimres.len();
                        assert!(vecinterimres.is_sorted_by(fn_cmp_to_fn_le(fn_cmp_interim_result)));
                        // Remember old ranks and positions
                        let mut mapcardoposandrank_old = ECard::map_from_fn(|_| None);
                        for_each_interim_result(&mut vecinterimres, fn_cmp_interim_result, |posandrank, interimres| {
                            verify!(mapcardoposandrank_old[interimres.card].replace(posandrank).is_none()); // Implies that each card occured only once
                        });
                        // Copy over/update values that have been present in previous iteration
                        for (card, payoutstats) in internal_cards_and_ts(mapcardopayoutstats) {
                            if let Some(posandrank) = &mapcardoposandrank_old[card] {
                                assert_eq!(vecinterimres[posandrank.i_position].card, card);
                                vecinterimres[posandrank.i_position].payoutstats = payoutstats.clone(); // Update to new value
                            } else {
                                vecinterimres.push(SInterimResult{ornkchg: None, card, payoutstats: payoutstats.clone()});
                            }
                        }
                        // Compute rank changes - only on already known entries.
                        let slcinterimres_already_present = &mut vecinterimres[0..n_count_before];
                        slcinterimres_already_present.sort_by(&fn_cmp_interim_result);
                        for_each_interim_result(slcinterimres_already_present, fn_cmp_interim_result, |posandrank, interimres| {
                            interimres.ornkchg = Some(match posandrank.n_rank.cmp(&unwrap!(mapcardoposandrank_old[interimres.card].as_ref()).n_rank) {
                                std::cmp::Ordering::Less => VRankChange::Change(ELoHi::Lo),
                                std::cmp::Ordering::Greater => VRankChange::Change(ELoHi::Hi),
                                std::cmp::Ordering::Equal => VRankChange::Equal(match interimres.ornkchg {
                                    None | Some(VRankChange::Change(_)) => 1,
                                    Some(VRankChange::Equal(n_iterations)) => n_iterations + 1,
                                })
                            });
                        });
                        if n_count_before<vecinterimres.len() {
                            vecinterimres.sort_by(&fn_cmp_interim_result);
                        } else {
                            assert!(vecinterimres.is_sorted_by(fn_cmp_to_fn_le(fn_cmp_interim_result)));
                        }
                        print_payoutstatstable::<_,TplStrategies>(
                            &internal_table(
                                vecinterimres.iter()
                                    .map(|SInterimResult{ornkchg, card, payoutstats}| (
                                        format!("{} {}",
                                            match ornkchg {
                                                None => "".to_string(),
                                                Some(VRankChange::Change(ELoHi::Lo)) => "^".to_string(),
                                                Some(VRankChange::Equal(n_iterations)) => format!("=({n_iterations})"),
                                                Some(VRankChange::Change(ELoHi::Hi)) => "v".to_string(),
                                            },
                                            card,
                                        ),
                                        payoutstats,
                                    ))
                                    .collect(),
                                /*b_group*/false,
                                &fn_loss_or_win,
                            ),
                            /*b_print_table_description_before_table*/false,
                            /*fn_mark_played*/|_str_card| false, // TODO Mark played card in intermediate result
                        );
                    }
                },
            }
        },
        fn_payout,
    ).ok_or_else(||format_err!("Could not determine best card. Apparently could not generate valid hands."))?;
    if clapmatches.is_present("json") {
        // TODO output inspectionstatistics
        println!("{}", unwrap!(serde_json::to_string(
            &SJson::new(
                /*str_rules*/SDisplayRules::new(rules, /*b_include_playerindex*/true).to_string(),
                /*vec_stichseq*/stichseq.visible_cards().map(|(_epi, &card)| card).join(" "),
                ocard_played,
                /*str_hand*/ahand_fixed_with_holes.map(|hand|
                    SDisplayCardSlice::new(hand.cards().clone(), rules).to_string()
                ).into_raw(),
                /*vectableline*/itertools::chain(
                    determinebestcardresult.cards_and_ts()
                        .map(|(card, payoutstatsperstrategy)|
                            SJsonTableLine::new(
                                /*ostr_header*/Some(card.to_string()),
                                /*perminmaxstrategyvecpayout_histogram*/json_histograms::<TplStrategies>(payoutstatsperstrategy),
                            )
                        ),
                    std::iter::once(SJsonTableLine::new(
                        /*ostr_header*/Some("no-details".to_string()),
                        /*perminmaxstrategyvecpayout_histogram*/json_histograms::<TplStrategies>(&determinebestcardresult.t_combined),
                    )),
                ).collect::<Vec<SJsonTableLine<TplStrategies>>>(),
            ),
        )));
    } else {
        let inspectionstatistics = finalize_arc_mutex(inspectionstatistics);
        print_card_distribution_statistics(
            stichseq,
            rules,
            &inspectionstatistics.mapcardsetepi_distribution,
        );
        let payoutstatstable = table(
            &determinebestcardresult,
            rules,
            &fn_loss_or_win,
        );
        print_payoutstatstable::<_,TplStrategies>(
            &payoutstatstable,
            /*b_print_table_description_before_table*/b_verbose,
            /*fn_mark_played*/|card| Some(*card)==ocard_played,
        );
        println!("-----");
        print_payoutstatstable::<_,TplStrategies>(
            &internal_table(
                vec!(("no-details", determinebestcardresult.t_combined)),
                /*b_group*/false,
                &fn_loss_or_win,
            ),
            /*b_print_table_description_before_table*/false,
            /*fn_mark_played*/|_| false, // Do not mark played card in "combined" line
        );
        print_inspection_results(
            b_verbose,
            inspectionstatistics,
        );
    }
    Ok(())
}

#[derive(Debug, Clone)]
enum VRankChange { // Lower ranks considered better.
    Change(ELoHi),
    Equal(usize/*n_iterations*/),
}

struct SInterimResult<TplStrategies: TTplStrategies> {
    ornkchg: Option<VRankChange>,
    card: ECard,
    payoutstats: SPerMinMaxStrategyGeneric<SPayoutStats<std::cmp::Ordering>, TplStrategies>,
}

struct SPositionAndRank {
    i_position: usize,
    n_rank: usize,
}

fn for_each_interim_result<TplStrategies: TTplStrategies>( // TODORUST generic closure
    slcinterimres: &mut [SInterimResult<TplStrategies>], // TODO Taking mut here is unfortunate, but I did not see a simple way out of this without duplication
    fn_cmp_interim_result: impl FnMut(&SInterimResult<TplStrategies>, &SInterimResult<TplStrategies>)->std::cmp::Ordering,
    mut fn_callback: impl FnMut(SPositionAndRank, &mut SInterimResult<TplStrategies>),
) {
    let mut n_rank = 0;
    for slcinterimres_chunk in slcinterimres
        .chunk_by_mut(fn_cmp_to_fn_eq(fn_cmp_interim_result))
    {
        for (i_position, interimres) in (n_rank..).zip(slcinterimres_chunk.iter_mut()) {
            fn_callback(SPositionAndRank{i_position, n_rank}, interimres);
        }
        n_rank += slcinterimres_chunk.len();
    }
}

fn print_inspection_results(
    b_verbose: bool,
    SInspectionStatistics{vectplinspectionhistogramconstraint, n_ahand_total, ..}: SInspectionStatistics,
) {
    let percentage = |n_count: usize| n_count.as_num::<f64>()/n_ahand_total.as_num::<f64>();
    let b_more_than_one_constraint = 1<vectplinspectionhistogramconstraint.len();
    for (inspectionhistogram, constraint) in vectplinspectionhistogramconstraint {
        if b_verbose || b_more_than_one_constraint {
            println!("{constraint}");
        }
        let mut oresinspectionresult_weighted_sum = None;
        for (resinspectionresult, n_count) in inspectionhistogram.into_iter()
            .sorted_unstable_by(|lhs, rhs| Ord::cmp(&lhs.0, &rhs.0))
        {
            let str_result_or_err = match resinspectionresult {
                Ok(inspectionresult) => {
                    if let Ok(inspectionresult_weighted_sum) = oresinspectionresult_weighted_sum.get_or_insert_with(||
                        Ok(inspectionresult.map_numbers_remove_unknown(&|_| 0.,)) // Determine structure, initialize numbers with 0
                    ) {
                        inspectionresult_weighted_sum.accumulate_weighted_sum(
                            &inspectionresult.map_numbers_remove_unknown(&|number| number.to_total_ordered_float().0),
                            percentage(n_count),
                        );
                    }
                    format!("{inspectionresult}")
                },
                Err(str_err) => {
                    oresinspectionresult_weighted_sum = Some(Err(())); // Do not show weighted sum if there are errors.
                    str_err
                },
            };
            println!("{} {} ({:.2}%)", str_result_or_err, n_count, percentage(n_count)*100.);
        }
        if let Some(Ok(inspectionresult_weighted_sum))=oresinspectionresult_weighted_sum {
            println!("-----");
            println!("\u{2300} {inspectionresult_weighted_sum:.4}");
        }
    }
}

type InspectionHistogram = std::collections::HashMap<Result<VInspectionResult<VRecognizableAsNumber, String>, String>, usize>;
struct SInspectionStatistics<'lifetime> {
    n_ahand_total: u64,
    vectplinspectionhistogramconstraint: Vec<(InspectionHistogram, &'lifetime SConstraint)>,
    mapcardsetepi_distribution: EnumMap<ECard, EnumSet<EPlayerIndex>>,
    stichseq: &'lifetime SStichSequence,
    rules: &'lifetime SRules,
}

impl<'lifetime> SInspectionStatistics<'lifetime> {
    fn new(slcconstraint: &'lifetime [SConstraint], stichseq: &'lifetime SStichSequence, rules: &'lifetime SRules) -> Self {
        Self {
            n_ahand_total: 0,
            vectplinspectionhistogramconstraint: slcconstraint
                .iter()
                .map(|constraint| (InspectionHistogram::new(), constraint))
                .collect(),
            mapcardsetepi_distribution: ECard::map_from_fn(|_card| EnumSet::<EPlayerIndex>::new_empty()),
            stichseq,
            rules,
        }
    }

    fn update_inspection_statistics(
        &mut self,
        ahand: &EnumMap<EPlayerIndex, SHand>,
    ) {
        for (inspectionhistogram, constraint) in self.vectplinspectionhistogramconstraint.iter_mut() {
            // TODO: Should we evaluate result without holding lock?
            *inspectionhistogram.entry(
                constraint.internal_eval(
                    self.stichseq,
                    ahand,
                    self.rules.clone(),
                )
                    .map(VInspectionResult::new)
                    .map_err(|err| format!("Error: {err:?}")),
            ).or_insert(0) += 1;
        }
        self.n_ahand_total += 1;
        for epi in EPlayerIndex::values() {
            for &card in ahand[epi].cards() {
                self.mapcardsetepi_distribution[card].insert(epi);
            }
        }
    }
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let vecconstraint : Vec<SConstraint> = clapmatches.values_of("inspect")
        .map(|itstr_inspect|
            itstr_inspect.map(|str_inspect| /*-> Result<_, Error>*/ {
                str_inspect.parse::<SConstraint>()
                    .map_err(|_| format_err!("Cannot parse inspection target."))
            }).collect::<Result<_,_>>()
        )
        .transpose()?
        .unwrap_or_default();
    with_common_args(
        clapmatches,
        |itahand, rules, stichseq, ocard_played, ahand_fixed_with_holes, epi_position, expensifiers, b_verbose| {
            let otplrulesfn_points_as_payout = if clapmatches.is_present("points") {
                if let Some(tplrulesfn_points_as_payout) = rules.points_as_payout() {
                    Some(tplrulesfn_points_as_payout)
                } else {
                    if b_verbose {
                        println!("Rules {} do not support point based variant.", SDisplayRules::new(rules, /*b_include_playerindex*/false));
                    }
                    None
                }
            } else {
                None
            };
            let rules = if let Some((rules, _fn_payout_to_points)) = &otplrulesfn_points_as_payout {
                rules.clone()
            } else {
                rules.clone()
            };
            let rules = &rules;
            if clapmatches.is_present("no-gametree") {
                // TODO can/should we do something with ocard_played?
                let mut inspectionstatistics = SInspectionStatistics::new(&vecconstraint, stichseq, rules);
                for ahand in itahand {
                    inspectionstatistics.update_inspection_statistics(&ahand);
                }
                print_inspection_results(b_verbose, inspectionstatistics);
            } else {
                let fn_human_readable_payout = |stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, epi_position: EPlayerIndex, n_payout: isize| -> (isize, std::cmp::Ordering) {
                    if let Some((_rules, fn_payout_to_points)) = &otplrulesfn_points_as_payout {
                        (
                            fn_payout_to_points(
                                &SRuleStateCacheFixed::new(ahand, stichseq),
                                epi_position,
                                n_payout,
                            ),
                            n_payout.cmp(&0), // Human readable payout may not indicate loss or win: In Rufspiel, if epi_position has 60, it may mean win or loss, depending on whether epi_position is co-player.
                        )
                    } else {
                        (n_payout, n_payout.cmp(&0))
                    }
                };
                use EBranching::*;
                let oesinglestrategy = match clapmatches.value_of("strategy") {
                    Some("maxmin") => Ok(Some(ESingleStrategy::MaxMin)),
                    Some("maxselfishmin") => Ok(Some(ESingleStrategy::MaxSelfishMin)),
                    None => Ok(None),
                    Some(_) => Err(format_err!("Could not understand strategy.")),
                }?;
                // we are interested in payout => single-card-optimization useless
                macro_rules! forward{(
                    (($($func_filter_allowed_cards_ty: tt)*), $func_filter_allowed_cards: expr),
                    ($pruner:ident),
                    ($TplStrategies:ident, $fn_alphabetapruner:expr,),
                    $fn_snapshotcache:ident,
                    $fn_visualizer: expr,
                ) => {{ // TODORUST generic closures
                    run_internal::<$($func_filter_allowed_cards_ty)*,$TplStrategies,_,$pruner,_,_,_,_>( // TODO avoid explicit types
                        b_verbose,
                        clapmatches,
                        ahand_fixed_with_holes,
                        rules,
                        epi_position,
                        expensifiers,
                        stichseq,
                        ocard_played,
                        itahand,
                        $func_filter_allowed_cards,
                        $fn_alphabetapruner,
                        $fn_snapshotcache::<$TplStrategies>(rules),
                        $fn_visualizer,
                        /*fn_payout*/&|stichseq, ahand, n_payout| fn_human_readable_payout(
                            stichseq,
                            ahand,
                            epi_position,
                            n_payout,
                        ),
                        &vecconstraint,
                    )?
                }}}
                let oebranching = if let Some(str_branching) = clapmatches.value_of("branching") {
                    if str_branching.is_empty() {
                        None
                    } else if str_branching=="oracle" {
                        Some(Oracle)
                    } else if let Some(oepi_unfiltered) = str_branching.strip_prefix("oneperwinnerindex")
                        .map(|str_oepi_unfiltered| str_oepi_unfiltered.parse().ok())
                    {
                        Some(OnePerWinnerIndex(oepi_unfiltered))
                    } else if let Some(n_until_stichseq_len) = str_branching.strip_prefix("equiv")
                        .and_then(|str_n_until_remaining_cards| str_n_until_remaining_cards.parse().ok())
                    {
                        Some(Equivalent(n_until_stichseq_len, rules.equivalent_when_on_same_hand()))
                    } else {
                        let [str_lo, str_hi] = str_branching
                            .split(',')
                            .collect_array()
                            .ok_or_else(|| format_err!("Could not parse branching"))?;
                        let (n_lo, n_hi) = (str_lo.trim().parse::<usize>()?, str_hi.trim().parse::<usize>()?);
                        Some(Branching(n_lo, n_hi)) // TODO we should avoid branching in case n_lo is greater than all hand's fixed cards
                    }
                } else {
                    None
                };
                cartesian_match!(
                    forward,
                    match (oebranching) {
                        None => ((_), SNoFilter::factory()),
                        Some(Branching(n_lo, n_hi)) => ((_), {
                            let n_lo = n_lo.max(1);
                            SBranchingFactor::factory(n_lo, n_hi.max(n_lo+1))
                        }),
                        Some(Equivalent(n_until_stichseq_len, cardspartition)) => (
                            (_),
                            equivalent_cards_filter(
                                n_until_stichseq_len,
                                cardspartition.clone(),
                            )
                        ),
                        Some(Oracle) => ((SFilterByOracle), |stichseq, ahand| {
                            SFilterByOracle::new(rules, ahand, stichseq)
                        }),
                        Some(OnePerWinnerIndex(oepi_unfiltered)) => ((_), |_stichseq, _ahand| {
                            SFilterOnePerWinnerIndex::new(
                                oepi_unfiltered,
                                rules,
                            )
                        }),
                    },
                    match (clapmatches.value_of("prune")) {
                        // Some("hint") => (SPrunerViaHint), // TODO re-enable
                        _ => (SPrunerNothing),
                    },
                    match ((oesinglestrategy, clapmatches.is_present("abprune"), rules.alpha_beta_pruner_lohi_values())) {
                        (None, b_abprune, _) => (
                            STplStrategiesAll,
                            {
                                if b_abprune && b_verbose {
                                    println!("Warning: abprune not supported strategy/rules combination. Continuing without.");
                                }
                                |_stichseq, _ahand| SAlphaBetaPrunerNone
                            },
                        ),
                        (Some(ESingleStrategy::MaxMin), false, _) => (
                            STplStrategiesOnlyMaxMin,
                            |_stichseq, _ahand| SAlphaBetaPrunerNone,
                        ),
                        (Some(ESingleStrategy::MaxMin), true, _) => (
                            STplStrategiesOnlyMaxMin,
                            (|_stichseq, _ahand| SAlphaBetaPruner::new({
                                let mut mapepilohi = EPlayerIndex::map_from_fn(|_| ELoHi::Lo);
                                mapepilohi[epi_position] = ELoHi::Hi;
                                mapepilohi
                            })),
                        ),
                        (Some(ESingleStrategy::MaxSelfishMin), b_abprune@false, _) | (Some(ESingleStrategy::MaxSelfishMin), b_abprune@true, None) => (
                            STplStrategiesOnlyMaxSelfishMin,
                            {
                                if b_abprune && b_verbose {
                                    println!("Warning: abprune not supported strategy/rules combination. Continuing without.");
                                }
                                |_stichseq, _ahand| SAlphaBetaPrunerNone
                            },
                        ),
                        (Some(ESingleStrategy::MaxSelfishMin), true, Some(fn_alpha_beta_pruner_lohi_values)) => (
                            STplStrategiesOnlyMaxSelfishMin,
                            (|stichseq, ahand| SAlphaBetaPruner::new({
                                let mut mapepilohi = fn_alpha_beta_pruner_lohi_values(
                                    &SRuleStateCacheFixed::new(ahand, stichseq),
                                );
                                if mapepilohi[epi_position]==ELoHi::Lo {
                                    for lohi in mapepilohi.iter_mut() {
                                        *lohi = -*lohi;
                                    }
                                }
                                assert_eq!(mapepilohi[epi_position], ELoHi::Hi);
                                mapepilohi
                            })),
                        ),
                    },
                    match (clapmatches.is_present("snapshotcache")) { // TODO customizable depth
                        true => make_snapshot_cache,
                        false => make_snapshot_cache_none,
                    },
                    match (clapmatches.value_of("visualize")) {
                        _ => (SNoVisualization::factory()),
                        // Some(str_path) => { // TODO re-enable
                        //     visualizer_factory(
                        //         std::path::Path::new(str_path).to_path_buf(),
                        //         rules,
                        //         epi_position,
                        //     )
                        // },
                    },
                );
            }
            
            Ok(())
        }
    )
}
