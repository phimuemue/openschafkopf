use crate::ai::{*, handiterators::*, suspicion::*};
use crate::game::SStichSequence;
use crate::primitives::*;
use crate::util::{*, parser::*};
use crate::rules::*;
use crate::cardvector::*;
use itertools::*;
use combine::{char::*, *};

plain_enum_mod!(moderemainingcards, ERemainingCards {_1, _2, _3, _4, _5, _6, _7, _8,});

#[derive(Clone, Debug, Eq, PartialEq)]
enum VNumVal {
    Const(usize),
    Card(SCard, EPlayerIndex),
    TrumpfOrFarbe(VTrumpfOrFarbe, EPlayerIndex),
    Schlag(ESchlag, EPlayerIndex),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum VConstraint {
    Not(Box<VConstraint>),
    Relation {
        numval_lhs: VNumVal,
        ord: std::cmp::Ordering,
        numval_rhs: VNumVal,
    },
    Conjunction(Box<VConstraint>, Box<VConstraint>),
    Disjunction(Box<VConstraint>, Box<VConstraint>),
}

impl VNumVal {
    fn eval(&self, ahand: &EnumMap<EPlayerIndex, SHand>, rules: &dyn TRules) -> usize {
        fn count(hand: &SHand, fn_pred: impl Fn(&SCard)->bool) -> usize {
            hand.cards().iter().copied().filter(fn_pred).count()
        }
        match self {
            VNumVal::Const(n) => *n,
            VNumVal::Card(card, epi) => count(&ahand[*epi], |card_hand| card_hand==card),
            VNumVal::TrumpfOrFarbe(trumpforfarbe, epi) => count(&ahand[*epi], |card|
                trumpforfarbe==&rules.trumpforfarbe(*card)
            ),
            VNumVal::Schlag(eschlag, epi) => count(&ahand[*epi], |card|
                card.schlag()==*eschlag
            ),
        }
    }
}

impl std::fmt::Display for VNumVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            VNumVal::Const(n) => write!(f, "{}", n),
            VNumVal::Card(card, epi) => write!(f, "{}({})", card, epi),
            VNumVal::TrumpfOrFarbe(trumpforfarbe, epi) => match trumpforfarbe {
                VTrumpfOrFarbe::Trumpf => write!(f, "t({})", epi),
                VTrumpfOrFarbe::Farbe(efarbe) => write!(f, "{}({})", efarbe, epi),
            },
            VNumVal::Schlag(eschlag, epi) => write!(f, "{}({})", eschlag, epi),
        }
    }
}

impl VConstraint {
    fn eval(&self, ahand: &EnumMap<EPlayerIndex, SHand>, rules: &dyn TRules) -> bool {
        match self {
            VConstraint::Not(constraint) => !constraint.eval(ahand, rules),
            VConstraint::Relation{numval_lhs, ord, numval_rhs} => *ord == numval_lhs.eval(ahand, rules).cmp(&numval_rhs.eval(ahand, rules)),
            VConstraint::Conjunction(constraint_lhs, constraint_rhs) => constraint_lhs.eval(ahand, rules) && constraint_rhs.eval(ahand, rules),
            VConstraint::Disjunction(constraint_lhs, constraint_rhs) => constraint_lhs.eval(ahand, rules) || constraint_rhs.eval(ahand, rules),
        }
    }
}

impl std::fmt::Display for VConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            VConstraint::Not(constraint) => write!(f, "!({})", constraint),
            VConstraint::Relation{numval_lhs, ord, numval_rhs} => write!(f, "({}){}({})",
                numval_lhs,
                match ord {
                    std::cmp::Ordering::Less => "<",
                    std::cmp::Ordering::Equal => "=",
                    std::cmp::Ordering::Greater => ">",
                },
                numval_rhs
            ),
            VConstraint::Conjunction(constraint_lhs, constraint_rhs) => write!(f, "({})&({})", constraint_lhs, constraint_rhs),
            VConstraint::Disjunction(constraint_lhs, constraint_rhs) => write!(f, "({})|({})", constraint_lhs, constraint_rhs),
        }
    }
}

