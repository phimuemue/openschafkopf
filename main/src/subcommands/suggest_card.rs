use crate::ai::{*, handiterators::*, suspicion::*};
use crate::game::SStichSequence;
use crate::primitives::*;
use crate::util::*;
use crate::rules::*;
use itertools::*;

use super::handconstraint::*;

pub fn subcommand_given_game<'a>(str_subcommand: &'static str, str_about: &'static str) -> clap::App<'static, 'static> {
    let single_arg = |str_name, str_long| {
        clap::Arg::with_name(str_name)
            .long(str_long)
            .required(true)
            .takes_value(true)
    };
    clap::SubCommand::with_name(str_subcommand)
        .about(str_about)
        .arg(single_arg("rules", "rules"))
        .arg(single_arg("hand", "hand"))
        .arg(single_arg("cards_on_table", "cards-on-table"))
        .arg(clap::Arg::with_name("simulate_hands").long("simulate-hands").takes_value(true))
        .arg(clap::Arg::with_name("verbose").long("verbose").short("v"))
        .arg(clap::Arg::with_name("constrain_hands").long("constrain-hands").takes_value(true))
}

pub fn subcommand(str_subcommand: &'static str) -> clap::App {
    subcommand_given_game(str_subcommand, "Suggest a card to play given the game so far")
        .arg(clap::Arg::with_name("branching").long("branching").takes_value(true))
        .arg(clap::Arg::with_name("prune").long("prune").takes_value(true))
}

plain_enum_mod!(moderemainingcards, ERemainingCards {_1, _2, _3, _4, _5, _6, _7, _8,});

enum VChooseItAhand {
    All,
    Sample(usize),
}

pub trait TWithCommonArgs {
    fn call(
        self,
        rules: &dyn TRules,
        hand_fixed: SHand,
        itahand: impl Iterator<Item=EnumMap<EPlayerIndex, SHand>>+Send,
        eremainingcards: ERemainingCards,
        determinebestcard: SDetermineBestCard,
        b_verbose: bool,
    ) -> Result<(), Error>;
}

pub fn with_common_args(
    clapmatches: &clap::ArgMatches,
    withcommanargs: impl TWithCommonArgs
) -> Result<(), Error> {
    let b_verbose = clapmatches.is_present("verbose");
    let hand_fixed = super::str_to_hand(&unwrap!(clapmatches.value_of("hand")))?;
    let veccard_as_played = &cardvector::parse_cards::<Vec<_>>(
        &unwrap!(clapmatches.value_of("cards_on_table")),
    ).ok_or_else(||format_err!("Could not parse played cards"))?;
    // TODO check that everything is ok (no duplicate cards, cards are allowed, current stich not full, etc.)
    let rules = crate::rules::parser::parse_rule_description_simple(&unwrap!(clapmatches.value_of("rules")))?;
    let rules = rules.as_ref();
    let stichseq = SStichSequence::new_from_cards(
        /*ekurzlang*/EKurzLang::checked_from_cards_per_player(
            /*n_stichs_complete*/veccard_as_played.len() / EPlayerIndex::SIZE
                + hand_fixed.cards().len()
        )
            .ok_or_else(|| format_err!("Cannot determine ekurzlang from {} and {:?}.", hand_fixed, veccard_as_played))?,
        veccard_as_played.iter().copied(),
        rules
    );
    let determinebestcard =  SDetermineBestCard::new(
        rules,
        &stichseq,
        &hand_fixed,
    );
    let oconstraint = if_then_some!(let Some(str_constrain_hands)=clapmatches.value_of("constrain_hands"), {
        let relation = str_constrain_hands.parse::<VConstraint>().map_err(|_|format_err!("Cannot parse hand constraints"))?;
        if b_verbose {
            println!("Constraint parsed as: {}", relation);
        }
        relation
    });
    use VChooseItAhand::*;
    use ERemainingCards::*;
    let oiteratehands = if_then_some!(let Some(str_itahand)=clapmatches.value_of("simulate_hands"),
        if "all"==str_itahand.to_lowercase() {
            All
        } else {
            Sample(str_itahand.parse()?)
        }
    );
    let epi_fixed = determinebestcard.epi_fixed;
    let eremainingcards = unwrap!(ERemainingCards::checked_from_usize(remaining_cards_per_hand(&stichseq)[epi_fixed] - 1));
    macro_rules! forward{(($itahand: expr), ) => { // TODORUST generic closures
        withcommanargs.call(
            rules,
            hand_fixed.clone(),
            $itahand,
            eremainingcards,
            determinebestcard,
            b_verbose,
        )
    }};
    cartesian_match!(forward,
        match ((oiteratehands, eremainingcards)) {
            (Some(All), _)|(None, _1)|(None, _2)|(None, _3)|(None, _4) => (
                all_possible_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)
                    .filter(|ahand| oconstraint.as_ref().map_or(true, |relation|
                        relation.eval(ahand, rules)
                    ))
            ),
            (Some(Sample(n_samples)), _) => (
                forever_rand_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)
                    .filter(|ahand| oconstraint.as_ref().map_or(true, |relation|
                        relation.eval(ahand, rules)
                    ))
                    .take(n_samples)
            ),
            (None, _5)|(None, _6)|(None, _7)|(None, _8) => (
                forever_rand_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)
                    .filter(|ahand| oconstraint.as_ref().map_or(true, |relation|
                        relation.eval(ahand, rules)
                    ))
                    .take(/*n_suggest_card_samples*/50)
            ),
        },
    )
}

