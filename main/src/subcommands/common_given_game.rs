use openschafkopf_lib::{
    ai::{
        SAi,
        handiterators::*,
        gametree::EMinMaxStrategy,
    },
    primitives::*,
    rules::{
        ruleset::VStockOrT,
        SDoublings,
        SExpensifiers,
        SStoss,
        TRules,
        TRulesBoxClone,
        parser::parse_rule_description_simple,
    },
};
use openschafkopf_util::*;
use itertools::Itertools;
use failure::*;
use plain_enum::{EnumMap, PlainEnum};
use as_num::*;

pub use super::handconstraint::*;

enum VChooseItAhand {
    All,
    Sample(/*n_samples*/usize, /*on_pool*/Option<usize>),
}

pub fn subcommand_given_game(str_subcommand: &'static str, str_about: &'static str) -> clap::Command<'static> {
    use super::shared_args::*;
    clap::Command::new(str_subcommand)
        .about(str_about)
        .arg(ruleset_arg())
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
            .long_help("Position of the player. Players are numbere from 0 to 3, where 0 is the player to open the first stich (1, 2, 3 follow accordingly). If not given, this is assumed to be the one to play the next card.")
            .takes_value(true)
        )
        .arg(clap::Arg::new("hand")
            .long("hand")
            .takes_value(true)
            .required(true)
            .multiple_occurrences(true)
            .help("The cards on someone's hand")
            .long_help("The cards on the current player's hand (simply separated by spaces, such as \"eo go ho so eu gu hu su\" for a Sie), or the hands of all players. Specifying all player's hands works by first listing cards of player 0, then player 1, then player 2, then player 3 (Example: \"ea ez  ga gz  ha hz  sa sz\" means player 0 has Eichel-Ass and Eichel-Zehn, player 1 has Gras-Ass and Gras-Zehn, and so forth). You can use underscore to leave \"holes\" in other players' hands (Example: \"ea __  ga __  ha __  sa __\" means player 0 has Eichel-Ass and another unknown card, player 1 has Gras-Ass and unknown card, and so forth).")
        )
        .arg(clap::Arg::new("cards_on_table")
            .long("cards-on-table") // TODO rename to played-cards
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
        .arg(clap::Arg::new("simulate_hands")
            .long("simulate-hands")
            .takes_value(true)
            .help("Number of hands to simulate")
            .long_help("Number of unknown hands to simulate. Can either be a number or \"all\", causing the software to generate all possible combinations.")
        )
        .arg(clap::Arg::new("verbose")
            .long("verbose")
            .short('v')
            .help("Show more output")
        )
        .arg(clap::Arg::new("constrain_hands")
            .long("constrain-hands")
            .takes_value(true)
            .help("Constrain simulated hands")
            .long_help("Constrain simulated hands so that certain criteria are fulfilled. Example: \"4<ctx.trumpf(0) && ctx.ea(1)\" only considers card distributions where player 0 has more than 4 Trumpf and player 1 has Eichel-Ass. (Players are numbere from 0 to 3, where 0 is the player to open the first stich (1, 2, 3 follow accordingly).)") // TODO improve docs
        )
}

