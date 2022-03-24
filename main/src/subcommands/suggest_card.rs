use crate::ai::{*, suspicion::*};
use crate::primitives::*;
use crate::util::*;
use crate::rules::*;
use itertools::*;
use crate::game::SStichSequence;

use super::common_given_game::*;

pub fn subcommand(str_subcommand: &'static str) -> clap::App {
    subcommand_given_game(str_subcommand, "Suggest a card to play given the game so far")
        .arg(clap::Arg::with_name("repeat_hands").long("repeat-hands").takes_value(true))
        .arg(clap::Arg::with_name("branching").long("branching").takes_value(true))
        .arg(clap::Arg::with_name("prune").long("prune").takes_value(true))
        .arg(clap::Arg::with_name("visualize").long("visualize").takes_value(true))
        .arg(clap::Arg::with_name("points").long("points")) // TODO? also support by stichs
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    struct SWithCommonArgs<'argmatches> {
        clapmatches: &'argmatches clap::ArgMatches<'argmatches>,
    }
    impl<'argmatches> TWithCommonArgs for SWithCommonArgs<'argmatches> {
        fn call<'rules>(
            self,
            rules_raw: &'rules dyn TRules,
            itahand: Box<dyn Iterator<Item=EnumMap<EPlayerIndex, SHand>>+Send+'rules>,
            eremainingcards: ERemainingCards,
            determinebestcard: SDetermineBestCard,
            b_verbose: bool,
        ) -> Result<(), Error> {
            let clapmatches = self.clapmatches;
            let otplrulesfn_points_as_payout = if clapmatches.is_present("points") {
                if let Some(tplrulesfn_points_as_payout) = rules_raw.points_as_payout() {
                    Some(tplrulesfn_points_as_payout)
                } else {
                    if b_verbose { // TODO? dispatch statically
                        println!("Rules {} do not support point based variant.", rules_raw);
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
                rules_raw
            };
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
                            let make_equivalent = |n_until_remaining_cards| {
                                Equivalent(n_until_remaining_cards, rules.equivalent_when_on_same_hand())
                            };
                            if str_branching=="equiv0" {
                                make_equivalent(0)
                            } else if str_branching=="equiv1" {
                                make_equivalent(1)
                            } else if str_branching=="equiv2" {
                                make_equivalent(2)
                            } else if str_branching=="equiv3" {
                                make_equivalent(3)
                            } else if str_branching=="equiv4" {
                                make_equivalent(4)
                            } else if str_branching=="equiv5" {
                                make_equivalent(5)
                            } else if str_branching=="equiv6" {
                                make_equivalent(6)
                            } else if str_branching=="equiv7" {
                                make_equivalent(7)
                            } else if str_branching=="equiv8" {
                                make_equivalent(8)
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
                        (Some(Equivalent(n_until_remaining_cards, enumchainscard)), _) => (|| {
                            #[derive(Clone, PartialEq, Debug)]
                            struct SSimpleEquivalentCards {
                                enumchainscard: SEnumChains<SCard>,
                                epi_fixed: EPlayerIndex,
                                n_until_remaining_cards: usize,
                            }
                            impl TFilterAllowedCards for SSimpleEquivalentCards {
                                type UnregisterStich = EnumMap<EPlayerIndex, SRemoved<SCard>>;
                                fn register_stich(&mut self, stich: &SStich) -> Self::UnregisterStich {
                                    assert!(stich.is_full());
                                    #[cfg(debug_assertions)] let self_original = self.clone();
                                    // TODO Can we use EPlayerIndex::map_from_fn? (Unsure about evaluation order.)
                                    let mut remove_from_chain = |epi| self.enumchainscard.remove_from_chain(stich[epi]);
                                    let removed_0 = remove_from_chain(EPlayerIndex::EPI0);
                                    let removed_1 = remove_from_chain(EPlayerIndex::EPI1);
                                    let removed_2 = remove_from_chain(EPlayerIndex::EPI2);
                                    let removed_3 = remove_from_chain(EPlayerIndex::EPI3);
                                    let unregisterstich = EPlayerIndex::map_from_raw([removed_0, removed_1, removed_2, removed_3]);
                                    #[cfg(debug_assertions)] {
                                        let mut self_clone = self.clone();
                                        self_clone.unregister_stich(unregisterstich.clone());
                                        assert_eq!(self_original, self_clone);
                                    }
                                    unregisterstich
                                }
                                fn unregister_stich(&mut self, unregisterstich: Self::UnregisterStich) {
                                    for removed in unregisterstich.into_raw().into_iter().rev() {
                                        self.enumchainscard.readd(removed);
                                    }
                                }
                                fn filter_allowed_cards(&self, stichseq: &SStichSequence, veccard: &mut SHandVector) {
                                    // TODO assert that we actually have the correct enumchains
                                    // for (_epi, card) in stichseq.completed_stichs().iter().flat_map(SStich::iter) {
                                    //     enumchainscard.remove_from_chain(*card);
                                    // }
                                    // assert_eq!(enumchainscard, self.enumchainscard);
                                    if remaining_cards_per_hand(stichseq)[self.epi_fixed] >= self.n_until_remaining_cards {
                                        return; // hope that afterwards normal iteration is fast enough
                                    }
                                    // example: First stich was SO GU H8 E7
                                    // then, we have chains EO-GO-HO EU-HU-SU H9-H7, E9-E8, G9-G8-G7, S9-S8-S7
                                    // => If some cards from veccard form a contiguous sequence within enumchainscard, we only need to propagate one of the cards.
                                    let mut veccard_out = Vec::new(); // TODO use SHandVector
                                    for card_allowed in veccard.iter() {
                                        let card_first_in_chain = self.enumchainscard.prev_while(*card_allowed, |card|
                                            veccard.contains(&card)
                                        );
                                        veccard_out.push(card_first_in_chain);
                                    }
                                    veccard_out.sort_unstable_by_key(|card| card.to_usize());
                                    veccard_out.dedup();
                                    if veccard.len()!=veccard_out.len() {
                                        // println!("Found equivalent cards:\n stichseq: {}\n veccard_in : {:?}\n veccard_out: {:?}",
                                        //     stichseq,
                                        //     veccard,
                                        //     veccard_out,
                                        // )
                                    } else {
                                        #[cfg(debug_assertions)] {
                                            veccard.sort_unstable_by_key(|card| card.to_usize());
                                            debug_assert_eq!(veccard as &[SCard], &veccard_out as &[SCard]);
                                        }
                                    }
                                    *veccard = unwrap!((&veccard_out as &[SCard]).try_into());
                                }
                            }
                            SSimpleEquivalentCards {
                                enumchainscard: enumchainscard.clone(),
                                epi_fixed,
                                n_until_remaining_cards,
                            }
                        }),
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
            #[derive(Clone, /*TODO really needed for array construction?*/Copy)]
            struct SFormatInfo {
                f_min: f32,
                f_max: f32,
                n_width: usize,
            }
            let mut aformatinfo = [
                SFormatInfo {
                    f_min: f32::MAX,
                    f_max: f32::MIN,
                    n_width: 0,
                };
                N_COLUMNS
            ];
            for (card, minmax) in veccardminmax {
                let column_counts = |paystats: &SPayoutStats| {(
                    format!("{} ", paystats.counts().iter().join("/")),
                    (paystats.counts()[std::cmp::Ordering::Equal]+paystats.counts()[std::cmp::Ordering::Greater])
                        .as_num::<f32>(),
                )};
                let human_readable_payout = |f_payout| {
                    if let Some(fn_payout_to_points) = &ofn_payout_to_points {
                        fn_payout_to_points(
                            determinebestcard.stichseq,
                            determinebestcard.hand_fixed,
                            f_payout
                        )
                    } else {
                        f_payout
                    }
                };
                let column_min_or_max = |n: isize| {
                    let f_human_readable_payout = human_readable_payout(n.as_num::<f32>());
                    (format!("{} ", f_human_readable_payout), f_human_readable_payout)
                };
                let column_average = |paystats: &SPayoutStats| {
                    let f_human_readable_payout = human_readable_payout(paystats.avg());
                    (format!("{:.2} ", f_human_readable_payout), f_human_readable_payout)
                };
                let atplstrf = [
                    column_min_or_max(minmax.t_min.min()),
                    column_average(&minmax.t_min),
                    column_min_or_max(minmax.t_min.max()),
                    column_counts(&minmax.t_min),
                    column_min_or_max(minmax.t_selfish_min.min()),
                    column_average(&minmax.t_selfish_min),
                    column_min_or_max(minmax.t_selfish_min.max()),
                    column_counts(&minmax.t_selfish_min),
                    column_min_or_max(minmax.t_selfish_max.min()),
                    column_average(&minmax.t_selfish_max),
                    column_min_or_max(minmax.t_selfish_max.max()),
                    column_counts(&minmax.t_selfish_max),
                    column_min_or_max(minmax.t_max.min()),
                    column_average(&minmax.t_max),
                    column_min_or_max(minmax.t_max.max()),
                    column_counts(&minmax.t_max),
                ];
                for ((str_val, f_val), formatinfo) in atplstrf.iter().zip_eq(aformatinfo.iter_mut()) {
                    formatinfo.n_width = formatinfo.n_width.max(str_val.len());
                    // TODO? assign_min/assign_max
                    formatinfo.f_min = formatinfo.f_min.min(*f_val);
                    formatinfo.f_max = formatinfo.f_max.max(*f_val);
                }
                vecoutputline.push(SOutputLine{card, atplstrf});
            }
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
