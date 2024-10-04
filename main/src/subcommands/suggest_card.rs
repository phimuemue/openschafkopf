use openschafkopf_lib::{
    ai::{*, gametree::*, stichoracle::SFilterByOracle, cardspartition::*},
    rules::{SRules, TRules, SRuleStateCacheFixed},
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
use serde::Serialize;
use failure::*;
use derive_new::new;
use plain_enum::{EnumMap, PlainEnum};
use super::common_given_game::*;
use as_num::*;
use std::io::IsTerminal;

// TODO? can we make this a fn of SPayoutStatsTable?
fn print_payoutstatstable<T: std::fmt::Display, MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded>(
    payoutstatstable: &SPayoutStatsTable<T, MinMaxStrategiesHK>,
    b_print_table_description_before_table: bool
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
                    EMinMaxStrategy::Min => "adversarially",
                    EMinMaxStrategy::SelfishMin => "optimally for themselves, in disfavor of you in case of doubt",
                    EMinMaxStrategy::SelfishMax => "optimally for themselves, in favor of you in case of doubt",
                    EMinMaxStrategy::Max => "optimally for you",
                },
            );
            println!("  * In the worst case (over all generated card distributions), you can enforce a payout of {}", str_payout_min);
            println!("  * On average (over all generated card distributions), you can enforce a payout of {}", str_payout_avg);
            println!("  * In the best case (over all generated card distributions), you can enforce a payout of {}", str_payout_max);
            println!("  * {} shows the number of games lost[/zero-payout]/won", str_stats);
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
        .arg(clap::Arg::new("json")
            .long("json")
            .help("Output result as json")
        )
}

#[derive(new, Serialize)]
struct SJsonTableLine<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded>
    where
        MinMaxStrategiesHK::Type<Vec<((isize/*n_payout*/, char/*chr_loss_or_win*/), usize/*n_count*/)>>: Serialize,
{
    ostr_header: Option<String>,
    perminmaxstrategyvecpayout_histogram: MinMaxStrategiesHK::Type<Vec<((isize/*n_payout*/, char/*chr_loss_or_win*/), usize/*n_count*/)>>,
}

#[derive(new, Serialize)]
struct SJson<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded>
    where
        MinMaxStrategiesHK::Type<Vec<((isize/*n_payout*/, char/*chr_loss_or_win*/), usize/*n_count*/)>>: Serialize,
{
    str_rules: String,
    astr_hand: [String; EPlayerIndex::SIZE],
    vectableline: Vec<SJsonTableLine<MinMaxStrategiesHK>>,
}

fn json_histograms<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded>(payoutstatsperstrategy: &MinMaxStrategiesHK::Type<SPayoutStats<std::cmp::Ordering>>)
    -> MinMaxStrategiesHK::Type<Vec<((isize, char), usize)>>
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

fn make_snapshot_cache<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded>(rules: &SRules) -> impl Fn(&SRuleStateCacheFixed) -> Box<dyn TSnapshotCache<MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>>> + '_
    where
        MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>: PartialEq+std::fmt::Debug+Clone,
{
    move |rulestatecache| rules.snapshot_cache::<MinMaxStrategiesHK>(rulestatecache)
}

