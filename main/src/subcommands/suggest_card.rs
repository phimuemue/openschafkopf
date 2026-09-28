use openschafkopf_lib::{
    ai::{*, gametree::*, stichoracle::SFilterByOracle, cardspartition::*},
    rules::{SRules, SDisplayRules, TRules, SRuleStateCacheFixed, SExpensifiers},
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
use derive_new::new;
use plain_enum::{PlainEnum, EnumMap};
use super::{common_given_game::*, hand_stats::*};
use std::io::IsTerminal;
use std::sync::{Arc, Mutex};
use as_num::*;

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
    subcommand_given_game(str_subcommand, "Suggest a card to play given the game so far")
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
    mapcardsetepi_distribution: Arc<Mutex<EnumMap<ECard, EnumSet<EPlayerIndex>>>>,
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
        // TODO output mapcardsetepi_distribution
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
        print_card_distribution_statistics(
            stichseq,
            rules,
            &unwrap!(mapcardsetepi_distribution.lock()), // Cannot finalize_arc_mutex, because still held by iterator
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
            finalize_arc_mutex(inspectionstatistics),
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
            stichseq,
            rules,
        }
    }

    fn update_inspection_statistics(
        &mut self,
        ahand: &EnumMap<EPlayerIndex, SHand>,
    ) {
        for (inspectionhistogram, constraint) in self.vectplinspectionhistogramconstraint.iter_mut() {
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
        |itahand, rules, stichseq, ocard_played, ahand_fixed_with_holes, epi_position, expensifiers, b_verbose, mapcardsetepi_distribution| {
            let otplrulesfn_points_as_payout = if clapmatches.is_present("points") {
                if let Some(tplrulesfn_points_as_payout) = rules.points_as_payout() {
                    Some(tplrulesfn_points_as_payout)
                } else {
                    if b_verbose { // TODO? dispatch statically
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
                        mapcardsetepi_distribution,
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