pub fn with_common_args<FnWithArgs>(
    clapmatches: &clap::ArgMatches,
    mut fn_with_args: FnWithArgs,
) -> Result<(), Error>
    where
        for<'rules> FnWithArgs: FnMut(
            Box<dyn Iterator<Item=EnumMap<EPlayerIndex, SHand>>+Send+'rules>,
            &'rules dyn TRules,
            &SStichSequence,
            &EnumMap<EPlayerIndex, SHand>, // TODO? Good idea? Could this simply given by itahand?
            EPlayerIndex/*epi_position*/,
            &SExpensifiers,
            bool/*b_verbose*/,
        ) -> Result<(), Error>,
{
    let b_verbose = clapmatches.is_present("verbose");
    let veccard_stichseq = match clapmatches.value_of("cards_on_table") { // TODO allow multiple stichseq (in particular something like "ea | ez ek e9  sa sz | sk s9" so that the user can query intermittent game states).
        None => Vec::new(),
        Some(str_cards_on_table) => cardvector::parse_cards(str_cards_on_table)
            .ok_or_else(||format_err!("Could not parse played cards"))?,
    };
    let vectplvecocardstr_ahand = unwrap!(clapmatches.values_of("hand"))
        .map(|str_ahand| 
            cardvector::parse_optional_cards::<Vec<_>>(str_ahand)
                .ok_or_else(||format_err!("Could not parse hand: {}", str_ahand))
                .map(|vecocard| (vecocard, str_ahand))
        )
        .collect::<Result<Vec<_>, _>>()?;
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
        Some(Err(e)) => bail!("Could not parse stoss: {}", e),
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
            bail!("Cards are used more than once: {}", veccard_duplicate.iter().join(", "));
        }
        let (itrules, b_single_rules) = match clapmatches.values_of("rules")
            .map(|values| values.map(parse_rule_description_simple))
            .into_iter()
            .flatten()
            .collect::<Result<Vec<_>,_>>()
        {
            Ok(vecrules) => {
                if vecrules.is_empty() {
                    let ruleset = super::get_ruleset(clapmatches)?;
                    (
                        Box::new(ruleset
                            .avecrulegroup.into_raw().into_iter()
                            .flat_map(|vecrulegroup|
                                vecrulegroup.into_iter().flat_map(|rulegroup| {
                                    rulegroup.vecorules.into_iter()
                                        .filter_map(|orules|
                                            orules.as_ref().map(|rules|
                                                TRulesBoxClone::box_clone(rules.upcast())
                                            )
                                        )
                                })
                            )
                            .chain(match ruleset.stockorramsch {
                                VStockOrT::Stock(_) => None,
                                VStockOrT::OrT(rules) => Some(rules)
                            })
                        ) as Box<dyn Iterator<Item=Box<dyn TRules>>>,
                        /*b_single_rules*/false,
                    )
                } else {
                    let b_single_rules = vecrules.len()==1;
                    (Box::new(vecrules.into_iter()) as Box<dyn Iterator<Item=Box<dyn TRules>>>, b_single_rules)
                }
            },
            Err(err) => {
                bail!("Could not parse rules: {}", err);
            },
        };
        for rules in itrules {
            let rules = rules.as_ref();
            let (stichseq, ahand_with_holes, epi_position) = EKurzLang::values()
                .filter_map(|ekurzlang| {
                    let mut stichseq = SStichSequence::new(ekurzlang);
                    for &card in veccard_stichseq.iter() {
                        if !ekurzlang.supports_card(card)
                            || stichseq.game_finished()
                        {
                            return None; // TODO? distinguish error
                        }
                        stichseq.zugeben(card, rules);
                    }
                    let epi_position = clapmatches.value_of_t("position")
                        .unwrap_or_else(|_|unwrap!(stichseq.current_stich().current_playerindex()));
                    if_then_some!(
                        stichseq.remaining_cards_per_hand()[epi_position]==vecocard_hand.len(), // TODO Allow to specify more than only currently held cards if compatible with stichseq
                        SHand::new_from_iter(vecocard_hand.iter().flatten())
                            .to_ahand(epi_position)
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
                .exactly_one()
                .map_err(|err| format_err!("Could not determine ekurzlang: {}", err))?;
            // TODO check that everything is ok (no duplicate cards, cards are allowed, current stich not full, etc.)
            if let Some(epi_active) = rules.playerindex() {
                let veccard_hand_active = stichseq.cards_from_player(&ahand_with_holes[epi_active], epi_active)
                    .collect::<Vec<_>>();
                if veccard_hand_active.len()==stichseq.kurzlang().cards_per_player() {
                    if !rules.can_be_played(SFullHand::new(&veccard_hand_active, stichseq.kurzlang())) {
                        if b_single_rules {
                            bail!("Rules {} cannot be played given these cards.", rules);
                        } else {
                            if b_verbose {
                                println!("Rules {} cannot be played given these cards.", rules);
                            }
                            continue;
                        }
                    }
                } else {
                    // let hand iterators try to generate valid hands.
                }
            }
            let oconstraint = if_then_some!(let Some(str_constrain_hands)=clapmatches.value_of("constrain_hands"), {
                let relation = str_constrain_hands.parse::<SConstraint>().map_err(|_|format_err!("Cannot parse hand constraints"))?;
                if b_verbose {
                    println!("Constraint parsed as: {}", relation);
                }
                relation
            });
            let mapepin_cards_per_hand = stichseq.remaining_cards_per_hand();
            use VChooseItAhand::*;
            let iteratehands = if_then_some!(let Some(str_itahand)=clapmatches.value_of("simulate_hands"),
                if "all"==str_itahand.to_lowercase() { // TODO replace this case by simply "0"?
                    All
                } else {
                    match str_itahand
                        .split('/')
                        .map(|str_n| str_n.parse().ok())
                        .collect::<Option<Vec<_>>>()
                        .as_deref()
                    {
                        Some(&[n_samples]) => Sample(n_samples, /*on_pool*/None),
                        Some(&[n_samples, n_pool]) => Sample(n_samples, Some(n_pool)),
                        _ => bail!("Failed to parse simulate_hands"),
                    }
                }
            ).unwrap_or_else(|| {
                All
            });
            for epi in EPlayerIndex::values() {
                assert!(ahand_with_holes[epi].cards().len() <= mapepin_cards_per_hand[epi]);
            }
            macro_rules! forward{($n_ahand_total: expr, $itahand_factory: expr, $fn_take: expr) => {{ // TODORUST generic closures
                let mut n_ahand_seen = 0;
                let mut n_ahand_valid = 0;
                if b_verbose || !b_single_rules {
                    println!("Rules: {}", rules);
                }
                if b_verbose || 1</*b_single_itahand*/vectplvecocardstr_ahand.len() {
                    println!("Hand(s): {}", str_ahand);
                }
                fn_with_args(
                    Box::new(
                        #[allow(clippy::redundant_closure_call)]
                        $fn_take($itahand_factory(
                            &stichseq,
                            ahand_with_holes.clone(),
                            epi_position,
                            rules,
                            &expensifiers.vecstoss,
                            /*fn_inspect*/|b_valid_so_far, ahand| {
                                n_ahand_seen += 1;
                                let b_valid = b_valid_so_far
                                    && oconstraint.as_ref().map_or(true, |relation|
                                        relation.eval(ahand, rules.box_clone())
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
                                        display_card_slices(&ahand, &rules, " | "),
                                    )
                                }
                                b_valid
                            }
                        ))
                    ),
                    rules,
                    &stichseq,
                    &ahand_with_holes,
                    epi_position,
                    &expensifiers,
                    b_verbose,
                )?;
            }}}
            match (iteratehands, rules.playerindex()) {
                (All, _oepi_active) => {
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
                (Sample(n_samples, None), _oepi_active) => {
                    forward!(n_samples, internal_forever_rand_hands, |itahand| Iterator::take(itahand, n_samples))
                },
                (Sample(n_samples, Some(_n_pool)), None) => {
                    forward!(n_samples, internal_forever_rand_hands, |itahand| Iterator::take(itahand, n_samples))
                },
                (Sample(n_samples, Some(n_pool)), Some(epi_active)) => {
                    forward!(
                        n_samples,
                        internal_forever_rand_hands,
                        |itahand_pool| {
                            let mut vectplahandpayout = Iterator::take(itahand_pool, n_pool)
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
                                        &expensifiers,
                                    )[EMinMaxStrategy::SelfishMin].avg();
                                    (ahand, payout)
                                })
                                .collect::<Vec<_>>();
                            vectplahandpayout.sort_unstable_by(|tplahandpayout_lhs, tplahandpayout_rhs| unwrap!(tplahandpayout_rhs.1.partial_cmp(&tplahandpayout_lhs.1)));
                            vectplahandpayout.into_iter()
                                .take(n_samples)
                                .map(|tplahandpayout| tplahandpayout.0)
                        }
                    )
                },
            };
        }
    }
    Ok(())
}