fn numval_parser<I: Stream<Item=char>>() -> impl Parser<Input = I, Output = VNumVal>
    where I::Error: ParseError<I::Item, I::Range, I::Position>, // Necessary due to rust-lang/rust#24159
{
    pub fn epi_parser<I: Stream<Item=char>>() -> impl Parser<Input = I, Output = EPlayerIndex>
        where I::Error: ParseError<I::Item, I::Range, I::Position>, // Necessary due to rust-lang/rust#24159
    {
        (spaces(), char('('), spaces())
            .with(choice!(
                char('0').map(|_chr| EPlayerIndex::EPI0),
                char('1').map(|_chr| EPlayerIndex::EPI1),
                char('2').map(|_chr| EPlayerIndex::EPI2),
                char('3').map(|_chr| EPlayerIndex::EPI3)
            ))
            .skip((spaces(), char(')'), spaces()))
    }
    choice!(
        attempt((card_parser(), epi_parser()).map(|(card, epi)| VNumVal::Card(card, epi))),
        (
            choice!(
                choice!(char('t'), char('T')).map(|_| VTrumpfOrFarbe::Trumpf),
                farbe_parser().map(VTrumpfOrFarbe::Farbe)
            ),
            epi_parser()
        ).map(|(trumpforfarbe, epi)| VNumVal::TrumpfOrFarbe(trumpforfarbe, epi)),
        attempt((schlag_parser(), epi_parser()).map(|(eschlag, epi)| VNumVal::Schlag(eschlag, epi))),
        (many1(digit())./*TODO use and_then and get rid of unwrap*/map(|string: /*TODO String needed?*/String|
            unwrap!(string.parse::<usize>())
        )).map(VNumVal::Const)
    )
}

fn single_constraint_parser_<I: Stream<Item=char>>() -> impl Parser<Input = I, Output = VConstraint>
    where I::Error: ParseError<I::Item, I::Range, I::Position>, // Necessary due to rust-lang/rust#24159
{
    choice!(
        (char('!').with(single_constraint_parser())).map(|constraint| VConstraint::Not(Box::new(constraint))),
        char('(').with(constraint_parser()).skip(char(')')),
        (
            numval_parser(),
            optional((
                choice!(
                    char('<').map(|_chr| std::cmp::Ordering::Less),
                    char('=').map(|_chr| std::cmp::Ordering::Equal),
                    char('>').map(|_chr| std::cmp::Ordering::Greater)
                ),
                numval_parser()
            ))
        ).map(|(numval_lhs, otplordnumval_rhs)| {
            let (ord, numval_rhs) = otplordnumval_rhs.unwrap_or((
                std::cmp::Ordering::Greater,
                VNumVal::Const(0)
            ));
            VConstraint::Relation{numval_lhs, ord, numval_rhs}
        })
    )
}
parser!{
    fn single_constraint_parser[I]()(I) -> VConstraint
        where [I: Stream<Item = char>]
    {
        single_constraint_parser_()
    }
}

fn constraint_parser_<I: Stream<Item=char>>() -> impl Parser<Input = I, Output = VConstraint>
    where I::Error: ParseError<I::Item, I::Range, I::Position>, // Necessary due to rust-lang/rust#24159
{
    choice!(
        attempt((sep_by1::<Vec<_>,_,_>(single_constraint_parser(), (spaces(), char('&'), spaces())))
            .map(|vecconstraint| unwrap!(vecconstraint.into_iter().fold1(|constraint_lhs, constraint_rhs|
                VConstraint::Conjunction(Box::new(constraint_lhs), Box::new(constraint_rhs))
            )))),
        attempt((sep_by1::<Vec<_>,_,_>(single_constraint_parser(), (spaces(), char('|'), spaces())))
            .map(|vecconstraint| unwrap!(vecconstraint.into_iter().fold1(|constraint_lhs, constraint_rhs|
                VConstraint::Disjunction(Box::new(constraint_lhs), Box::new(constraint_rhs))
            )))),
        attempt(single_constraint_parser())
    )
}

parser!{
    fn constraint_parser[I]()(I) -> VConstraint
        where [I: Stream<Item = char>]
    {
        constraint_parser_()
    }
}

