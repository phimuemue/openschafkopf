use openschafkopf_lib::{
    ai::{*, gametree::*, stichoracle::SFilterByOracle, cardspartition::*},
    primitives::*,
    game_analysis::determine_best_card_table::{
        table,
        internal_table,
        SFormatInfo,
        SOutputLine,
        SPayoutStatsTable,
    },
};
use openschafkopf_util::*;
use itertools::*;
use rayon::prelude::*;
use serde::Serialize;
use failure::*;
use derive_new::new;
use plain_enum::PlainEnum;
use super::common_given_game::*;
use as_num::*;
use std::io::IsTerminal;

// TODO? can we make this a fn of SPayoutStatsTable?
fn print_payoutstatstable<T: std::fmt::Display, HigherKinded: TMinMaxStrategiesPublicHigherKinded>(
    payoutstatstable: &SPayoutStatsTable<T, HigherKinded>,
    b_verbose: bool
) {
    let slcoutputline = &payoutstatstable.output_lines();
    if b_verbose { // TODO? only for second-level verbosity
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
                    EMinMaxStrategy::Min => "adversarially",
                    EMinMaxStrategy::SelfishMin => "optimally for themselves, in disfavor of you in case of doubt",
                    EMinMaxStrategy::SelfishMax => "optimally for themselves, in favor of you in case of doubt",
                    EMinMaxStrategy::Max => "optimally for you",
                },
            );
            println!("  * In the worst case (over all generated card distributions), you can enforce a payout of {}", str_payout_min);
            println!("  * On average (over all generated card distributions), you can enforce a payout of {}", str_payout_avg);
            println!("  * In the best case (over all generated card distributions), you can enforce a payout of {}", str_payout_max);
            println!("  * {} shows the number of games lost/zero-payout/won", str_stats);
        }
        println!();
    }
    // TODO interface should probably output payout interval per card
    let mut vecstr_id = Vec::new();
    let mut n_width_id = 0;
    for outputline in slcoutputline.iter() {
        let str_id = outputline.vect.iter().join(" ");
        assign_max(&mut n_width_id, str_id.len());
        vecstr_id.push(str_id);
    }
    for (str_id, SOutputLine{vect:_, perminmaxstrategyatplstrf}) in vecstr_id.iter().zip_eq(slcoutputline.iter()) {
        print!("{str_id:<n_width_id$}: ");
        for ((_emmstrategy_atplstrf, atplstrf), (_emmstrategy_aformatinfo, aformatinfo)) in itertools::zip_eq(
            perminmaxstrategyatplstrf.via_accessors(),
            payoutstatstable.format_infos().via_accessors(),
        ) {
            for ((str_num, f), SFormatInfo{f_min, f_max, n_width}) in atplstrf.iter().zip_eq(aformatinfo.iter()) {
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
                print!("{:>width$}", str_num, width=n_width);
                unwrap!(stdout.reset());
            }
            print!("   ");
        }
        println!();
    }
}

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
        .arg(clap::Arg::new("json")
            .long("json")
            .help("Output result as json")
        )
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    #[derive(new, Serialize)]
    struct SJsonTableLine {
        ostr_header: Option<String>,
        perminmaxstrategyvecpayout_histogram: SPerMinMaxStrategy<Vec<((isize/*n_payout*/, char/*chr_loss_or_win*/), usize/*n_count*/)>>,
    }
    #[derive(new, Serialize)]
    struct SJson {
        str_rules: String,
        astr_hand: [String; EPlayerIndex::SIZE],
        vectableline: Vec<SJsonTableLine>,
    }
    with_common_args(
        clapmatches,
        |itahand, rules, stichseq, ahand_fixed_with_holes, epi_position, expensifiers, b_verbose| {
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
            let fn_human_readable_payout = |stichseq: SStichSequence, (epi_position, hand), n_payout: isize| -> (isize, std::cmp::Ordering) {
                if let Some((_rules, fn_payout_to_points)) = &otplrulesfn_points_as_payout {
                    (
                        fn_payout_to_points(
                            &stichseq,
                            (epi_position, &hand),
                            n_payout.as_num::<f32>(),
                        ).as_num::<isize>(),
                        n_payout.cmp(&0), // Human readable payout may not indicate loss or win: In Rufspiel, if epi_position has 60, it may mean win or loss, depending on whether epi_position is co-player.
                    )
                } else {
                    (n_payout, n_payout.cmp(&0))
                }
            };
            fn json_histograms<HigherKinded: TMinMaxStrategiesPublicHigherKinded>(payoutstatsperstrategy: &HigherKinded::Type<SPayoutStats<std::cmp::Ordering>>)
                -> HigherKinded::Type<Vec<((isize, char), usize)>>
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
            let print_json = |vectableline| {
                if_then_true!(clapmatches.is_present("json"), {
                    println!("{}", unwrap!(serde_json::to_string(
                        &SJson::new(
                            /*str_rules*/rules.to_string(),
                            /*str_hand*/ahand_fixed_with_holes.map(|hand|
                                SDisplayCardSlice::new(hand.cards().clone(), &rules).to_string()
                            ).into_raw(),
                            vectableline,
                        ),
                    )));
                })
            };
            enum EBranching {
                Branching(usize, usize),
                Equivalent(usize, SCardsPartition),
                Oracle,
                OnePerWinnerIndex(Option<EPlayerIndex>),
            }
            use EBranching::*;
            macro_rules! forward_with_args{($forward:ident) => {
                cartesian_match!(
                    forward,
                    match (
                        if let Some(str_branching) = clapmatches.value_of("branching") {
                            if str_branching=="" {
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
                                let (str_lo, str_hi) = str_branching
                                    .split(',')
                                    .collect_tuple()
                                    .ok_or_else(|| format_err!("Could not parse branching"))?;
                                let (n_lo, n_hi) = (str_lo.trim().parse::<usize>()?, str_hi.trim().parse::<usize>()?);
                                Some(Branching(n_lo, n_hi)) // TODO we should avoid branching in case n_lo is greater than all hand's fixed cards
                            }
                        } else {
                            None
                        }
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
                        Some(OnePerWinnerIndex(oepi_unfiltered)) => ((_), |_stichseq, _ahand| {
                            SFilterOnePerWinnerIndex::new(
                                oepi_unfiltered,
                                rules,
                            )
                        }),
                    },
                    match (clapmatches.value_of("prune")) {
                        Some("hint") => (
                            SMinReachablePayoutBase::<
                                SPrunerViaHint,
                                SPerMinMaxStrategyHigherKinded,
                            >
                        ),
                        _ => (
                            SMinReachablePayoutBase::<
                                SPrunerNothing,
                                SPerMinMaxStrategyHigherKinded,
                            >
                        ),
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
            if clapmatches.is_present("no-details") {
                macro_rules! forward{((($($func_filter_allowed_cards_ty: tt)*), $func_filter_allowed_cards: expr), ($foreachsnapshot: ty), ($fn_snapshotcache:expr), $fn_visualizer: expr,) => {{ // TODORUST generic closures
                    itahand
                        .enumerate()
                        .par_bridge() // TODO can we derive a true parallel iterator?
                        .map(|(i_ahand, ahand)| {
                            let mut visualizer = $fn_visualizer(i_ahand, &ahand, /*ocard*/None);
                            explore_snapshots::<_,$($func_filter_allowed_cards_ty)*,_,_,_>(
                                (&mut ahand.clone(), &mut stichseq.clone()),
                                rules,
                                &$func_filter_allowed_cards,
                                &<$foreachsnapshot>::new(
                                    rules,
                                    epi_position,
                                    expensifiers.clone(),
                                ),
                                &$fn_snapshotcache,
                                &mut visualizer,
                            ).map(|mapepiminmax| {
                                SPayoutStats::new_1(fn_human_readable_payout(
                                    stichseq.clone(),
                                    (epi_position, ahand[epi_position].clone(/*TODO needed?*/)),
                                    mapepiminmax[epi_position],
                                ))
                            })
                        })
                        .reduce(
                            /*identity*/|| SPerMinMaxStrategy::new(SPayoutStats::new_identity_for_accumulate()),
                            /*op*/mutate_return!(|perminmaxstrategypayoutstats_lhs, perminmaxstrategypayoutstats_rhs| {
                                perminmaxstrategypayoutstats_lhs.modify_with_other(
                                    &perminmaxstrategypayoutstats_rhs,
                                    SPayoutStats::accumulate,
                                )
                            }),
                        )
                }}}
                let mapemmstrategypaystats = forward_with_args!(forward);
                // TODO this prints a table/json for each iterated rules, but we want to only print one table
                if !print_json(
                    /*vectableline*/vec![SJsonTableLine::new(
                        /*ostr_header*/None, // already given by str_rules
                        /*perminmaxstrategyvecpayout_histogram*/json_histograms::<SPerMinMaxStrategyHigherKinded>(&mapemmstrategypaystats),
                    )],
                ) {
                    print_payoutstatstable::<_,SPerMinMaxStrategyHigherKinded>(
                        &internal_table(
                            vec![(rules, mapemmstrategypaystats)],
                            /*b_group*/false,
                            /*fn_loss_or_win*/&|_n_payout, ord_vs_0| ord_vs_0,
                        ),
                        b_verbose,
                    );
                }
            } else {
                let determinebestcardresult = { // we are interested in payout => single-card-optimization useless
                    macro_rules! forward{((($($func_filter_allowed_cards_ty: tt)*), $func_filter_allowed_cards: expr), ($foreachsnapshot: ty), ($fn_snapshotcache:expr), $fn_visualizer: expr,) => {{ // TODORUST generic closures
                        let n_repeat_hand = clapmatches.value_of("repeat_hands").unwrap_or("1").parse()?;
                        determine_best_card::<$($func_filter_allowed_cards_ty)*,_,_,_,_,_,_,_>( // TODO avoid explicit types
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
                            &<$foreachsnapshot>::new(
                                rules,
                                epi_position,
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
                            epi_position,
                            /*fn_payout*/&|stichseq, ahand, n_payout| fn_human_readable_payout(
                                stichseq.clone(),
                                (epi_position, ahand[epi_position].clone(/*TODO needed?*/)),
                                n_payout,
                            ),
                        )
                    }}}
                    forward_with_args!(forward)
                }.ok_or_else(||format_err!("Could not determine best card. Apparently could not generate valid hands."))?;
                if !print_json(
                    /*vectableline*/determinebestcardresult.cards_and_ts()
                        .map(|(card, payoutstatsperstrategy)|
                            SJsonTableLine::new(
                                /*ostr_header*/Some(card.to_string()),
                                /*perminmaxstrategyvecpayout_histogram*/json_histograms::<SPerMinMaxStrategyHigherKinded>(payoutstatsperstrategy),
                            )
                        )
                        .collect(),
                ) {
                    print_payoutstatstable::<_,SPerMinMaxStrategyHigherKinded>(
                        &table(
                            &determinebestcardresult,
                            rules,
                            /*fn_loss_or_win*/&|_n_payout, ord_vs_0| ord_vs_0,
                        ),
                        b_verbose,
                    )
                }
            }
            Ok(())
        }
    )
}
