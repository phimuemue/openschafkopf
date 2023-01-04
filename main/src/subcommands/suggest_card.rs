use crate::ai::{*, gametree::*, stichoracle::SFilterByOracle, cardspartition::*};
use crate::primitives::*;
use crate::util::*;
use itertools::*;
use crate::game_analysis::determine_best_card_table::{table, internal_table};
use rayon::prelude::*;
use crate::rules::{SExpensifiers};

use super::common_given_game::*;

pub fn subcommand(str_subcommand: &'static str) -> clap::Command {
    subcommand_given_game(str_subcommand, "Suggest a card to play given the game so far")
        .arg(clap::Arg::new("repeat_hands")
            .long("repeat-hands")
            .takes_value(true)
            .help("Repeat each simulated card distribution")
        )
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
        .arg(clap::Arg::new("snapshotcache")
            .long("snapshotcache")
            .help("Use snapshot cache")
            .long_help("Use snapshot cache to possibly speed up game tree exploration.")
        )
        .arg(clap::Arg::new("no-details")
            .long("no-details")
            .help("Do not investigate cards separately")
        )
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    with_common_args(
        clapmatches,
        |itahand, rules, stichseq, ahand_fixed_with_holes, epi_position, b_verbose| {
            let otplrulesfn_points_as_payout = if clapmatches.is_present("points") {
                if let Some(tplrulesfn_points_as_payout) = rules.points_as_payout() {
                    Some(tplrulesfn_points_as_payout)
                } else {
                    if b_verbose { // TODO? dispatch statically
                        println!("Rules {} do not support point based variant.", rules);
                    }
                    None
                }
            } else {
                None
            };
            let rules = if let Some((rules, _fn_payout_to_points)) = &otplrulesfn_points_as_payout {
                rules.as_ref()
            } else {
                rules
            };
            let fn_human_readable_payout = |f_payout| {
                if let Some((_rules, fn_payout_to_points)) = &otplrulesfn_points_as_payout {
                    fn_payout_to_points(
                        stichseq,
                        (epi_position, &ahand_fixed_with_holes[epi_position]),
                        f_payout
                    )
                } else {
                    f_payout
                }
            };
            let expensifiers = SExpensifiers::new_no_stock_doublings_stoss(); // TODO? make customizable
            let epi_current = unwrap!(stichseq.current_stich().current_playerindex());
            enum EBranching {
                Branching(usize, usize),
                Equivalent(usize, SCardsPartition),
                Oracle,
            }
            use EBranching::*;
            macro_rules! forward_with_args{($forward:ident) => {
                cartesian_match!(
                    forward,
                    match (
                        if_then_some!(let Some(str_branching) = clapmatches.value_of("branching"), {
                            if str_branching=="oracle" {
                                Oracle
                            } else if let Some(n_until_stichseq_len) = str_branching.strip_prefix("equiv")
                                .and_then(|str_n_until_remaining_cards| str_n_until_remaining_cards.parse().ok())
                            {
                                Equivalent(n_until_stichseq_len, rules.equivalent_when_on_same_hand())
                            } else {
                                let (str_lo, str_hi) = str_branching
                                    .split(',')
                                    .collect_tuple()
                                    .ok_or_else(|| format_err!("Could not parse branching"))?;
                                let (n_lo, n_hi) = (str_lo.trim().parse::<usize>()?, str_hi.trim().parse::<usize>()?);
                                Branching(n_lo, n_hi) // TODO we should avoid branching in case n_lo is greater than all hand's fixed cards
                            }
                        })
                    ) {
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
                    },
                    match (clapmatches.value_of("prune")) {
                        Some("hint") => (SMinReachablePayoutLowerBoundViaHint),
                        _ => (SMinReachablePayout),
                    },
                    match (clapmatches.is_present("snapshotcache")) { // TODO customizable depth
                        true => (
                            (|rulestatecache| rules.snapshot_cache(rulestatecache))
                        ),
                        false => ((SSnapshotCacheNone::factory())),
                    },
                    match (clapmatches.value_of("visualize")) {
                        None => (SNoVisualization::factory()),
                        Some(str_path) => {
                            visualizer_factory(
                                std::path::Path::new(str_path).to_path_buf(),
                                rules,
                                epi_position,
                            )
                        },
                    },
                )
            }}
            if epi_current!=epi_position || clapmatches.is_present("no-details") {
                macro_rules! forward{((($($func_filter_allowed_cards_ty: tt)*), $func_filter_allowed_cards: expr), ($foreachsnapshot: ident), ($fn_snapshotcache:expr), $fn_visualizer: expr,) => {{ // TODORUST generic closures
                    SPerMinMaxStrategy(itahand
                        .enumerate()
                        .par_bridge() // TODO can we derive a true parallel iterator?
                        .map(|(i_ahand, mut ahand)| {
                            let mut visualizer = $fn_visualizer(i_ahand, &ahand, /*ocard*/None);
                            explore_snapshots::<_,$($func_filter_allowed_cards_ty)*,_,_,_>(
                                (&mut ahand, &mut stichseq.clone()),
                                rules,
                                &$func_filter_allowed_cards,
                                &$foreachsnapshot::new(
                                    rules,
                                    epi_position,
                                    expensifiers.clone(),
                                ),
                                &$fn_snapshotcache,
                                &mut visualizer,
                            ).0.map(|mapepiminmax| {
                                SPayoutStats::new_1(mapepiminmax[epi_position])
                            })
                        })
                        .reduce(
                            /*identity*/|| EMinMaxStrategy::map_from_fn(|_|SPayoutStats::new_identity_for_accumulate()),
                            /*op*/mutate_return!(|mapemmstrategypayoutstats_lhs, mapemmstrategypayoutstats_rhs| {
                                for emmstrategy in EMinMaxStrategy::values() {
                                    mapemmstrategypayoutstats_lhs[emmstrategy].accumulate(&mapemmstrategypayoutstats_rhs[emmstrategy]);
                                }
                            }),
                        ))
                }}}
                let mapemmstrategypaystats = forward_with_args!(forward);
                // TODO this prints a table for each iterated rules, but we want to only print one table
                internal_table(
                    vec![(rules, mapemmstrategypaystats)],
                    /*b_group*/false,
                    &fn_human_readable_payout,
                ).print(b_verbose);
            } else {
                let determinebestcardresult = { // we are interested in payout => single-card-optimization useless
                    macro_rules! forward{((($($func_filter_allowed_cards_ty: tt)*), $func_filter_allowed_cards: expr), ($foreachsnapshot: ident), ($fn_snapshotcache:expr), $fn_visualizer: expr,) => {{ // TODORUST generic closures
                        let n_repeat_hand = clapmatches.value_of("repeat_hands").unwrap_or("1").parse()?;
                        determine_best_card::<$($func_filter_allowed_cards_ty)*, _, _, _, _, _>( // TODO avoid explicit types
                            stichseq,
                            rules,
                            Box::new(
                                itahand
                                    .flat_map(|ahand| {
                                        itertools::repeat_n(
                                            ahand,
                                            n_repeat_hand,
                                        )
                                    })
                            ) as Box<_>,
                            $func_filter_allowed_cards,
                            &$foreachsnapshot::new(
                                rules,
                                verify_eq!(epi_position, epi_current),
                                expensifiers.clone(),
                            ),
                            $fn_snapshotcache,
                            $fn_visualizer,
                            /*fn_inspect*/&|b_before, i_ahand, ahand, card| {
                                if b_verbose {
                                    println!(" {} {} ({}): {}",
                                        if b_before {'>'} else {'<'},
                                        i_ahand+1, // TODO use same hand counters as in common_given_game
                                        card,
                                        display_card_slices(&ahand, &rules, " | "),
                                    );
                                }
                            },
                        )
                    }}}
                    forward_with_args!(forward)
                }.ok_or_else(||format_err!("Could not determine best card. Apparently could not generate valid hands."))?;
                table(
                    &determinebestcardresult,
                    rules,
                    &fn_human_readable_payout,
                )
                    .print(
                        b_verbose,
                    );
            }
            Ok(())
        }
    )
}
