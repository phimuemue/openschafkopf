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
            let mut veclinestrings : Vec<(SCard, /*strings*/_, /*numbers*/_)> = Vec::new();
            let mut an_width = [0; N_COLUMNS];
            let mut af_min = [f32::MAX; N_COLUMNS];
            let mut af_max = [f32::MIN; N_COLUMNS];
            for (card, minmax) in veccardminmax {
                let sortable_count = |paystats: &SPayoutStats| {
                    (paystats.counts()[std::cmp::Ordering::Equal]+paystats.counts()[std::cmp::Ordering::Greater])
                        .as_num::<f32>()
                };
                let af : [_; N_COLUMNS] = [
                    minmax.t_min.min().as_num::<f32>(),
                    minmax.t_min.avg(),
                    minmax.t_min.max().as_num::<f32>(),
                    sortable_count(&minmax.t_min),
                    minmax.t_selfish_min.min().as_num::<f32>(),
                    minmax.t_selfish_min.avg(),
                    minmax.t_selfish_min.max().as_num::<f32>(),
                    sortable_count(&minmax.t_selfish_min),
                    minmax.t_selfish_max.min().as_num::<f32>(),
                    minmax.t_selfish_max.avg(),
                    minmax.t_selfish_max.max().as_num::<f32>(),
                    sortable_count(&minmax.t_selfish_max),
                    minmax.t_max.min().as_num::<f32>(),
                    minmax.t_max.avg(),
                    minmax.t_max.max().as_num::<f32>(),
                    sortable_count(&minmax.t_max),
                ];
                let displayable_count =  |paystats: &SPayoutStats| {
                    format!("{} ", paystats.counts().iter().join("/"))
                };
                let astr : [_; N_COLUMNS] = [
                    format!("{} ", af[0]),
                    format!("{:.2} ", af[1]),
                    format!("{} ", af[2]),
                    displayable_count(&minmax.t_min),
                    format!("{} ", af[4]),
                    format!("{:.2} ", af[5]),
                    format!("{} ", af[6]),
                    displayable_count(&minmax.t_selfish_min),
                    format!("{} ", af[8]),
                    format!("{:.2} ", af[9]),
                    format!("{} ", af[10]),
                    displayable_count(&minmax.t_selfish_max),
                    format!("{} ", af[12]),
                    format!("{:.2} ", af[13]),
                    format!("{} ", af[14]),
                    displayable_count(&minmax.t_max),
                ];
                for (n_width, str) in an_width.iter_mut().zip(astr.iter()) {
                    *n_width = (*n_width).max(str.len());
                }
                for (f_min, f_max, f) in izip!(af_min.iter_mut(), af_max.iter_mut(), af.iter()) {
                    // TODO? assign_min/assign_max
                    *f_min = f_min.min(*f);
                    *f_max = f_max.max(*f);
                }
                veclinestrings.push((card, astr, af));
            }
            for (card, astr, af) in veclinestrings.iter() {
                print!("{}: ", card); // all cards have same width
                for (str_num, f, n_width, f_min, f_max) in izip!(astr.iter(), af.iter(), an_width.iter(), af_min.iter(), af_max.iter()) {
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