#[allow(clippy::extra_unused_type_parameters)]
fn make_snapshot_cache_none<MinMaxStrategiesHK>(_rules: &SRules) -> impl Fn(&SRuleStateCacheFixed)->SSnapshotCacheNone {
    SSnapshotCacheNone::factory()
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
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
                rules.clone()
            } else {
                rules.clone()
            };
            let rules = &rules;
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
            use EBranching::*;
            let oesinglestrategy = match clapmatches.value_of("strategy") {
                Some("maxmin") => Ok(Some(ESingleStrategy::MaxMin)),
                Some("maxselfishmin") => Ok(Some(ESingleStrategy::MaxSelfishMin)),
                None => Ok(None),
                Some(_) => Err(format_err!("Could not understand strategy.")),
            }?;
            // we are interested in payout => single-card-optimization useless
            macro_rules! forward{((($($func_filter_allowed_cards_ty: tt)*), $func_filter_allowed_cards: expr), ($pruner:ident), ($MinMaxStrategiesHK:ident, $fn_alphabetapruner:expr,), $fn_snapshotcache:ident, $fn_visualizer: expr,) => {{ // TODORUST generic closures
                let n_repeat_hand = clapmatches.value_of("repeat_hands").unwrap_or("1").parse()?;
                let determinebestcardresult = determine_best_card::<$($func_filter_allowed_cards_ty)*,_,_,_,_,_,_,_,_>( // TODO avoid explicit types
                    stichseq,
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
                    &|stichseq, ahand| <SMinReachablePayoutBase<$pruner, $MinMaxStrategiesHK, _>>::new_with_pruner(
                        rules,
                        epi_position,
                        expensifiers.clone(),
                        #[allow(clippy::redundant_closure_call)]
                        $fn_alphabetapruner(stichseq, ahand),
                    ),
                    $fn_snapshotcache::<$MinMaxStrategiesHK>(rules),
                    $fn_visualizer,
                    /*fn_inspect*/&|b_before, i_ahand, ahand, card| {
                        if b_verbose {
                            println!(" {} {} ({}): {}",
                                if b_before {'>'} else {'<'},
                                i_ahand+1, // TODO use same hand counters as in common_given_game
                                card,
                                display_card_slices(&ahand, rules, " | "),
                            );
                        }
                    },
                    /*fn_payout*/&|stichseq, ahand, n_payout| fn_human_readable_payout(
                        stichseq.clone(),
                        (epi_position, ahand[epi_position].clone(/*TODO needed?*/)),
                        n_payout,
                    ),
                ).ok_or_else(||format_err!("Could not determine best card. Apparently could not generate valid hands."))?;
                if clapmatches.is_present("json") {
                    println!("{}", unwrap!(serde_json::to_string(
                        &SJson::new(
                            /*str_rules*/rules.to_string(),
                            /*str_hand*/ahand_fixed_with_holes.map(|hand|
                                SDisplayCardSlice::new(hand.cards().clone(), rules).to_string()
                            ).into_raw(),
                            /*vectableline*/itertools::chain(
                                determinebestcardresult.cards_and_ts()
                                    .map(|(card, payoutstatsperstrategy)|
                                        SJsonTableLine::new(
                                            /*ostr_header*/Some(card.to_string()),
                                            /*perminmaxstrategyvecpayout_histogram*/json_histograms::<$MinMaxStrategiesHK>(payoutstatsperstrategy),
                                        )
                                    ),
                                std::iter::once(SJsonTableLine::new(
                                    /*ostr_header*/Some("no-details".to_string()),
                                    /*perminmaxstrategyvecpayout_histogram*/json_histograms::<$MinMaxStrategiesHK>(&determinebestcardresult.t_combined),
                                )),
                            ).collect::<Vec<SJsonTableLine<$MinMaxStrategiesHK>>>(),
                        ),
                    )));
                } else {
                    let payoutstatstable = table(
                        &determinebestcardresult,
                        rules,
                        /*fn_loss_or_win*/&|_n_payout, ord_vs_0| ord_vs_0,
                    );
                    print_payoutstatstable::<_,$MinMaxStrategiesHK>(
                        &payoutstatstable,
                        /*b_print_table_description_before_table*/b_verbose,
                    );
                    println!("-----");
                    print_payoutstatstable::<_,$MinMaxStrategiesHK>(
                        &internal_table(
                            vec!(("no-details", determinebestcardresult.t_combined)),
                            /*b_group*/false,
                            /*fn_loss_or_win*/&|_n_payout, ord_vs_0| ord_vs_0,
                        ),
                        /*b_print_table_description_before_table*/false,
                    );
                }
            }}}
            cartesian_match!(
                forward,
                match (
                    if let Some(str_branching) = clapmatches.value_of("branching") {
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
                    // Some("hint") => (SPrunerViaHint), // TODO re-enable
                    _ => (SPrunerNothing),
                },
                match ((oesinglestrategy, clapmatches.is_present("abprune"), rules.alpha_beta_pruner_lohi_values())) {
                    (None, b_abprune, _) => (
                        SPerMinMaxStrategyHigherKinded,
                        {
                            if b_abprune && b_verbose {
                                println!("Warning: abprune not supported strategy/rules combination. Continuing without.");
                            }
                            |_stichseq, _ahand| SAlphaBetaPrunerNone
                        },
                    ),
                    (Some(ESingleStrategy::MaxMin), false, _) => (
                        SMaxMinStrategyHigherKinded,
                        |_stichseq, _ahand| SAlphaBetaPrunerNone,
                    ),
                    (Some(ESingleStrategy::MaxMin), true, _) => (
                        SMaxMinStrategyHigherKinded,
                        (|_stichseq, _ahand| SAlphaBetaPruner::new({
                            let mut mapepilohi = EPlayerIndex::map_from_fn(|_| ELoHi::Lo);
                            mapepilohi[epi_position] = ELoHi::Hi;
                            mapepilohi
                        })),
                    ),
                    (Some(ESingleStrategy::MaxSelfishMin), b_abprune@false, _) | (Some(ESingleStrategy::MaxSelfishMin), b_abprune@true, None) => (
                        SMaxSelfishMinStrategyHigherKinded,
                        {
                            if b_abprune && b_verbose {
                                println!("Warning: abprune not supported strategy/rules combination. Continuing without.");
                            }
                            |_stichseq, _ahand| SAlphaBetaPrunerNone
                        },
                    ),
                    (Some(ESingleStrategy::MaxSelfishMin), true, Some(fn_alpha_beta_pruner_lohi_values)) => (
                        SMaxSelfishMinStrategyHigherKinded,
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
            Ok(())
        }
    )
}