pub fn run<'argmatches>(clapmatches: &'argmatches clap::ArgMatches) -> Result<(), Error> {
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
                macro_rules! forward{(($func_filter_allowed_cards: expr), ($foreachsnapshot: ident),) => {{ // TODORUST generic closures
                    determine_best_card(
                        &determinebestcard,
                        itahand
                            .inspect(|ahand| {
                                if b_verbose { // TODO? dispatch statically
                                    // TODO make output pretty
                                    for hand in ahand.iter() {
                                        print!("{} | ", hand);
                                    }
                                    println!("");
                                }
                            }),
                        $func_filter_allowed_cards,
                        &$foreachsnapshot::new(
                            rules,
                            epi_fixed,
                            /*tpln_stoss_doubling*/(0, 0), // TODO? make customizable
                            /*n_stock*/0, // TODO? make customizable
                        ),
                        /*opath_out_dir*/None, // TODO? make customizable
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
                        (Some(None), _)|(None,_1)|(None,_2)|(None,_3)|(None,_4) => (&|_,_| (/*no filtering*/)),
                        (Some(Some((n_lo, n_hi))), _) => (&branching_factor(move |_stichseq| {
                            let n_lo = n_lo.max(1);
                            (n_lo, (n_hi.max(n_lo+1)))
                        })),
                        (None,_5)|(None,_6)|(None,_7)|(None,_8) => (&branching_factor(|_stichseq| (1, 3))),
                    },
                    match ((clapmatches.value_of("prune"), eremainingcards)) {
                        (Some("none"),_)|(_, _1)|(_, _2)|(_, _3) => (SMinReachablePayout),
                        (Some("hint"),_)|(_, _4)|(_, _5)|(_, _6)|(_, _7)|(_, _8) => (SMinReachablePayoutLowerBoundViaHint),
                    },
                )
            };
            // TODO interface should probably output payout interval per card
            let mut veccardminmax = determinebestcardresult.cards_and_ts().collect::<Vec<_>>();
            veccardminmax.sort_unstable_by_key(|&(_card, minmax)| minmax);
            veccardminmax.reverse(); // descending
            // crude formatting: treat all numbers as f32, and convert structured input to a plain number table
            const N_COLUMNS : usize = EMinMaxStrategy::SIZE*3;
            let mut vecaf = Vec::new();
            let mut veclinestrings : Vec<(/*card*/String, /*numbers*/[String; N_COLUMNS])> = Vec::new();
            let mut an_width = [0; N_COLUMNS];
            let mut af_min = [f32::MAX; N_COLUMNS];
            let mut af_max = [f32::MIN; N_COLUMNS];
            for (card, minmax) in veccardminmax {
                let af = [
                    minmax.0[EMinMaxStrategy::OthersMin].min().as_num::<f32>(),
                    minmax.0[EMinMaxStrategy::OthersMin].avg(),
                    minmax.0[EMinMaxStrategy::OthersMin].max().as_num::<f32>(),
                    minmax.0[EMinMaxStrategy::MaxPerEpi].min().as_num::<f32>(),
                    minmax.0[EMinMaxStrategy::MaxPerEpi].avg(),
                    minmax.0[EMinMaxStrategy::MaxPerEpi].max().as_num::<f32>(),
                ];
                let astr = [
                    format!("{} ", af[0]),
                    format!("{:.2} ", af[1]),
                    format!("{} ", af[2]),
                    format!("{} ", af[3]),
                    format!("{:.2} ", af[4]),
                    format!("{}", af[5]),
                ];
                for (n_width, str) in an_width.iter_mut().zip(astr.iter()) {
                    *n_width = (*n_width).max(str.len());
                }
                for (f_min, f_max, f) in izip!(af_min.iter_mut(), af_max.iter_mut(), af.iter()) {
                    // TODO? assign_min/assign_max
                    *f_min = f_min.min(*f);
                    *f_max = f_max.max(*f);
                }
                veclinestrings.push((format!("{}", card), astr));
                vecaf.push(af);
            }
            for ((card, astr), af) in veclinestrings.iter().zip(vecaf) {
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