#[test]
fn test_constraint_parser() {
    fn test_internal(str_in: &str, constraint: VConstraint) {
        assert_eq!(unwrap!(str_in.parse::<VConstraint>()), constraint);
    }
    use VConstraint::*;
    use VNumVal::*;
    use EFarbe::*;
    use ESchlag::*;
    use EPlayerIndex::*;
    use VTrumpfOrFarbe::*;
    use std::cmp::Ordering::*;
    fn test_comparison(str_in: &str, numval_lhs: VNumVal, ord: std::cmp::Ordering, numval_rhs: VNumVal) {
        let relation = Relation{numval_lhs, ord, numval_rhs};
        test_internal(str_in, relation.clone());
        test_internal(&format!("!{}", str_in), Not(Box::new(relation.clone())));
        test_internal(&format!("!!{}", str_in), Not(Box::new(Not(Box::new(relation)))));
    }
    fn test_simple_greater_0(str_in: &str, numval_lhs: VNumVal) {
        test_comparison(str_in, numval_lhs, Greater, Const(0));
    }
    test_simple_greater_0("ea(1)", Card(SCard::new(Eichel, Ass), EPI1));
    test_simple_greater_0("t(2)", TrumpfOrFarbe(Trumpf, EPI2));
    test_simple_greater_0("e(0)", TrumpfOrFarbe(Farbe(Eichel), EPI0));
    test_simple_greater_0("o(0)", Schlag(Ober, EPI0));
    test_simple_greater_0("7(0)", Schlag(S7, EPI0));
    test_simple_greater_0("7", Const(7));
    test_comparison("ea(1)>e(0)", Card(SCard::new(Eichel, Ass), EPI1), Greater, TrumpfOrFarbe(Farbe(Eichel), EPI0));
    test_comparison("t(2)=t(3)", TrumpfOrFarbe(Trumpf, EPI2), Equal, TrumpfOrFarbe(Trumpf, EPI3));
    test_comparison("e(0)>3", TrumpfOrFarbe(Farbe(Eichel), EPI0), Greater, Const(3));
    test_comparison("o(0)<3", Schlag(Ober, EPI0), Less, Const(3));
    test_comparison("8(0)<2", Schlag(S8, EPI0), Less, Const(2));
    test_comparison("8<2", Const(8), Less, Const(2));
    // TODO more tests
}

impl std::str::FromStr for VConstraint {
    type Err = Error;
    fn from_str(str_in: &str) -> Result<Self, Self::Err> {
        parse_trimmed(str_in, "constraint", constraint_parser())
    }
}

pub fn suggest_card(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
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
    let epi_fixed = determinebestcard.epi_fixed;
    let eremainingcards = unwrap!(ERemainingCards::checked_from_usize(remaining_cards_per_hand(&stichseq)[epi_fixed] - 1));
    let determinebestcardresult = { // we are interested in payout => single-card-optimization useless
        macro_rules! forward{(($itahand: expr), ($func_filter_allowed_cards: expr), ($foreachsnapshot: ident),) => {{ // TODORUST generic closures
            determine_best_card(
                &determinebestcard,
                $itahand
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
        enum VChooseItAhand {
            All,
            Sample(usize),
        };
        use VChooseItAhand::*;
        let oiteratehands = if_then_some!(let Some(str_itahand)=clapmatches.value_of("simulate_hands"),
            if "all"==str_itahand.to_lowercase() {
                All
            } else {
                Sample(str_itahand.parse()?)
            }
        );
        use ERemainingCards::*;
        let orelation = if_then_some!(let Some(str_constrain_hands)=clapmatches.value_of("constrain_hands"), {
            let relation = str_constrain_hands.parse::<VConstraint>()?;
            if b_verbose {
                println!("Constraint parsed as: {}", relation);
            }
            relation
        });
        cartesian_match!(
            forward,
            match ((oiteratehands, eremainingcards)) {
                (Some(All), _)|(None, _1)|(None, _2)|(None, _3)|(None, _4) => (
                    all_possible_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)
                        .filter(|ahand| orelation.as_ref().map_or(true, |relation|
                            relation.eval(ahand, rules)
                        ))
                ),
                (Some(Sample(n_samples)), _) => (
                    forever_rand_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)
                        .filter(|ahand| orelation.as_ref().map_or(true, |relation|
                            relation.eval(ahand, rules)
                        ))
                        .take(n_samples)
                ),
                (None, _5)|(None, _6)|(None, _7)|(None, _8) => (
                    forever_rand_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)
                        .filter(|ahand| orelation.as_ref().map_or(true, |relation|
                            relation.eval(ahand, rules)
                        ))
                        .take(/*n_suggest_card_samples*/50)
                ),
            },
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
