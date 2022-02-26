use crate::ai::{*, suspicion::*};
use crate::primitives::*;
use crate::util::*;
use crate::rules::*;
use itertools::*;

use super::common_given_game::*;

pub fn subcommand(str_subcommand: &'static str) -> clap::App {
    subcommand_given_game(str_subcommand, "Suggest a card to play given the game so far")
        .arg(clap::Arg::with_name("repeat_hands").long("repeat-hands").takes_value(true))
        .arg(clap::Arg::with_name("branching").long("branching").takes_value(true))
        .arg(clap::Arg::with_name("prune").long("prune").takes_value(true))
        .arg(clap::Arg::with_name("visualize").long("visualize").takes_value(true))
        // TODO Add possibility to request analysis by points/stichs
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    struct SWithCommonArgs<'argmatches> {
        clapmatches: &'argmatches clap::ArgMatches<'argmatches>,
    }
    impl<'argmatches> TWithCommonArgs for SWithCommonArgs<'argmatches> {
        fn call(
            self,
            rules: &dyn TRules,
            hand_fixed: SHand,
            itahand: impl Iterator<Item=EnumMap<EPlayerIndex, SHand>>+Send,
            eremainingcards: ERemainingCards,
            determinebestcard: SDetermineBestCard,
            b_verbose: bool,
        ) -> Result<(), Error> {
            let clapmatches = self.clapmatches;
            let epi_fixed = determinebestcard.epi_fixed;
            let determinebestcardresult = { // we are interested in payout => single-card-optimization useless
                macro_rules! forward{(($func_filter_allowed_cards: expr), ($foreachsnapshot: ident), $fn_visualizer: expr,) => {{ // TODORUST generic closures
                    let n_repeat_hand = clapmatches.value_of("repeat_hands").unwrap_or("1").parse()?;
                    determine_best_card(
                        &determinebestcard,
                        itahand
                            .inspect(|ahand| {
                                if b_verbose { // TODO? dispatch statically
                                    println!("{}", ahand.iter().join(" | "));
                                }
                            })
                            .flat_map(|ahand| {
                                itertools::repeat_n(
                                    ahand,
                                    n_repeat_hand,
                                )
                            }),
                        $func_filter_allowed_cards,
                        &$foreachsnapshot::new(
                            rules,
                            epi_fixed,
                            /*tpln_stoss_doubling*/(0, 0), // TODO? make customizable
                            /*n_stock*/0, // TODO? make customizable
                        ),
                        $fn_visualizer,
                    )
                }}}
                use ERemainingCards::*;
                cartesian_match!(
                    forward,
                    match ((
                        if_then_some!(let Some(str_tpln_branching) = clapmatches.value_of("branching"), {
                            let (str_lo, str_hi) = str_tpln_branching
                                .split(',')
                                .collect_tuple()
                                .ok_or_else(|| format_err!("Could not parse branching"))?;
                            let (n_lo, n_hi) = (str_lo.trim().parse::<usize>()?, str_hi.trim().parse::<usize>()?);
                            if_then_some!(n_lo < hand_fixed.cards().len(), {
                                if b_verbose {
                                    println!("Branching bounds are large enough to eliminate branching factor.");
                                }
                                (n_lo, n_hi)
                            })
                        }),
                        eremainingcards
                    )) {
                        (Some(None), _)|(None,_1|_2|_3|_4) => (&|_,_| (/*no filtering*/)),
                        (Some(Some((n_lo, n_hi))), _) => (&branching_factor(move |_stichseq| {
                            let n_lo = n_lo.max(1);
                            (n_lo, (n_hi.max(n_lo+1)))
                        })),
                        (None,_5|_6|_7|_8) => (&branching_factor(|_stichseq| (1, 3))),
                    },
                    match ((clapmatches.value_of("prune"), eremainingcards)) {
                        (Some("none"),_)|(_, _1|_2|_3) => (SMinReachablePayout),
                        (Some("hint"),_)|(_, _4|_5|_6|_7|_8) => (SMinReachablePayoutLowerBoundViaHint),
                    },
                    match (clapmatches.value_of("visualize")) {
                        None => (|_,_,_| SNoVisualization),
                        Some(str_path) => {
                            visualizer_factory(
                                std::path::Path::new(str_path).to_path_buf(),
                                rules,
                                epi_fixed,
                            )
                        },
                    },
                )
            };
            // TODO interface should probably output payout interval per card
            let mut veccardminmax = determinebestcardresult.cards_and_ts().collect::<Vec<_>>();
            veccardminmax.sort_unstable_by_key(|&(_card, minmax)| minmax);
            veccardminmax.reverse(); // descending
            // crude formatting: treat all numbers as f32, and convert structured input to a plain number table
            const N_COLUMNS : usize = 16;
            struct SOutputLine {
                card: SCard,
                atplstrf: [(String, f32); N_COLUMNS],
            }
            let mut vecoutputline : Vec<SOutputLine> = Vec::new();
            let mut an_width = [0; N_COLUMNS];
            let mut af_min = [f32::MAX; N_COLUMNS];
            let mut af_max = [f32::MIN; N_COLUMNS];
            for (card, minmax) in veccardminmax {
                let column_counts = |paystats: &SPayoutStats| {(
                    format!("{} ", paystats.counts().iter().join("/")),
                    (paystats.counts()[std::cmp::Ordering::Equal]+paystats.counts()[std::cmp::Ordering::Greater])
                        .as_num::<f32>(),
                )};
                fn column_no_decimals(n: isize) -> (String, f32) {
                    (format!("{} ", n), n.as_num::<f32>())
                }
                fn column_average(paystats: &SPayoutStats) -> (String, f32) {
                    let f_avg = paystats.avg();
                    (format!("{:.2} ", f_avg), f_avg)
                }
                let atplstrf = [
                    column_no_decimals(minmax.t_min.min()),
                    column_average(&minmax.t_min),
                    column_no_decimals(minmax.t_min.max()),
                    column_counts(&minmax.t_min),
                    column_no_decimals(minmax.t_selfish_min.min()),
                    column_average(&minmax.t_selfish_min),
                    column_no_decimals(minmax.t_selfish_min.max()),
                    column_counts(&minmax.t_selfish_min),
                    column_no_decimals(minmax.t_selfish_max.min()),
                    column_average(&minmax.t_selfish_max),
                    column_no_decimals(minmax.t_selfish_max.max()),
                    column_counts(&minmax.t_selfish_max),
                    column_no_decimals(minmax.t_max.min()),
                    column_average(&minmax.t_max),
                    column_no_decimals(minmax.t_max.max()),
                    column_counts(&minmax.t_max),
                ];
                for (n_width, (str_val, _f_val)) in an_width.iter_mut().zip_eq(atplstrf.iter()) {
                    *n_width = (*n_width).max(str_val.len());
                }
                for (f_min, f_max, (_str_val, f_val)) in izip!(af_min.iter_mut(), af_max.iter_mut(), atplstrf.iter()) {
                    // TODO? assign_min/assign_max
                    *f_min = f_min.min(*f_val);
                    *f_max = f_max.max(*f_val);
                }
                vecoutputline.push(SOutputLine{card, atplstrf});
            }
            for SOutputLine{card, atplstrf} in vecoutputline.iter() {
                print!("{}: ", card); // all cards have same width
                for ((str_num, f), n_width, f_min, f_max) in izip!(atplstrf.iter(), an_width.iter(), af_min.iter(), af_max.iter()) {
                    use termcolor::*;
                    let mut stdout = StandardStream::stdout(if atty::is(atty::Stream::Stdout) {
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
                println!();
            }
            Ok(())
        }
    }
    with_common_args(clapmatches, SWithCommonArgs{clapmatches})
}
