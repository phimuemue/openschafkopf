use crate::ai::{*, suspicion::*};
use crate::primitives::*;
use crate::util::*;
use itertools::*;
use crate::game::SStichSequence;
use crate::game_analysis::determine_best_card_table::{table, SOutputLine, SFormatInfo};

use super::common_given_game::*;

pub fn subcommand(str_subcommand: &'static str) -> clap::Command {
    subcommand_given_game(str_subcommand, "Suggest a card to play given the game so far")
        .arg(clap::Arg::new("repeat_hands").long("repeat-hands").takes_value(true))
        .arg(clap::Arg::new("branching").long("branching").takes_value(true))
        .arg(clap::Arg::new("prune").long("prune").takes_value(true))
        .arg(clap::Arg::new("visualize").long("visualize").takes_value(true))
        .arg(clap::Arg::new("points").long("points")) // TODO? also support by stichs
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    struct SWithCommonArgs<'argmatches> {
        clapmatches: &'argmatches clap::ArgMatches,
    }
    impl<'argmatches> TWithCommonArgs for SWithCommonArgs<'argmatches> {
        fn call<'rules>(
            self,
            itahand: Box<dyn Iterator<Item=EnumMap<EPlayerIndex, SHand>>+Send+'rules>,
            eremainingcards: ERemainingCards,
            determinebestcard: SDetermineBestCard,
            b_verbose: bool,
        ) -> Result<(), Error> {
            let clapmatches = self.clapmatches;
            let otplrulesfn_points_as_payout = if clapmatches.is_present("points") {
                if let Some(tplrulesfn_points_as_payout) = determinebestcard.rules.points_as_payout() {
                    Some(tplrulesfn_points_as_payout)
                } else {
                    if b_verbose { // TODO? dispatch statically
                        println!("Rules {} do not support point based variant.", determinebestcard.rules);
                    }
                    None
                }
            } else {
                None
            };
            let mut ofn_payout_to_points = None;
            let rules = if let Some((rules, fn_payout_to_points)) = &otplrulesfn_points_as_payout {
                ofn_payout_to_points = Some(fn_payout_to_points);
                rules.as_ref()
            } else {
                determinebestcard.rules
            };
            let epi_fixed = determinebestcard.epi_fixed;
            let determinebestcardresult = { // we are interested in payout => single-card-optimization useless
                macro_rules! forward{(($func_filter_allowed_cards: expr), ($foreachsnapshot: ident), $fn_visualizer: expr,) => {{ // TODORUST generic closures
                    let n_repeat_hand = clapmatches.value_of("repeat_hands").unwrap_or("1").parse()?;
                    determine_best_card(
                        &determinebestcard,
                        itahand
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
                enum EBranching {
                    NoFilter,
                    Branching(usize, usize),
                    Equivalent(usize, SEnumChains<SCard>),
                }
                use ERemainingCards::*;
                use EBranching::*;
                cartesian_match!(
                    forward,
                    match ((
                        if_then_some!(let Some(str_branching) = clapmatches.value_of("branching"), {
                            if let Some(n_until_remaining_cards) = str_branching.strip_prefix("equiv")
                                .and_then(|str_n_until_remaining_cards| str_n_until_remaining_cards.parse().ok())
                            {
                                Equivalent(n_until_remaining_cards, rules.equivalent_when_on_same_hand())
                            } else {
                                let (str_lo, str_hi) = str_branching
                                    .split(',')
                                    .collect_tuple()
                                    .ok_or_else(|| format_err!("Could not parse branching"))?;
                                let (n_lo, n_hi) = (str_lo.trim().parse::<usize>()?, str_hi.trim().parse::<usize>()?);
                                if n_lo < determinebestcard.hand_fixed.cards().len() {
                                    Branching(n_lo, n_hi)
                                } else {
                                    if b_verbose {
                                        println!("Branching bounds are large enough to eliminate branching factor.");
                                    }
                                    NoFilter
                                }
                            }
                        }),
                        eremainingcards
                    )) {
                        (Some(NoFilter), _)|(None,_1|_2|_3|_4) => (|| |_: &SStichSequence, _: &mut SHandVector| (/*no filtering*/)),
                        (Some(Branching(n_lo, n_hi)), _) => (|| branching_factor(move |_stichseq| {
                            let n_lo = n_lo.max(1);
                            (n_lo, (n_hi.max(n_lo+1)))
                        })),
                        (Some(Equivalent(n_until_remaining_cards, enumchainscard)), _) => (
                            equivalent_cards_filter(
                                n_until_remaining_cards,
                                epi_fixed,
                                enumchainscard,
                            )
                        ),
                        (None,_5|_6|_7|_8) => (|| branching_factor(|_stichseq| (1, 3))),
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
            let (vecoutputline, aformatinfo) = table(
                &determinebestcardresult,
                /*fn_human_readable_payout*/&|f_payout| {
                    if let Some(fn_payout_to_points) = &ofn_payout_to_points {
                        fn_payout_to_points(
                            determinebestcard.stichseq,
                            determinebestcard.hand_fixed,
                            f_payout
                        )
                    } else {
                        f_payout
                    }
                },
            );
            // TODO interface should probably output payout interval per card
            for SOutputLine{card, atplstrf} in vecoutputline.iter() {
                print!("{}: ", card); // all cards have same width
                for ((str_num, f), SFormatInfo{f_min, f_max, n_width}) in atplstrf.iter().zip_eq(aformatinfo.iter()) {
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
